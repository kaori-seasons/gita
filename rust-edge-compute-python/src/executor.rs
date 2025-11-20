//! Python WASM Executor实现
//!
//! 集成Python执行器和WASM沙箱，提供完整的Python算法执行能力

use rust_edge_compute_core::core::{
    Executor, ComputeRequest, ComputeResponse, ResourceRequirements, HealthStatus,
};
use rust_edge_compute_core::core::error::{Result, EdgeComputeError};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::Instant;

use crate::python::{PythonExecutor, PythonExecutorConfig};
use crate::wasm::{WasmSandbox, WasmSandboxConfig};

/// Python WASM Executor配置
#[derive(Debug, Clone)]
pub struct PythonWasmExecutorConfig {
    /// Python执行器配置
    pub python_config: PythonExecutorConfig,
    /// WASM沙箱配置
    pub wasm_config: WasmSandboxConfig,
    /// 最大并发任务数
    pub max_concurrent_tasks: usize,
    /// 任务超时时间（毫秒）
    pub task_timeout_ms: u64,
}

impl Default for PythonWasmExecutorConfig {
    fn default() -> Self {
        Self {
            python_config: PythonExecutorConfig::default(),
            wasm_config: WasmSandboxConfig::default(),
            max_concurrent_tasks: 10,
            task_timeout_ms: 30000,
        }
    }
}

/// 执行统计
#[derive(Debug, Clone, Default)]
struct ExecutionStats {
    /// 总执行次数
    total_executions: u64,
    /// 成功执行次数
    successful_executions: u64,
    /// 失败执行次数
    failed_executions: u64,
    /// 总执行时间（毫秒）
    total_execution_time_ms: u64,
}

/// Python WASM Executor
pub struct PythonWasmExecutor {
    /// Executor名称
    name: String,
    /// Executor版本
    version: String,
    /// 支持的算法列表
    supported_algorithms: Vec<String>,
    /// 配置
    config: PythonWasmExecutorConfig,
    /// Python执行器
    python_executor: Arc<PythonExecutor>,
    /// WASM沙箱
    wasm_sandbox: Arc<WasmSandbox>,
    /// 执行统计
    execution_stats: Arc<RwLock<ExecutionStats>>,
    /// 当前活跃任务数
    active_tasks: Arc<RwLock<usize>>,
}

impl PythonWasmExecutor {
    /// 创建新的Python WASM Executor
    pub fn new() -> Result<Self> {
        Self::with_config(PythonWasmExecutorConfig::default())
    }
    
    /// 使用配置创建新的Python WASM Executor
    pub fn with_config(config: PythonWasmExecutorConfig) -> Result<Self> {
        // 创建Python执行器
        let python_executor = Arc::new(
            PythonExecutor::new(config.python_config.clone())
                .map_err(|e| EdgeComputeError::Config {
                    message: format!("Failed to create Python executor: {}", e),
                    source: Some("python-wasm".to_string()),
                })?
        );
        
        // 创建WASM沙箱
        let wasm_sandbox = Arc::new(
            WasmSandbox::new(config.wasm_config.clone())
                .map_err(|e| EdgeComputeError::Config {
                    message: format!("Failed to create WASM sandbox: {}", e),
                    source: Some("python-wasm".to_string()),
                })?
        );
        
        Ok(Self {
            name: "python-wasm".to_string(),
            version: "1.0.0".to_string(),
            supported_algorithms: vec![
                "custom_python".to_string(),
                "candle_python".to_string(),
            ],
            config,
            python_executor,
            wasm_sandbox,
            execution_stats: Arc::new(RwLock::new(ExecutionStats::default())),
            active_tasks: Arc::new(RwLock::new(0)),
        })
    }
    
    /// 执行任务（内部方法）
    async fn execute_task_internal(
        &self,
        request: ComputeRequest,
    ) -> Result<ComputeResponse> {
        let start_time = Instant::now();
        
        // 检查并发限制
        {
            let mut active = self.active_tasks.write().await;
            if *active >= self.config.max_concurrent_tasks {
                return Err(EdgeComputeError::ResourceExhausted {
                    resource: "concurrency".to_string(),
                    message: format!(
                        "Max concurrent tasks limit: {}",
                        self.config.max_concurrent_tasks
                    ),
                });
            }
            *active += 1;
        }
        
        // 更新统计
        {
            let mut stats = self.execution_stats.write().await;
            stats.total_executions += 1;
        }
        
        // 执行任务
        let result = match request.algorithm.as_str() {
            "custom_python" => self.execute_python_code(request).await,
            "candle_python" => self.execute_candle_python(request).await,
            _ => Err(EdgeComputeError::Validation {
                message: format!("Unsupported algorithm: {}", request.algorithm),
                field: Some("algorithm".to_string()),
                value: Some(request.algorithm.clone()),
            }),
        };
        
        // 更新统计
        {
            let mut stats = self.execution_stats.write().await;
            let execution_time = start_time.elapsed().as_millis() as u64;
            stats.total_execution_time_ms += execution_time;
            
            match &result {
                Ok(_) => stats.successful_executions += 1,
                Err(_) => stats.failed_executions += 1,
            }
        }
        
        // 减少活跃任务数
        {
            let mut active = self.active_tasks.write().await;
            *active = active.saturating_sub(1);
        }
        
        result
    }
    
