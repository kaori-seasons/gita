//! C++ Executor实现
//!
//! 集成FFI桥接代码，提供完整的C++算法执行能力

use rust_edge_compute_core::core::{
    Executor, ComputeRequest, ComputeResponse, ResourceRequirements, HealthStatus,
};
use rust_edge_compute_core::core::error::{Result, EdgeComputeError};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::Instant;

/// C++ Executor配置
#[derive(Debug, Clone)]
pub struct CppExecutorConfig {
    /// 最大并发任务数
    pub max_concurrent_tasks: usize,
    /// 任务超时时间（毫秒）
    pub task_timeout_ms: u64,
    /// 是否启用资源监控
    pub enable_resource_monitoring: bool,
    /// 内存限制（MB）
    pub memory_limit_mb: Option<u64>,
}

impl Default for CppExecutorConfig {
    fn default() -> Self {
        Self {
            max_concurrent_tasks: 10,
            task_timeout_ms: 30000,
            enable_resource_monitoring: true,
            memory_limit_mb: Some(512),
        }
    }
}

/// 资源使用统计
#[derive(Debug, Clone, Default)]
struct ResourceStats {
    /// 当前使用的内存（字节）
    current_memory_bytes: u64,
    /// 峰值内存使用（字节）
    peak_memory_bytes: u64,
    /// 总任务数
    total_tasks: u64,
    /// 成功任务数
    successful_tasks: u64,
    /// 失败任务数
    failed_tasks: u64,
    /// 总执行时间（毫秒）
    total_execution_time_ms: u64,
}

/// C++ Executor
pub struct CppExecutor {
    /// Executor名称
    name: String,
    /// Executor版本
    version: String,
    /// 支持的算法列表
    supported_algorithms: Vec<String>,
    /// 配置
    config: CppExecutorConfig,
    /// 资源统计
    resource_stats: Arc<RwLock<ResourceStats>>,
    /// 当前活跃任务数
    active_tasks: Arc<RwLock<usize>>,
    /// 初始化状态
    initialized: Arc<RwLock<bool>>,
}

impl CppExecutor {
    /// 创建新的C++ Executor
    pub fn new() -> Self {
        Self::with_config(CppExecutorConfig::default())
    }
    
    /// 使用配置创建新的C++ Executor
    pub fn with_config(config: CppExecutorConfig) -> Self {
        Self {
            name: "cpp".to_string(),
            version: "1.0.0".to_string(),
            supported_algorithms: vec![
                "add".to_string(),
                "multiply".to_string(),
                "complex_math".to_string(),
                "vibrate31".to_string(),
            ],
            config,
            resource_stats: Arc::new(RwLock::new(ResourceStats::default())),
            active_tasks: Arc::new(RwLock::new(0)),
            initialized: Arc::new(RwLock::new(false)),
        }
    }
    
    /// 初始化executor
    pub async fn initialize(&self) -> Result<()> {
        let mut initialized = self.initialized.write().await;
        if *initialized {
            return Ok(());
        }
        
        // 在阻塞线程池中初始化FFI桥接
        let init_result = tokio::task::spawn_blocking(|| {
            let mut executor = crate::ffi::CppAlgorithmExecutorBridge::new()
                .map_err(|e| format!("Failed to create executor: {}", e))?;
            executor.initialize()
                .map_err(|e| format!("Failed to initialize executor: {}", e))?;
            Ok::<(), String>(())
        }).await
        .map_err(|e| EdgeComputeError::AlgorithmExecution {
            message: format!("Task join error: {}", e),
            algorithm: None,
            input_size: None,
        })?;
        
        init_result.map_err(|e| EdgeComputeError::AlgorithmExecution {
            message: e,
            algorithm: None,
            input_size: None,
        })?;
        
        *initialized = true;
        tracing::info!("C++ Executor initialized");
        Ok(())
    }
    
    /// 检查是否已初始化
    pub async fn is_initialized(&self) -> bool {
        *self.initialized.read().await
    }
    
    /// 执行C++算法（内部方法）
    async fn execute_algorithm_internal(
        &self,
        request: ComputeRequest,
    ) -> Result<ComputeResponse> {
        let start_time = Instant::now();
        
        // 检查是否已初始化
        if !self.is_initialized().await {
            return Err(EdgeComputeError::AlgorithmExecution {
                message: format!("C++ Executor not initialized: {}", self.name),
                algorithm: Some(request.algorithm.clone()),
                input_size: None,
            });
        }
        
        // 检查资源限制
        if let Some(memory_limit) = self.config.memory_limit_mb {
            let stats = self.resource_stats.read().await;
            if stats.current_memory_bytes > memory_limit * 1024 * 1024 {
                return Err(EdgeComputeError::ResourceExhausted {
                    resource: "memory".to_string(),
                    requested: Some(memory_limit * 1024 * 1024),
                    available: Some(stats.current_memory_bytes),
                });
            }
        }
        
        // 检查并发限制
        {
            let mut active = self.active_tasks.write().await;
            if *active >= self.config.max_concurrent_tasks {
                return Err(EdgeComputeError::ResourceExhausted {
                    resource: "concurrency".to_string(),
                    requested: Some(self.config.max_concurrent_tasks as u64),
                    available: Some(*active as u64),
                });
            }
            *active += 1;
        }
        
        // 更新统计
        {
            let mut stats = self.resource_stats.write().await;
            stats.total_tasks += 1;
        }
        
        // 执行算法
        let result = self.execute_cpp_algorithm(&request).await;
        
        // 更新统计
        {
            let mut stats = self.resource_stats.write().await;
            let execution_time = start_time.elapsed().as_millis() as u64;
            stats.total_execution_time_ms += execution_time;
            
            match &result {
                Ok(_) => stats.successful_tasks += 1,
                Err(_) => stats.failed_tasks += 1,
            }
        }
        
        // 减少活跃任务数
        {
            let mut active = self.active_tasks.write().await;
            *active = active.saturating_sub(1);
        }
        
        result
    }
    
