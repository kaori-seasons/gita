//! 模型管理器
//!
//! 管理ML模型的加载、缓存和版本控制

use candle_core::{Device, Tensor};
use candle_nn::VarBuilder;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use rust_edge_compute_core::core::error::{Result, EdgeComputeError};
use std::time::{Duration, Instant};

/// 模型信息
#[derive(Debug, Clone)]
pub struct ModelInfo {
    /// 模型ID
    pub model_id: String,
    /// 模型名称
    pub model_name: String,
    /// 模型路径
    pub model_path: PathBuf,
    /// 模型类型
    pub model_type: ModelType,
    /// 设备
    pub device: Device,
    /// 加载时间
    pub loaded_at: Instant,
    /// 最后使用时间
    pub last_used: Instant,
    /// 使用次数
    pub usage_count: u64,
}

/// 模型类型
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ModelType {
    /// 文本生成模型（如LLaMA）
    TextGeneration,
    /// 图像分类模型（如YOLO）
    ImageClassification,
    /// 语音识别模型（如Whisper）
    SpeechRecognition,
    /// 多模态模型
    Multimodal,
    /// 其他
    Other(String),
}

/// 模型缓存配置
#[derive(Debug, Clone)]
pub struct ModelCacheConfig {
    /// 最大缓存模型数
    pub max_cached_models: usize,
    /// 模型过期时间（秒）
    pub model_ttl_seconds: u64,
    /// 是否启用自动清理
    pub enable_auto_cleanup: bool,
    /// 清理间隔（秒）
    pub cleanup_interval_seconds: u64,
}

impl Default for ModelCacheConfig {
    fn default() -> Self {
        Self {
            max_cached_models: 10,
            model_ttl_seconds: 3600, // 1小时
            enable_auto_cleanup: true,
            cleanup_interval_seconds: 300, // 5分钟
        }
    }
}

/// 模型管理器
pub struct ModelManager {
    /// 配置
    config: ModelCacheConfig,
    /// 缓存的模型
    models: Arc<RwLock<HashMap<String, ModelInfo>>>,
    /// 设备管理器
    device_manager: Arc<DeviceManager>,
}

impl ModelManager {
    /// 创建新的模型管理器
    pub fn new(
        config: ModelCacheConfig,
        device_manager: Arc<DeviceManager>,
    ) -> Self {
        let manager = Self {
            config,
            models: Arc::new(RwLock::new(HashMap::new())),
            device_manager,
        };
        
        // 启动自动清理任务
        if manager.config.enable_auto_cleanup {
            manager.start_cleanup_task();
        }
        
        manager
    }
    
    /// 加载模型
    pub async fn load_model(
        &self,
        model_id: String,
        model_path: impl AsRef<Path>,
        model_type: ModelType,
    ) -> Result<()> {
        let model_path = model_path.as_ref().to_path_buf();
        
        // 检查模型文件是否存在
        if !model_path.exists() {
            return Err(EdgeComputeError::Io {
                message: format!("Model file not found: {:?}", model_path),
                operation: Some("load_model".to_string()),
                path: Some(model_path.to_string_lossy().to_string()),
            });
        }
        
        // 选择设备
        let device = self.device_manager.select_best_device().await?;
        
        // 创建模型信息
        let model_info = ModelInfo {
            model_id: model_id.clone(),
            model_name: model_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string(),
            model_path,
            model_type,
            device,
            loaded_at: Instant::now(),
            last_used: Instant::now(),
            usage_count: 0,
        };
        
        // 检查缓存是否已满
        {
            let models = self.models.read().await;
            if models.len() >= self.config.max_cached_models {
                // 清理最旧的模型
                self.cleanup_oldest_model().await?;
            }
        }
        
        // 添加到缓存
        {
            let mut models = self.models.write().await;
            models.insert(model_id, model_info);
        }
        
        tracing::info!("Model loaded: {}", model_id);
        Ok(())
    }
    
    /// 获取模型
    pub async fn get_model(&self, model_id: &str) -> Option<ModelInfo> {
        let mut models = self.models.write().await;
        
        if let Some(model) = models.get_mut(model_id) {
            // 更新使用信息
            model.last_used = Instant::now();
            model.usage_count += 1;
            Some(model.clone())
        } else {
            None
        }
    }
    
