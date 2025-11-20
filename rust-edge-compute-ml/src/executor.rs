//! Candle ML Executor实现
//!
//! 集成DeviceManager和ModelManager，提供完整的ML推理能力

use rust_edge_compute_core::core::{
    Executor, ComputeRequest, ComputeResponse, ResourceRequirements, HealthStatus,
};
use rust_edge_compute_core::core::error::{Result, EdgeComputeError};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::Instant;

use crate::device_manager::{DeviceManager, DeviceManagerConfig};
use crate::model_manager::{ModelManager, ModelCacheConfig, ModelType};
use crate::inference::{
    TextGenerationService, ImageClassificationService, 
    SpeechRecognitionService, MultimodalService,
};

/// Candle ML Executor配置
#[derive(Debug, Clone)]
pub struct CandleMlExecutorConfig {
    /// 设备管理器配置
    pub device_config: DeviceManagerConfig,
    /// 模型缓存配置
    pub model_cache_config: ModelCacheConfig,
    /// 最大并发推理任务数
    pub max_concurrent_inferences: usize,
    /// 推理超时时间（毫秒）
    pub inference_timeout_ms: u64,
}

impl Default for CandleMlExecutorConfig {
    fn default() -> Self {
        Self {
            device_config: DeviceManagerConfig::default(),
            model_cache_config: ModelCacheConfig::default(),
            max_concurrent_inferences: 5,
            inference_timeout_ms: 60000, // 60秒
        }
    }
}

/// 推理统计信息
#[derive(Debug, Clone, Default)]
struct InferenceStats {
    /// 总推理次数
    total_inferences: u64,
    /// 成功推理次数
    successful_inferences: u64,
    /// 失败推理次数
    failed_inferences: u64,
    /// 总推理时间（毫秒）
    total_inference_time_ms: u64,
}

/// Candle ML Executor
pub struct CandleMlExecutor {
    /// Executor名称
    name: String,
    /// Executor版本
    version: String,
    /// 支持的算法列表
    supported_algorithms: Vec<String>,
    /// 配置
    config: CandleMlExecutorConfig,
    /// 设备管理器
    device_manager: Arc<DeviceManager>,
    /// 模型管理器
    model_manager: Arc<ModelManager>,
    /// 文本生成服务
    text_generation_service: Arc<TextGenerationService>,
    /// 图像分类服务
    image_classification_service: Arc<ImageClassificationService>,
    /// 语音识别服务
    speech_recognition_service: Arc<SpeechRecognitionService>,
    /// 多模态服务
    multimodal_service: Arc<MultimodalService>,
    /// 推理统计
    inference_stats: Arc<RwLock<InferenceStats>>,
    /// 当前活跃推理任务数
    active_inferences: Arc<RwLock<usize>>,
}

impl CandleMlExecutor {
    /// 创建新的Candle ML Executor
    pub fn new() -> Result<Self> {
        Self::with_config(CandleMlExecutorConfig::default())
    }
    
    /// 使用配置创建新的Candle ML Executor
    pub fn with_config(config: CandleMlExecutorConfig) -> Result<Self> {
        // 创建设备管理器
        let device_manager = Arc::new(
            DeviceManager::new(config.device_config.clone())
                .map_err(|e| EdgeComputeError::Config {
                    message: format!("Failed to create device manager: {}", e),
                    source: Some("candle-ml".to_string()),
                })?
        );
        
        // 创建模型管理器
        let model_manager = Arc::new(
            ModelManager::new(
                config.model_cache_config.clone(),
                Arc::clone(&device_manager),
            )
        );
        
        // 创建推理服务
        let text_generation_service = Arc::new(
            TextGenerationService::new(
                Arc::clone(&device_manager),
                Arc::clone(&model_manager),
            )
        );
        
        let image_classification_service = Arc::new(
            ImageClassificationService::new(
                Arc::clone(&device_manager),
                Arc::clone(&model_manager),
            )
        );
        
        let speech_recognition_service = Arc::new(
            SpeechRecognitionService::new(
                Arc::clone(&device_manager),
                Arc::clone(&model_manager),
            )
        );
        
        let multimodal_service = Arc::new(
            MultimodalService::new(
                Arc::clone(&device_manager),
                Arc::clone(&model_manager),
            )
        );
        
        Ok(Self {
            name: "candle-ml".to_string(),
            version: "1.0.0".to_string(),
            supported_algorithms: vec![
                "llama".to_string(),
                "yolo".to_string(),
                "whisper".to_string(),
                "bert".to_string(),
            ],
            config,
            device_manager,
            model_manager,
            text_generation_service,
            image_classification_service,
            speech_recognition_service,
            multimodal_service,
            inference_stats: Arc::new(RwLock::new(InferenceStats::default())),
            active_inferences: Arc::new(RwLock::new(0)),
        })
    }
    
