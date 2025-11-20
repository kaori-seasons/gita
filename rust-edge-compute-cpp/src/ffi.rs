//! C++ FFI桥接
//!
//! 提供Rust与C++的安全互操作接口

use cxx::UniquePtr;
use serde_json;
use std::collections::HashMap;

// CXX桥接定义
#[cxx::bridge]
pub mod ffi {
    // 共享结构体定义
    #[derive(Debug)]
    struct AlgorithmInput {
        algorithm_name: String,
        parameters_json: String,
        device_id: String,
        timestamp_ms: u64,
    }

    #[derive(Debug)]
    struct AlgorithmOutput {
        success: bool,
        result_json: String,
        error_message: String,
        execution_time_ms: u64,
        memory_used_bytes: u64,
    }

    // C++函数声明
    unsafe extern "C++" {
        include!("src/ffi/cpp_bridge.h");

        // CppAlgorithmExecutor类
        type CppAlgorithmExecutor;

        // 构造函数
        fn new_cpp_executor() -> UniquePtr<CppAlgorithmExecutor>;

        // 初始化方法
        fn initialize(self: &CppAlgorithmExecutor) -> bool;

        // 通用算法执行
        fn execute_algorithm(
            self: &CppAlgorithmExecutor,
            input: &AlgorithmInput,
        ) -> AlgorithmOutput;

        // 获取可用插件列表
        fn get_available_plugins(self: &CppAlgorithmExecutor) -> Vec<String>;

        // 获取插件信息
        fn get_plugin_info(
            self: &CppAlgorithmExecutor,
            plugin_name: &str,
        ) -> String;

        // 简单数学函数（向后兼容）
        fn simple_math_add(a: f64, b: f64) -> AlgorithmOutput;
        fn simple_math_multiply(a: f64, b: f64) -> AlgorithmOutput;
    }
}

/// C++算法执行器包装
pub struct CppAlgorithmExecutorBridge {
    executor: UniquePtr<ffi::CppAlgorithmExecutor>,
    initialized: bool,
}

impl CppAlgorithmExecutorBridge {
    /// 创建新的执行器
    pub fn new() -> Result<Self, String> {
        let executor = ffi::new_cpp_executor();
        Ok(Self {
            executor,
            initialized: false,
        })
    }

    /// 初始化执行器
    pub fn initialize(&mut self) -> Result<bool, String> {
        if !self.initialized {
            let result = self.executor.initialize();
            self.initialized = result;
            Ok(result)
        } else {
            Ok(true)
        }
    }

    /// 检查是否已初始化
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// 执行算法
    pub fn execute_algorithm(
        &self,
        algorithm_name: &str,
        parameters: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        if !self.initialized {
            return Err("CppAlgorithmExecutor not initialized".to_string());
        }

        // 创建输入
        let input = ffi::AlgorithmInput {
            algorithm_name: algorithm_name.to_string(),
            parameters_json: serde_json::to_string(parameters)
                .map_err(|e| format!("Failed to serialize parameters: {}", e))?,
            device_id: "default".to_string(),
            timestamp_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        };

        // 执行算法
        let output = self.executor.execute_algorithm(&input);

        if output.success {
            serde_json::from_str(&output.result_json)
                .map_err(|e| format!("Failed to parse result: {}", e))
        } else {
            Err(output.error_message)
        }
    }

    /// 获取可用插件列表
    pub fn get_available_plugins(&self) -> Result<Vec<String>, String> {
        if !self.initialized {
            return Err("CppAlgorithmExecutor not initialized".to_string());
        }
        Ok(self.executor.get_available_plugins())
    }

    /// 获取插件信息
    pub fn get_plugin_info(&self, plugin_name: &str) -> Result<String, String> {
        if !self.initialized {
            return Err("CppAlgorithmExecutor not initialized".to_string());
        }
        Ok(self.executor.get_plugin_info(plugin_name))
    }
}

/// 简单数学函数（向后兼容）
pub fn simple_math_add(a: f64, b: f64) -> Result<serde_json::Value, String> {
    let output = ffi::simple_math_add(a, b);
    if output.success {
        serde_json::from_str(&output.result_json)
            .map_err(|e| format!("Failed to parse result: {}", e))
    } else {
        Err(output.error_message)
    }
}

pub fn simple_math_multiply(a: f64, b: f64) -> Result<serde_json::Value, String> {
    let output = ffi::simple_math_multiply(a, b);
    if output.success {
        serde_json::from_str(&output.result_json)
            .map_err(|e| format!("Failed to parse result: {}", e))
    } else {
        Err(output.error_message)
    }
}