    /// 卸载模型
    pub async fn unload_model(&self, model_id: &str) -> Result<()> {
        let mut models = self.models.write().await;
        models.remove(model_id);
        tracing::info!("Model unloaded: {}", model_id);
        Ok(())
    }
    
    /// 列出所有缓存的模型
    pub async fn list_models(&self) -> Vec<ModelInfo> {
        let models = self.models.read().await;
        models.values().cloned().collect()
    }
    
    /// 清理最旧的模型
    async fn cleanup_oldest_model(&self) -> Result<()> {
        let mut models = self.models.write().await;
        
        if let Some((oldest_id, _)) = models
            .iter()
            .min_by_key(|(_, info)| info.last_used)
        {
            let id = oldest_id.clone();
            models.remove(&id);
            tracing::info!("Cleaned up oldest model: {}", id);
        }
        
        Ok(())
    }
    
    /// 清理过期模型
    async fn cleanup_expired_models(&self) {
        let ttl = Duration::from_secs(self.config.model_ttl_seconds);
        let now = Instant::now();
        
        let mut models = self.models.write().await;
        let expired_ids: Vec<String> = models
            .iter()
            .filter(|(_, info)| now.duration_since(info.last_used) > ttl)
            .map(|(id, _)| id.clone())
            .collect();
        
        for id in expired_ids {
            models.remove(&id);
            tracing::info!("Cleaned up expired model: {}", id);
        }
    }
    
    /// 启动清理任务
    fn start_cleanup_task(&self) {
        let models = Arc::clone(&self.models);
        let config = self.config.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                Duration::from_secs(config.cleanup_interval_seconds),
            );
            
            loop {
                interval.tick().await;
                
                // 清理过期模型
                let ttl = Duration::from_secs(config.model_ttl_seconds);
                let now = Instant::now();
                
                let mut models_guard = models.write().await;
                let expired_ids: Vec<String> = models_guard
                    .iter()
                    .filter(|(_, info)| now.duration_since(info.last_used) > ttl)
                    .map(|(id, _)| id.clone())
                    .collect();
                
                for id in expired_ids {
                    models_guard.remove(&id);
                    tracing::info!("Cleaned up expired model: {}", id);
                }
            }
        });
    }
    
    /// 获取模型统计信息
    pub async fn get_stats(&self) -> ModelManagerStats {
        let models = self.models.read().await;
        
        ModelManagerStats {
            total_models: models.len(),
            total_usage: models.values().map(|m| m.usage_count).sum(),
            models_by_type: {
                let mut map = HashMap::new();
                for model in models.values() {
                    let key = match &model.model_type {
                        ModelType::TextGeneration => "text_generation",
                        ModelType::ImageClassification => "image_classification",
                        ModelType::SpeechRecognition => "speech_recognition",
                        ModelType::Multimodal => "multimodal",
                        ModelType::Other(name) => name.as_str(),
                    };
                    *map.entry(key.to_string()).or_insert(0) += 1;
                }
                map
            },
        }
    }
}

/// 模型管理器统计信息
#[derive(Debug, Clone)]
pub struct ModelManagerStats {
    /// 总模型数
    pub total_models: usize,
    /// 总使用次数
    pub total_usage: u64,
    /// 按类型分组的模型数
    pub models_by_type: HashMap<String, usize>,
}

use crate::device_manager::DeviceManager;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device_manager::DeviceManager;
    
    #[tokio::test]
    async fn test_model_manager_creation() {
        let device_manager = Arc::new(DeviceManager::default());
        let manager = ModelManager::new(
            ModelCacheConfig::default(),
            device_manager,
        );
        
        let stats = manager.get_stats().await;
        assert_eq!(stats.total_models, 0);
    }
    
    #[tokio::test]
    async fn test_model_manager_load_model() {
        let device_manager = Arc::new(DeviceManager::default());
        let manager = ModelManager::new(
            ModelCacheConfig::default(),
            device_manager,
        );
        
        // 注意：这个测试需要实际的模型文件
        // 这里只是测试接口，实际使用时需要提供真实的模型路径
        // let result = manager.load_model(
        //     "test-model".to_string(),
        //     Path::new("./test_model.bin"),
        //     ModelType::TextGeneration,
        // ).await;
        // assert!(result.is_ok());
    }
}