    /// 执行推理（内部方法）
    async fn execute_inference_internal(
        &self,
        request: ComputeRequest,
    ) -> Result<ComputeResponse> {
        let start_time = Instant::now();
        
        // 检查并发限制
        {
            let mut active = self.active_inferences.write().await;
            if *active >= self.config.max_concurrent_inferences {
                return Err(EdgeComputeError::ResourceExhausted {
                    resource: "concurrency".to_string(),
                    message: format!(
                        "Max concurrent inferences limit: {}",
                        self.config.max_concurrent_inferences
                    ),
                });
            }
            *active += 1;
        }
        
        // 更新统计
        {
            let mut stats = self.inference_stats.write().await;
            stats.total_inferences += 1;
        }
        
        // 执行推理
        let result = match request.algorithm.as_str() {
            "llama" => self.execute_text_generation(request).await,
            "yolo" => self.execute_image_classification(request).await,
            "whisper" => self.execute_speech_recognition(request).await,
            "bert" => self.execute_multimodal(request).await,
            _ => Err(EdgeComputeError::Validation {
                message: format!("Unsupported algorithm: {}", request.algorithm),
                field: Some("algorithm".to_string()),
                value: Some(request.algorithm.clone()),
            }),
        };
        
        // 更新统计
        {
            let mut stats = self.inference_stats.write().await;
            let inference_time = start_time.elapsed().as_millis() as u64;
            stats.total_inference_time_ms += inference_time;
            
            match &result {
                Ok(_) => stats.successful_inferences += 1,
                Err(_) => stats.failed_inferences += 1,
            }
        }
        
        // 减少活跃推理任务数
        {
            let mut active = self.active_inferences.write().await;
            *active = active.saturating_sub(1);
        }
        
        result
    }
    
    /// 执行文本生成推理（LLaMA等）
    async fn execute_text_generation(
        &self,
        request: ComputeRequest,
    ) -> Result<ComputeResponse> {
        tracing::info!("Executing text generation inference: {}", request.id);
        
        // 获取参数
        let model_id = request.parameters
            .get("model_id")
            .and_then(|v| v.as_str())
            .unwrap_or("default-llama");
        
        let prompt = request.parameters
            .get("prompt")
            .and_then(|v| v.as_str())
            .ok_or_else(|| EdgeComputeError::Validation {
                message: "Missing 'prompt' parameter".to_string(),
                field: Some("parameters.prompt".to_string()),
                value: None,
            })?;
        
        let max_tokens = request.parameters
            .get("max_tokens")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize);
        
        let temperature = request.parameters
            .get("temperature")
            .and_then(|v| v.as_f64());
        
        // 执行推理
        let generated_text = self.text_generation_service
            .generate(model_id, prompt, max_tokens, temperature)
            .await?;
        
        Ok(ComputeResponse::success(
            request.id,
            serde_json::json!({
                "generated_text": generated_text,
                "algorithm": "llama",
            }),
            0,
        ))
    }
    
    /// 执行图像分类推理（YOLO等）
    async fn execute_image_classification(
        &self,
        request: ComputeRequest,
    ) -> Result<ComputeResponse> {
        tracing::info!("Executing image classification inference: {}", request.id);
        
        // 获取参数
        let model_id = request.parameters
            .get("model_id")
            .and_then(|v| v.as_str())
            .unwrap_or("default-yolo");
        
        let image_data = request.parameters
            .get("image_data")
            .and_then(|v| v.as_str())
            .and_then(|s| base64::decode(s).ok())
            .ok_or_else(|| EdgeComputeError::Validation {
                message: "Missing or invalid 'image_data' parameter (base64 encoded)".to_string(),
                field: Some("parameters.image_data".to_string()),
                value: None,
            })?;
        
        let image_format = request.parameters
            .get("image_format")
            .and_then(|v| v.as_str())
            .unwrap_or("jpg");
        
        // 执行推理
        let classifications = self.image_classification_service
            .classify(model_id, &image_data, image_format)
            .await?;
        
        Ok(ComputeResponse::success(
            request.id,
            serde_json::json!({
                "classifications": classifications,
                "algorithm": "yolo",
            }),
            0,
        ))
    }
    
    /// 执行语音识别推理（Whisper等）
    async fn execute_speech_recognition(
        &self,
        request: ComputeRequest,
    ) -> Result<ComputeResponse> {
        tracing::info!("Executing speech recognition inference: {}", request.id);
        
        // 获取参数
        let model_id = request.parameters
            .get("model_id")
            .and_then(|v| v.as_str())
            .unwrap_or("default-whisper");
        
        let audio_data = request.parameters
            .get("audio_data")
            .and_then(|v| v.as_str())
            .and_then(|s| base64::decode(s).ok())
            .ok_or_else(|| EdgeComputeError::Validation {
                message: "Missing or invalid 'audio_data' parameter (base64 encoded)".to_string(),
                field: Some("parameters.audio_data".to_string()),
                value: None,
            })?;
        
        let sample_rate = request.parameters
            .get("sample_rate")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32)
            .unwrap_or(16000);
        
        // 执行推理
        let transcribed_text = self.speech_recognition_service
            .transcribe(model_id, &audio_data, sample_rate)
            .await?;
        
        Ok(ComputeResponse::success(
            request.id,
            serde_json::json!({
                "transcribed_text": transcribed_text,
                "algorithm": "whisper",
            }),
            0,
        ))
    }
    
    /// 执行多模态推理（BERT等）
    async fn execute_multimodal(
        &self,
        request: ComputeRequest,
    ) -> Result<ComputeResponse> {
        tracing::info!("Executing multimodal inference: {}", request.id);
        
        // 获取参数
        let model_id = request.parameters
            .get("model_id")
            .and_then(|v| v.as_str())
            .unwrap_or("default-bert");
        
        let text_input = request.parameters
            .get("text_input")
            .and_then(|v| v.as_str());
        
        let image_input = request.parameters
            .get("image_input")
            .and_then(|v| v.as_str())
            .and_then(|s| base64::decode(s).ok());
        
        let audio_input = request.parameters
            .get("audio_input")
            .and_then(|v| v.as_str())
            .and_then(|s| base64::decode(s).ok());
        
        // 执行推理
        let result = self.multimodal_service
            .infer(
                model_id,
                text_input,
                image_input.as_deref(),
                audio_input.as_deref(),
            )
            .await?;
        
        Ok(ComputeResponse::success(
            request.id,
            result,
            0,
        ))
    }
    
    /// 获取推理统计信息
    pub async fn get_inference_stats(&self) -> InferenceStats {
        self.inference_stats.read().await.clone()
    }
    
    /// 获取当前活跃推理任务数
    pub async fn get_active_inferences_count(&self) -> usize {
        *self.active_inferences.read().await
    }
}

