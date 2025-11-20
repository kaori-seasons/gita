//! Python模型加载器
//!
//! 提供Python模型的加载、缓存和管理功能

use rust_edge_compute_core::core::error::{Result, EdgeComputeError};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{Duration, SystemTime};
use serde::{Deserialize, Serialize};

/// 模型加载器配置
#[derive(Debug, Clone)]
pub struct ModelLoaderConfig {
    /// 模型目录
    pub model_dir: PathBuf,
    /// 是否自动扫描
    pub auto_scan: bool,
    /// 扫描间隔（秒）
    pub scan_interval_seconds: u64,
    /// 最大模型缓存大小（MB）
    pub max_cache_size_mb: u64,
}

impl Default for ModelLoaderConfig {
    fn default() -> Self {
        Self {
            model_dir: PathBuf::from("./python_models"),
            auto_scan: true,
            scan_interval_seconds: 60,
            max_cache_size_mb: 1024,
        }
    }
}

/// 模型信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    /// 模型名称
    pub name: String,
    /// 模型路径
    pub path: PathBuf,
    /// 模型类型（如：pytorch, tensorflow, onnx等）
    pub model_type: String,
    /// 文件大小（字节）
    pub file_size: u64,
    /// 加载时间
    pub loaded_at: Option<SystemTime>,
    /// 使用次数
    pub usage_count: u64,
    /// 最后使用时间
    pub last_used: Option<SystemTime>,
    /// 元数据
    pub metadata: HashMap<String, String>,
}

/// 模型加载器
pub struct ModelLoader {
    /// 配置
    config: ModelLoaderConfig,
    /// 已加载的模型缓存
    loaded_models: Arc<RwLock<HashMap<String, ModelInfo>>>,
    /// 加载统计
    load_stats: Arc<RwLock<LoadStats>>,
    /// 扫描任务句柄
    scan_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
}

/// 加载统计
#[derive(Debug, Clone, Default)]
pub struct LoadStats {
    /// 总加载次数
    pub total_loads: u64,
    /// 成功加载次数
    pub successful_loads: u64,
    /// 失败加载次数
    pub failed_loads: u64,
    /// 缓存命中次数
    pub cache_hits: u64,
    /// 总扫描次数
    pub total_scans: u64,
}

