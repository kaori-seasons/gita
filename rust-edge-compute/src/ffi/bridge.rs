//! FFI 桥接 - 生产级实现
//!
//! 使用 rust-edge-compute-cpp 包提供的 CXX 桥接实现

pub use super::super::core::{ComputeRequest, ComputeResponse};
use std::collections::HashMap;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// C++ 算法执行器
pub struct CppAlgorithmExecutor {
    bridge: rust_edge_compute_cpp::CppAlgorithmExecutorBridge,
    initialized: bool,
}

impl CppAlgorithmExecutor {
    /// 创建新的执行器
    pub fn new() -> Result<Self> {
        let bridge = rust_edge_compute_cpp::CppAlgorithmExecutorBridge::new()
            .map_err(|e| format!("Failed to create CppAlgorithmExecutorBridge: {}", e))?;
        Ok(Self {
            bridge,
            initialized: false,
        })
    }

    /// 初始化执行器
    pub fn initialize(&mut self) -> Result<bool> {
        let result = self.bridge.initialize()
            .map_err(|e| format!("Failed to initialize bridge: {}", e))?;
        self.initialized = result;
        Ok(result)
    }

    /// 检查是否已初始化
    pub fn is_initialized(&self) -> bool {
        self.initialized && self.bridge.is_initialized()
    }

    /// 获取可用插件列表
    pub fn get_available_plugins(&self) -> Result<Vec<String>> {
        self.bridge.get_available_plugins()
            .map_err(|e| format!("Failed to get available plugins: {}", e).into())
    }

    /// 执行算法
    pub async fn execute(&self, request: ComputeRequest) -> Result<ComputeResponse> {
        if !self.is_initialized() {
            return Err("CppAlgorithmExecutor not initialized".into());
        }

        let algorithm = request.algorithm.clone();
        let parameters = request.parameters.clone();
        let request_id = request.id.clone();
        
        // 在阻塞线程池中执行FFI调用
        let bridge_clone = self.bridge.clone();
        let result = tokio::task::spawn_blocking(move || {
            bridge_clone.execute_algorithm(&algorithm, &parameters, "default_device")
        }).await
        .map_err(|e| format!("Task join error: {}", e))??;

        Ok(ComputeResponse::success(
            request_id,
            result.result_json,
            result.execution_time_ms,
        ))
    }

    /// 执行插件
    pub async fn execute_plugin(
        &self,
        plugin_name: &str,
        input_data: serde_json::Value,
        _parameters: HashMap<String, String>,
    ) -> Result<serde_json::Value> {
        if !self.is_initialized() {
            return Err("CppAlgorithmExecutor not initialized".into());
        }

        let plugin_name = plugin_name.to_string();
        
        // 在阻塞线程池中执行FFI调用
        let bridge_clone = self.bridge.clone();
        let result = tokio::task::spawn_blocking(move || {
            bridge_clone.execute_algorithm(&plugin_name, &input_data, "default_device")
        }).await
        .map_err(|e| format!("Task join error: {}", e))??;

        Ok(result.result_json)
    }
}

impl Clone for CppAlgorithmExecutor {
    fn clone(&self) -> Self {
        Self {
            bridge: self.bridge.clone(),
            initialized: self.initialized,
        }
    }
}

impl Default for CppAlgorithmExecutor {
    fn default() -> Self {
        Self::new().expect("Failed to create CppAlgorithmExecutor")
    }
}