#[async_trait]
impl Executor for CandleMlExecutor {
    async fn execute(&self, request: ComputeRequest) -> Result<ComputeResponse> {
        self.execute_inference_internal(request).await
    }
    
    fn name(&self) -> &str {
        &self.name
    }
    
    fn version(&self) -> &str {
        &self.version
    }
    
    fn supported_algorithms(&self) -> Vec<String> {
        self.supported_algorithms.clone()
    }
    
    fn resource_requirements(&self) -> ResourceRequirements {
        ResourceRequirements {
            cpu_cores: 2.0,
            memory_mb: 4096,
            disk_mb: Some(2048),
            gpu_memory_mb: Some(8192),
            network_mbps: None,
        }
    }
    
    async fn health_check(&self) -> Result<HealthStatus> {
        let active_inferences = self.get_active_inferences_count().await;
        let stats = self.get_inference_stats().await;
        let devices = self.device_manager.list_devices().await;
        let models = self.model_manager.list_models().await;
        
        let mut details = HashMap::new();
        details.insert("active_inferences".to_string(), active_inferences.to_string());
        details.insert("total_inferences".to_string(), stats.total_inferences.to_string());
        details.insert("successful_inferences".to_string(), stats.successful_inferences.to_string());
        details.insert("failed_inferences".to_string(), stats.failed_inferences.to_string());
        details.insert("available_devices".to_string(), devices.len().to_string());
        details.insert("cached_models".to_string(), models.len().to_string());
        
        let healthy = active_inferences < self.config.max_concurrent_inferences;
        let message = if healthy {
            "Candle ML Executor is healthy".to_string()
        } else {
            format!("Candle ML Executor health check failed: {} active inferences", active_inferences)
        };
        
        Ok(HealthStatus {
            healthy,
            message,
            details,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_candle_ml_executor_creation() {
        let executor = CandleMlExecutor::new().unwrap();
        assert_eq!(executor.name(), "candle-ml");
        assert_eq!(executor.version(), "1.0.0");
    }
    
    #[tokio::test]
    async fn test_candle_ml_executor_execute() {
        let executor = CandleMlExecutor::new().unwrap();
        
        let request = ComputeRequest {
            id: "test-1".to_string(),
            algorithm: "llama".to_string(),
            parameters: serde_json::json!({"prompt": "Hello"}),
            timeout_seconds: Some(30),
        };
        
        let response = executor.execute(request).await.unwrap();
        assert_eq!(response.status, rust_edge_compute_core::core::TaskStatus::Completed);
    }
    
    #[tokio::test]
    async fn test_candle_ml_executor_health_check() {
        let executor = CandleMlExecutor::new().unwrap();
        
        let health = executor.health_check().await.unwrap();
        assert!(health.healthy);
        assert!(health.details.contains_key("active_inferences"));
    }
}