impl ModelLoader {
    /// 创建新的模型加载器
    pub fn new(config: ModelLoaderConfig) -> Result<Self> {
        // 创建模型目录
        if let Some(parent) = config.model_dir.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| EdgeComputeError::Config {
                    message: format!("Failed to create model directory: {}", e),
                    source: Some("model-loader".to_string()),
                })?;
        }
        
        let loader = Self {
            config: config.clone(),
            loaded_models: Arc::new(RwLock::new(HashMap::new())),
            load_stats: Arc::new(RwLock::new(LoadStats::default())),
            scan_handle: Arc::new(RwLock::new(None)),
        };
        
        // 如果启用自动扫描，启动扫描任务
        if config.auto_scan {
            loader.start_scan_task();
        }
        
        Ok(loader)
    }
    
    /// 启动自动扫描任务
    fn start_scan_task(&self) {
        let model_dir = self.config.model_dir.clone();
        let loaded_models = Arc::clone(&self.loaded_models);
        let load_stats = Arc::clone(&self.load_stats);
        let interval = Duration::from_secs(self.config.scan_interval_seconds);
        
        let handle = tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval);
            
            loop {
                interval_timer.tick().await;
                
                // 扫描模型目录
                if let Err(e) = Self::scan_model_directory(&model_dir, &loaded_models, &load_stats).await {
                    tracing::error!("Error scanning model directory: {}", e);
                }
            }
        });
        
        let mut scan_handle = self.scan_handle.write().unwrap();
        *scan_handle = Some(handle);
    }
    
    /// 扫描模型目录
    async fn scan_model_directory(
        model_dir: &Path,
        loaded_models: &Arc<RwLock<HashMap<String, ModelInfo>>>,
        load_stats: &Arc<RwLock<LoadStats>>,
    ) -> Result<()> {
        if !model_dir.exists() {
            return Ok(());
        }
        
        tracing::debug!("Scanning model directory: {:?}", model_dir);
        
        // 更新统计
        {
            let mut stats = load_stats.write().await;
            stats.total_scans += 1;
        }
        
        // 读取目录
        let entries = std::fs::read_dir(model_dir)
            .map_err(|e| EdgeComputeError::Config {
                message: format!("Failed to read model directory: {}", e),
                source: Some("model-loader".to_string()),
            })?;
        
        let mut found_models = HashMap::new();
        
        for entry in entries {
            let entry = entry.map_err(|e| EdgeComputeError::Config {
                message: format!("Failed to read directory entry: {}", e),
                source: Some("model-loader".to_string()),
            })?;
            
            let path = entry.path();
            
            // 检查是否是模型文件
            if Self::is_model_file(&path) {
                let model_name = path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string();
                
                let file_size = path.metadata()
                    .map(|m| m.len())
                    .unwrap_or(0);
                
                let model_type = Self::detect_model_type(&path);
                
                let model_info = ModelInfo {
                    name: model_name.clone(),
                    path: path.clone(),
                    model_type,
                    file_size,
                    loaded_at: None,
                    usage_count: 0,
                    last_used: None,
                    metadata: HashMap::new(),
                };
                
                found_models.insert(model_name, model_info);
            }
        }
        
        // 更新已加载的模型缓存
        let mut models = loaded_models.write().await;
        for (name, info) in found_models {
            // 如果模型已存在，保留加载时间和使用统计
            if let Some(existing) = models.get(&name) {
                models.insert(name.clone(), ModelInfo {
                    loaded_at: existing.loaded_at,
                    usage_count: existing.usage_count,
                    last_used: existing.last_used,
                    ..info
                });
            } else {
                models.insert(name, info);
            }
        }
        
        tracing::debug!("Model scan completed, found {} models", models.len());
        Ok(())
    }
    
    /// 检查是否是模型文件
    fn is_model_file(path: &Path) -> bool {
        if let Some(ext) = path.extension() {
            let ext_str = ext.to_string_lossy().to_lowercase();
            matches!(ext_str.as_str(), "pt" | "pth" | "onnx" | "pb" | "h5" | "pkl" | "joblib" | "model")
        } else {
            false
        }
    }
    
    /// 检测模型类型
    fn detect_model_type(path: &Path) -> String {
        if let Some(ext) = path.extension() {
            let ext_str = ext.to_string_lossy().to_lowercase();
            match ext_str.as_str() {
                "pt" | "pth" => "pytorch".to_string(),
                "onnx" => "onnx".to_string(),
                "pb" => "tensorflow".to_string(),
                "h5" => "keras".to_string(),
                "pkl" | "joblib" => "sklearn".to_string(),
                _ => "unknown".to_string(),
            }
        } else {
            "unknown".to_string()
        }
    }
    
    /// 加载模型
    pub async fn load_model(&self, model_name: &str) -> Result<ModelInfo> {
        // 检查是否已加载
        {
            let models = self.loaded_models.read().await;
            if let Some(model) = models.get(model_name) {
                // 更新使用统计
                let mut stats = self.load_stats.write().await;
                stats.cache_hits += 1;
                
                // 更新最后使用时间
                let mut model = model.clone();
                model.last_used = Some(SystemTime::now());
                model.usage_count += 1;
                
                // 写回
                drop(models);
                let mut models = self.loaded_models.write().await;
                models.insert(model_name.to_string(), model.clone());
                
                tracing::debug!("Model {} loaded from cache", model_name);
                return Ok(model);
            }
        }
        
        tracing::info!("Loading model: {}", model_name);
        
        // 更新统计
        {
            let mut stats = self.load_stats.write().await;
            stats.total_loads += 1;
        }
        
        // 查找模型文件
        let model_path = self.config.model_dir.join(model_name);
        if !model_path.exists() {
            // 尝试添加常见扩展名
            let mut found = false;
            for ext in &["pt", "pth", "onnx", "pb", "h5", "pkl"] {
                let path_with_ext = model_path.with_extension(ext);
                if path_with_ext.exists() {
                    let model_info = self.create_model_info(&path_with_ext, model_name).await?;
                    
                    // 添加到缓存
                    let mut models = self.loaded_models.write().await;
                    models.insert(model_name.to_string(), model_info.clone());
                    
                    // 更新统计
                    let mut stats = self.load_stats.write().await;
                    stats.successful_loads += 1;
                    
                    found = true;
                    return Ok(model_info);
                }
            }
            
            if !found {
                return Err(EdgeComputeError::Validation {
                    message: format!("Model not found: {}", model_name),
                    field: Some("model_name".to_string()),
                    value: Some(model_name.to_string()),
                });
            }
        }
        
        // 创建模型信息
        let model_info = self.create_model_info(&model_path, model_name).await?;
        
        // 添加到缓存
        let mut models = self.loaded_models.write().await;
        models.insert(model_name.to_string(), model_info.clone());
        
        // 更新统计
        let mut stats = self.load_stats.write().await;
        stats.successful_loads += 1;
        
        tracing::info!("Model {} loaded successfully", model_name);
        Ok(model_info)
    }
    
    /// 创建模型信息
    async fn create_model_info(&self, path: &Path, name: &str) -> Result<ModelInfo> {
        let metadata = path.metadata()
            .map_err(|e| EdgeComputeError::Config {
                message: format!("Failed to read model metadata: {}", e),
                source: Some("model-loader".to_string()),
            })?;
        
        let file_size = metadata.len();
        let model_type = Self::detect_model_type(path);
        
        Ok(ModelInfo {
            name: name.to_string(),
            path: path.to_path_buf(),
            model_type,
            file_size,
            loaded_at: Some(SystemTime::now()),
            usage_count: 1,
            last_used: Some(SystemTime::now()),
            metadata: HashMap::new(),
        })
    }
    
    /// 列出所有模型
    pub async fn list_models(&self) -> Vec<ModelInfo> {
        let models = self.loaded_models.read().await;
        models.values().cloned().collect()
    }
    
    /// 获取模型信息
    pub async fn get_model_info(&self, model_name: &str) -> Option<ModelInfo> {
        let models = self.loaded_models.read().await;
        models.get(model_name).cloned()
    }
    
    /// 卸载模型（从缓存中移除）
    pub async fn unload_model(&self, model_name: &str) -> Result<()> {
        let mut models = self.loaded_models.write().await;
        if models.remove(model_name).is_some() {
            tracing::info!("Model {} unloaded from cache", model_name);
            Ok(())
        } else {
            Err(EdgeComputeError::Validation {
                message: format!("Model not found in cache: {}", model_name),
                field: Some("model_name".to_string()),
                value: Some(model_name.to_string()),
            })
        }
    }
    
    /// 获取加载统计
    pub async fn get_stats(&self) -> LoadStats {
        self.load_stats.read().await.clone()
    }
    
    /// 清理缓存（移除未使用的模型）
    pub async fn clear_cache(&self) -> Result<()> {
        tracing::info!("Clearing model cache");
        
        let mut models = self.loaded_models.write().await;
        let before_count = models.len();
        
        // 移除未使用的模型（超过1小时未使用）
        let cutoff_time = SystemTime::now() - Duration::from_secs(3600);
        models.retain(|_, model| {
            if let Some(last_used) = model.last_used {
                last_used > cutoff_time
            } else {
                true // 保留从未使用的模型
            }
        });
        
        let after_count = models.len();
        tracing::info!("Cache cleared: {} models removed ({} remaining)", 
            before_count - after_count, after_count);
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_model_loader_creation() {
        let config = ModelLoaderConfig {
            model_dir: PathBuf::from("./test_models"),
            auto_scan: false, // 测试时不启动扫描任务
            ..Default::default()
        };
        
        let loader = ModelLoader::new(config);
        assert!(loader.is_ok());
    }
    
    #[tokio::test]
    async fn test_model_loader_list() {
        let config = ModelLoaderConfig {
            model_dir: PathBuf::from("./test_models"),
            auto_scan: false,
            ..Default::default()
        };
        
        let loader = ModelLoader::new(config).unwrap();
        let models = loader.list_models().await;
        assert_eq!(models.len(), 0);
    }
}