    /// 执行C++算法（实际实现）
    async fn execute_cpp_algorithm(
        &self,
        request: &ComputeRequest,
    ) -> Result<ComputeResponse> {
        tracing::info!("C++ Executor executing algorithm: {}", request.algorithm);
        
        // 使用FFI桥接执行算法
        // 注意：这里需要在tokio运行时中执行同步FFI调用
        let algorithm_name = request.algorithm.clone();
        let parameters = request.parameters.clone();
        
        // 在阻塞线程池中执行FFI调用
        let result = tokio::task::spawn_blocking(move || {
            // 创建执行器
            let mut executor = crate::ffi::CppAlgorithmExecutorBridge::new()
                .map_err(|e| EdgeComputeError::AlgorithmExecution {
                    message: format!("Failed to create C++ executor: {}", e),
                    algorithm: Some(algorithm_name.clone()),
                    input_size: None,
                })?;
            
            // 初始化
            executor.initialize()
                .map_err(|e| EdgeComputeError::AlgorithmExecution {
                    message: format!("Failed to initialize C++ executor: {}", e),
                    algorithm: Some(algorithm_name.clone()),
                    input_size: None,
                })?;
            
            // 执行算法
            executor.execute_algorithm(&algorithm_name, &parameters)
                .map_err(|e| EdgeComputeError::AlgorithmExecution {
                    message: format!("C++ algorithm execution failed: {}", e),
                    algorithm: Some(algorithm_name),
                    input_size: None,
                })
        }).await
        .map_err(|e| EdgeComputeError::AlgorithmExecution {
            message: format!("Task join error: {}", e),
            algorithm: Some(request.algorithm.clone()),
            input_size: None,
        })?;
        
        match result {
            Ok(result_json) => {
                Ok(ComputeResponse::success(
                    request.id.clone(),
                    result_json,
                    0,
                ))
            }
            Err(e) => Err(e),
        }
    }
    
    /// 获取资源统计信息
    pub async fn get_resource_stats(&self) -> ResourceStats {
        self.resource_stats.read().await.clone()
    }
    
    /// 获取当前活跃任务数
    pub async fn get_active_tasks_count(&self) -> usize {
        *self.active_tasks.read().await
    }
}

#[async_trait]
impl Executor for CppExecutor {
    async fn execute(&self, request: ComputeRequest) -> Result<ComputeResponse> {
        self.execute_algorithm_internal(request).await
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
            cpu_cores: 1.0,
            memory_mb: self.config.memory_limit_mb.unwrap_or(256),
            disk_mb: Some(512),
            gpu_memory_mb: None,
            network_mbps: None,
        }
    }
    
    async fn health_check(&self) -> Result<HealthStatus> {
        let initialized = self.is_initialized().await;
        let active_tasks = self.get_active_tasks_count().await;
        let stats = self.get_resource_stats().await;
        
        let mut details = HashMap::new();
        details.insert("initialized".to_string(), initialized.to_string());
        details.insert("active_tasks".to_string(), active_tasks.to_string());
        details.insert("total_tasks".to_string(), stats.total_tasks.to_string());
        details.insert("successful_tasks".to_string(), stats.successful_tasks.to_string());
        details.insert("failed_tasks".to_string(), stats.failed_tasks.to_string());
        details.insert("current_memory_mb".to_string(), 
            (stats.current_memory_bytes / 1024 / 1024).to_string());
        details.insert("peak_memory_mb".to_string(), 
            (stats.peak_memory_bytes / 1024 / 1024).to_string());
        
        let healthy = initialized && active_tasks < self.config.max_concurrent_tasks;
        let message = if healthy {
            "C++ Executor is healthy".to_string()
        } else {
            format!("C++ Executor health check failed: {} active tasks", active_tasks)
        };
        
        Ok(HealthStatus {
            healthy,
            message,
            details,
        })
    }
}

impl Default for CppExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_cpp_executor_creation() {
        let executor = CppExecutor::new();
        assert_eq!(executor.name(), "cpp");
        assert_eq!(executor.version(), "1.0.0");
    }
    
    #[tokio::test]
    async fn test_cpp_executor_initialization() {
        let executor = CppExecutor::new();
        assert!(!executor.is_initialized().await);
        
        executor.initialize().await.unwrap();
        assert!(executor.is_initialized().await);
    }
    
    #[tokio::test]
    async fn test_cpp_executor_execute_add() {
        let executor = CppExecutor::new();
        executor.initialize().await.unwrap();
        
        let request = ComputeRequest {
            id: "test-1".to_string(),
            algorithm: "add".to_string(),
            parameters: serde_json::json!({"a": 10, "b": 20}),
            timeout_seconds: Some(30),
        };
        
        let response = executor.execute(request).await.unwrap();
        assert_eq!(response.status, rust_edge_compute_core::core::TaskStatus::Completed);
        assert!(response.result.is_some());
    }
    
    #[tokio::test]
    async fn test_cpp_executor_health_check() {
        let executor = CppExecutor::new();
        executor.initialize().await.unwrap();
        
        let health = executor.health_check().await.unwrap();
        assert!(health.healthy);
        assert!(health.details.contains_key("initialized"));
    }
    
    #[tokio::test]
    async fn test_cpp_executor_resource_stats() {
        let executor = CppExecutor::new();
        executor.initialize().await.unwrap();
        
        let stats = executor.get_resource_stats().await;
        assert_eq!(stats.total_tasks, 0);
    }
}