    /// 执行Python代码
    async fn execute_python_code(
        &self,
        request: ComputeRequest,
    ) -> Result<ComputeResponse> {
        tracing::info!("Executing Python code: {}", request.id);
        
        // 获取Python代码
        let code = request.parameters
            .get("code")
            .and_then(|v| v.as_str())
            .ok_or_else(|| EdgeComputeError::Validation {
                message: "Missing 'code' parameter".to_string(),
                field: Some("parameters.code".to_string()),
                value: None,
            })?;
        
        // 执行Python代码
        let output = self.python_executor.execute_code(code).await?;
        
        Ok(ComputeResponse::success(
            request.id,
            serde_json::json!({
                "output": output,
            }),
            0,
        ))
    }
    
    /// 执行Candle Python模型
    async fn execute_candle_python(
        &self,
        request: ComputeRequest,
    ) -> Result<ComputeResponse> {
        tracing::info!("Executing Candle Python model: {}", request.id);
        
        // 获取参数
        let model_path = request.parameters
            .get("model_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| EdgeComputeError::Validation {
                message: "Missing 'model_path' parameter".to_string(),
                field: Some("parameters.model_path".to_string()),
                value: None,
            })?;
        
        let input_data = request.parameters
            .get("input_data")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        
        // 构建Python代码来执行Candle模型
        let python_code = format!(
            r#"
import json
import sys
import candle_pyo3 as candle

# 加载模型
model = candle.load_model("{}")

# 准备输入数据
input_data = json.loads('{}')

# 执行推理
result = model.infer(input_data)

# 返回结果
output = {{
    "result": result,
    "status": "success"
}}

print(json.dumps(output))
"#,
            model_path,
            serde_json::to_string(&input_data).unwrap_or_else(|_| "{}".to_string())
        );
        
        // 执行Python代码
        let output = self.python_executor.execute_code(&python_code).await?;
        
        // 解析输出
        let result: serde_json::Value = serde_json::from_str(&output)
            .unwrap_or_else(|_| serde_json::json!({
                "output": output,
                "status": "success"
            }));
        
        Ok(ComputeResponse::success(
            request.id,
            result,
            0,
        ))
    }
    
    /// 获取执行统计
    pub async fn get_execution_stats(&self) -> ExecutionStats {
        self.execution_stats.read().await.clone()
    }
    
    /// 获取当前活跃任务数
    pub async fn get_active_tasks_count(&self) -> usize {
        *self.active_tasks.read().await
    }
}

#[async_trait]
impl Executor for PythonWasmExecutor {
    async fn execute(&self, request: ComputeRequest) -> Result<ComputeResponse> {
        self.execute_task_internal(request).await
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
            memory_mb: self.config.python_config.max_memory_mb.unwrap_or(512),
            disk_mb: Some(1024),
            gpu_memory_mb: None,
            network_mbps: None,
        }
    }
    
    async fn health_check(&self) -> Result<HealthStatus> {
        let active_tasks = self.get_active_tasks_count().await;
        let stats = self.get_execution_stats().await;
        let python_stats = self.python_executor.get_stats().await;
        let wasm_stats = self.wasm_sandbox.get_stats().await;
        
        let mut details = HashMap::new();
        details.insert("active_tasks".to_string(), active_tasks.to_string());
        details.insert("total_executions".to_string(), stats.total_executions.to_string());
        details.insert("successful_executions".to_string(), stats.successful_executions.to_string());
        details.insert("failed_executions".to_string(), stats.failed_executions.to_string());
        details.insert("python_total_executions".to_string(), python_stats.total_executions.to_string());
        details.insert("wasm_total_executions".to_string(), wasm_stats.total_executions.to_string());
        
        let healthy = active_tasks < self.config.max_concurrent_tasks;
        let message = if healthy {
            "Python WASM Executor is healthy".to_string()
        } else {
            format!("Python WASM Executor health check failed: {} active tasks", active_tasks)
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
    async fn test_python_wasm_executor_creation() {
        let executor = PythonWasmExecutor::new().unwrap();
        assert_eq!(executor.name(), "python-wasm");
        assert_eq!(executor.version(), "1.0.0");
    }
    
    #[tokio::test]
    async fn test_python_wasm_executor_health_check() {
        let executor = PythonWasmExecutor::new().unwrap();
        
        let health = executor.health_check().await.unwrap();
        assert!(health.healthy);
        assert!(health.details.contains_key("active_tasks"));
    }
}
