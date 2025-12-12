//! C++ FFI桥接 - 生产级实现
//! 
//! 提供 Rust 与 C++ 的安全互操作接口
//! 使用bindgen框架实现真实的C++ FFI

use serde_json;
use std::ffi::{CStr, CString};

// Include the generated bindings from bindgen
include!(concat!(env!("OUT_DIR"), "/algorithm_ffi_bindings.rs"));

/// Convenient wrapper for algorithm execution results
pub struct AlgorithmOutputWrapper {
    pub success: bool,
    pub result_json: serde_json::Value,
    pub error_message: String,
    pub execution_time_ms: u64,
    pub memory_used_bytes: u64,
}

/// C++ Algorithm Executor Bridge
pub struct CppAlgorithmExecutorBridge {
    initialized: bool,
}

impl CppAlgorithmExecutorBridge {
    /// Create a new executor
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            initialized: false,
        })
    }

    /// Initialize the executor
    pub fn initialize(&mut self) -> Result<bool, String> {
        unsafe {
            let result = algorithm_executor_init();
            self.initialized = result == 0;
            Ok(self.initialized)
        }
    }

    /// Check if initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Execute an algorithm with the given input
    pub fn execute_algorithm(
        &self,
        algorithm_name: &str,
        parameters: &serde_json::Value,
        device_id: &str,
    ) -> Result<AlgorithmOutputWrapper, String> {
        if !self.initialized {
            return Err("CppAlgorithmExecutor not initialized".to_string());
        }

        // Create C strings from Rust types
        let algorithm_name_c = CString::new(algorithm_name)
            .map_err(|e| format!("Invalid algorithm name: {}", e))?;
        
        let parameters_json = serde_json::to_string(parameters)
            .unwrap_or_else(|_| "{}".to_string());
        let parameters_c = CString::new(parameters_json)
            .map_err(|e| format!("Invalid parameters JSON: {}", e))?;
        
        let device_id_c = CString::new(device_id)
            .map_err(|e| format!("Invalid device ID: {}", e))?;

        unsafe {
            // Create FFI input structure
            let ffi_input = AlgorithmInput {
                algorithm_name: algorithm_name_c.as_ptr() as *mut i8,
                parameters_json: parameters_c.as_ptr() as *mut i8,
                device_id: device_id_c.as_ptr() as *mut i8,
            };

            // Call C++ function
            let ffi_output = algorithm_executor_execute(&ffi_input);
            if ffi_output.is_null() {
                return Err("Failed to execute algorithm".to_string());
            }

            let output_ref = &*ffi_output;
            
            // Convert C strings back to Rust
            let result_json_str = CStr::from_ptr(output_ref.result_json)
                .to_string_lossy()
                .to_string();
            
            let error_message = CStr::from_ptr(output_ref.error_message)
                .to_string_lossy()
                .to_string();

            // Parse JSON result
            let result_value = serde_json::from_str(&result_json_str)
                .unwrap_or_else(|_| serde_json::json!({"raw_result": result_json_str}));

            let output = AlgorithmOutputWrapper {
                success: output_ref.success,
                result_json: result_value,
                error_message,
                execution_time_ms: output_ref.execution_time_ms,
                memory_used_bytes: output_ref.memory_used_bytes,
            };

            // Free C++ allocated memory
            algorithm_output_free(ffi_output as *mut _);

            Ok(output)
        }
    }

    /// Get list of available plugins
    pub fn get_available_plugins(&self) -> Result<Vec<String>, String> {
        unsafe {
            let plugins_ptr = algorithm_get_available_plugins();
            if plugins_ptr.is_null() {
                return Ok(vec![]);
            }

            let plugins_str = CStr::from_ptr(plugins_ptr)
                .to_string_lossy()
                .to_string();
            
            algorithm_free_string(plugins_ptr as *mut _);

            let plugins = plugins_str
                .split(',')
                .map(|s| s.to_string())
                .collect();
            
            Ok(plugins)
        }
    }

    /// Get plugin information
    pub fn get_plugin_info(&self, plugin_name: &str) -> Result<serde_json::Value, String> {
        let info = match plugin_name {
            "vibrate31" => serde_json::json!({
                "name": "vibrate31",
                "version": "1.0.0",
                "type": "FEATURE",
                "description": "vibration feature extraction plugin v31"
            }),
            "motor97" => serde_json::json!({
                "name": "motor97",
                "version": "1.0.0",
                "type": "DECISION",
                "description": "motor state recognition plugin"
            }),
            "current_feature_extractor" => serde_json::json!({
                "name": "current_feature_extractor",
                "version": "1.0.0",
                "type": "FEATURE",
                "description": "current feature extraction plugin"
            }),
            "temperature_feature_extractor" => serde_json::json!({
                "name": "temperature_feature_extractor",
                "version": "1.0.0",
                "type": "FEATURE",
                "description": "temperature feature extraction plugin"
            }),
            "audio_feature_extractor" => serde_json::json!({
                "name": "audio_feature_extractor",
                "version": "1.0.0",
                "type": "FEATURE",
                "description": "audio feature extraction plugin"
            }),
            "universal_classify1" => serde_json::json!({
                "name": "universal_classify1",
                "version": "1.0.0",
                "type": "DECISION",
                "description": "universal classifier plugin"
            }),
            "comp_realtime_health34" => serde_json::json!({
                "name": "comp_realtime_health34",
                "version": "1.0.0",
                "type": "EVALUATION",
                "description": "realtime health evaluation plugin"
            }),
            "error18" => serde_json::json!({
                "name": "error18",
                "version": "1.0.0",
                "type": "EVALUATION",
                "description": "error detection plugin"
            }),
            "score_alarm5" => serde_json::json!({
                "name": "score_alarm5",
                "version": "1.0.0",
                "type": "EVENT",
                "description": "score alarm plugin"
            }),
            "status_alarm4" => serde_json::json!({
                "name": "status_alarm4",
                "version": "1.0.0",
                "type": "EVENT",
                "description": "status alarm plugin"
            }),
            _ => serde_json::json!({
                "error": "Unknown plugin"
            }),
        };
        Ok(info)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_executor_initialization() {
        let mut executor = CppAlgorithmExecutorBridge::new().expect("Failed to create executor");
        assert!(!executor.is_initialized());
        
        let result = executor.initialize();
        assert!(result.is_ok());
        assert!(executor.is_initialized());
    }

    #[test]
    fn test_execute_algorithm_without_init() {
        let executor = CppAlgorithmExecutorBridge::new().expect("Failed to create executor");
        
        let result = executor.execute_algorithm("vibrate31", &json!({}), "test");
        assert!(result.is_err());
    }

    #[test]
    fn test_execute_algorithm_with_init() {
        let mut executor = CppAlgorithmExecutorBridge::new().expect("Failed to create executor");
        executor.initialize().expect("Failed to initialize");
        
        let result = executor.execute_algorithm("vibrate31", &json!({"test": "data"}), "test_device");
        
        assert!(result.is_ok());
        
        let output = result.unwrap();
        assert!(output.success);
    }

    #[test]
    fn test_get_available_plugins() {
        let executor = CppAlgorithmExecutorBridge::new().expect("Failed to create executor");
        let plugins = executor.get_available_plugins().expect("Failed to get plugins");
        
        assert!(plugins.contains(&"vibrate31".to_string()));
        assert!(plugins.contains(&"motor97".to_string()));
        assert!(plugins.len() > 0);
    }

    #[test]
    fn test_get_plugin_info() {
        let executor = CppAlgorithmExecutorBridge::new().expect("Failed to create executor");
        let info = executor.get_plugin_info("vibrate31").expect("Failed to get plugin info");
        
        assert_eq!(info["name"], "vibrate31");
        assert_eq!(info["type"], "FEATURE");
    }
}
