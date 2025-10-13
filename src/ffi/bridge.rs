//! CXX桥接实现 - 集成cpp_plugins架构

use serde_json::json;
use std::time::Instant;
use std::sync::Arc;
use std::collections::HashMap;

// 定义Result类型
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

// 重新导出核心类型
use super::super::core::{ComputeRequest, ComputeResponse};

// CXX桥接定义
#[cxx::bridge]
mod ffi {
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

    // 振动数据结构
    #[derive(Debug)]
    struct VibrationData {
        wave_data: Vec<f64>,
        speed_data: Vec<f64>,
        sampling_rate: i32,
        device_id: String,
    }

    // 振动特征结构
    #[derive(Debug)]
    struct VibrationFeatures {
        mean_hf: f64,
        mean_lf: f64,
        mean: f64,
        std_dev: f64,
        peak_freq: f64,
        peak_power: f64,
        spectrum_energy: f64,
        status: i32,
        load: f64,
    }

    // C++函数声明
    extern "C++" {
        include!("rust-edge-compute/src/ffi/cpp/bridge.h");

        // 类型映射
        type AlgorithmInput;
        type AlgorithmOutput;
        type VibrationData;

        // CppAlgorithmExecutor类
        type CppAlgorithmExecutor;

        // 构造函数和析构函数
        fn new_cpp_executor() -> UniquePtr<CppAlgorithmExecutor>;

        // 初始化方法
        fn initialize(self: &CppAlgorithmExecutor) -> bool;

        // 通用算法执行
        fn execute_algorithm(self: &CppAlgorithmExecutor, input: &AlgorithmInput) -> AlgorithmOutput;

        // vibrate31专用执行方法
        fn execute_vibrate31(self: &CppAlgorithmExecutor, vibration_data: &VibrationData, parameters: &CxxVector<CxxString>) -> AlgorithmOutput;

        // 插件管理
        fn get_available_plugins(self: &CppAlgorithmExecutor) -> Vec<String>;
        fn get_plugin_info(self: &CppAlgorithmExecutor, plugin_name: &str) -> String;
        fn load_plugin(self: &CppAlgorithmExecutor, plugin_name: &str) -> bool;
        fn unload_plugin(self: &CppAlgorithmExecutor, plugin_name: &str) -> bool;

        // 兼容性函数
        fn simple_math_add(a: f64, b: f64) -> AlgorithmOutput;
        fn simple_math_multiply(a: f64, b: f64) -> AlgorithmOutput;
        fn string_reverse(input: &str) -> AlgorithmOutput;
        fn data_sort_integers(input: &Vec<i32>) -> AlgorithmOutput;
    }
}

// 生产级API命名空间
#[cxx::bridge]
mod production_api {
    extern "C++" {
        include!("rust-edge-compute/src/ffi/cpp/bridge.h");

        // 插件状态结构
        #[derive(Debug)]
        struct PluginStatus {
            plugin_name: String,
            loaded: bool,
            initialized: bool,
            version: String,
            last_error: String,
            execution_count: u64,
            avg_execution_time_ms: f64,
        }

        // 系统状态结构
        #[derive(Debug)]
        struct SystemStatus {
            total_memory_bytes: u64,
            used_memory_bytes: u64,
            active_plugins: u32,
            total_plugins: u32,
            system_health: String,
        }

        // 性能指标结构
        #[derive(Debug)]
        struct PerformanceMetrics {
            cpu_usage_percent: f64,
            memory_usage_bytes: u64,
            active_threads: u32,
            uptime_seconds: u64,
        }

        // 生产级API函数
        fn get_plugin_status() -> Vec<PluginStatus>;
        fn get_system_status() -> SystemStatus;
        fn health_check() -> bool;
        fn get_performance_metrics() -> PerformanceMetrics;
    }
}

/// C++算法执行器 - 集成cpp_plugins架构
pub struct CppAlgorithmExecutor {
    executor: cxx::UniquePtr<ffi::CppAlgorithmExecutor>,
    memory_manager: Arc<super::MemoryManager>,
    initialized: bool,
}

impl CppAlgorithmExecutor {
    /// 创建新的执行器 - 集成cpp_plugins
    pub fn new() -> Result<Self> {
        // 创建C++执行器实例
        let executor = ffi::new_cpp_executor();

        // 创建内存管理器
        let memory_manager = Arc::new(super::MemoryManager::new());

        // 启动自动垃圾回收
        let gc_manager = Arc::clone(&memory_manager);
        tokio::spawn(async move {
            gc_manager.start_auto_gc().await;
        });

        Ok(Self {
            executor,
            memory_manager,
            initialized: false,
        })
    }

    /// 初始化执行器
    pub fn initialize(&mut self) -> Result<bool> {
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

    /// 执行算法 - 支持cpp_plugins架构
    pub async fn execute(&self, request: ComputeRequest) -> Result<ComputeResponse> {
        let start_time = Instant::now();

        // 检查是否已初始化
        if !self.initialized {
            return Err("CppAlgorithmExecutor not initialized".into());
        }

        // 分配内存用于输入数据
        let input_size = request.algorithm.len() + serde_json::to_string(&request.parameters)
            .unwrap_or_default()
            .len();
        let input_memory = self.memory_manager.allocate(input_size).await
            .unwrap_or(0);

        // 将请求转换为C++输入格式
        let input = self.create_algorithm_input(&request)?;

        // 调用C++算法
        let output = self.executor.execute_algorithm(&input);

        let execution_time = start_time.elapsed().as_millis() as u64;

        // 释放输入内存
        if input_memory > 0 {
            let _ = self.memory_manager.deallocate(input_memory).await;
        }

        // 处理C++输出结果
        if output.success {
            // 解析C++返回的JSON结果
            let result: serde_json::Value = serde_json::from_str(&output.result_json)
                .unwrap_or_else(|_| json!({"result": "parse_error"}));

            Ok(ComputeResponse::success(
                request.id,
                result,
                execution_time,
            ))
        } else {
            Ok(ComputeResponse::failure(
                request.id,
                format!("C++ algorithm error: {}", output.error_message),
            ))
        }
    }

    /// 通用插件执行方法 - 根据插件类型自动处理
    pub async fn execute_plugin(&self,
                               plugin_name: &str,
                               input_data: serde_json::Value,
                               parameters: HashMap<String, String>) -> Result<serde_json::Value> {
        // 检查是否已初始化
        if !self.initialized {
            return Err("CppAlgorithmExecutor not initialized".into());
        }

        // 根据插件类型自动处理数据转换
        match plugin_name {
            "vibrate31" => {
                self.execute_vibration_plugin(plugin_name, &input_data, parameters).await
            }
            "current_feature_extractor" | "temperature_feature_extractor" | "audio_feature_extractor" => {
                self.execute_feature_plugin(plugin_name, &input_data, parameters).await
            }
            "motor97" | "universal_classify1" => {
                self.execute_decision_plugin(plugin_name, &input_data, parameters).await
            }
            "comp_realtime_health34" | "error18" => {
                self.execute_evaluation_plugin(plugin_name, &input_data, parameters).await
            }
            "score_alarm5" | "status_alarm4" => {
                self.execute_event_plugin(plugin_name, &input_data, parameters).await
            }
            _ => {
                // 通用插件处理
                self.execute_generic_plugin(plugin_name, &input_data, parameters).await
            }
        }
    }

    /// 执行振动特征提取插件
    async fn execute_vibration_plugin(&self,
                                     plugin_name: &str,
                                     input_data: &serde_json::Value,
                                     parameters: HashMap<String, String>) -> Result<serde_json::Value> {
        // 提取振动数据
        let wave_data = input_data.get("wave_data")
            .and_then(|v| v.as_array())
            .ok_or("Missing wave_data parameter")?
            .iter()
            .map(|v| v.as_f64().unwrap_or(0.0))
            .collect::<Vec<f64>>();

        let speed_data = input_data.get("speed_data")
            .and_then(|v| v.as_array())
            .ok_or("Missing speed_data parameter")?
            .iter()
            .map(|v| v.as_f64().unwrap_or(0.0))
            .collect::<Vec<f64>>();

        let sampling_rate = input_data.get("sampling_rate")
            .and_then(|v| v.as_i64())
            .unwrap_or(1000) as i32;

        let device_id = input_data.get("device_id")
            .and_then(|v| v.as_str())
            .unwrap_or("default_device");

        // 创建振动数据结构
        let vibration_data = ffi::VibrationData {
            wave_data,
            speed_data,
            sampling_rate,
            device_id: device_id.to_string(),
        };

        // 将参数转换为C++格式
        let mut cxx_params = cxx::CxxVector::new();
        for (key, value) in parameters {
            cxx_params.push(format!("{}={}", key, value));
        }

        // 调用C++的振动插件执行方法
        let output = self.executor.execute_vibrate31(&vibration_data, &cxx_params);

        // 处理结果
        if output.success {
            let result: serde_json::Value = serde_json::from_str(&output.result_json)
                .unwrap_or_else(|_| json!({"result": "parse_error"}));
            Ok(result)
        } else {
            Err(format!("{} execution failed: {}", plugin_name, output.error_message).into())
        }
    }

    /// 执行通用特征提取插件
    async fn execute_feature_plugin(&self,
                                   plugin_name: &str,
                                   input_data: &serde_json::Value,
                                   parameters: HashMap<String, String>) -> Result<serde_json::Value> {
        // 构建算法输入
        let algorithm_input = self.build_algorithm_input(plugin_name, input_data, parameters)?;

        // 执行算法
        let output = self.executor.execute_algorithm(&algorithm_input);

        // 处理结果
        if output.success {
            let result: serde_json::Value = serde_json::from_str(&output.result_json)
                .unwrap_or_else(|_| json!({"result": "parse_error"}));
            Ok(result)
        } else {
            Err(format!("{} execution failed: {}", plugin_name, output.error_message).into())
        }
    }

    /// 执行状态识别插件
    async fn execute_decision_plugin(&self,
                                    plugin_name: &str,
                                    input_data: &serde_json::Value,
                                    parameters: HashMap<String, String>) -> Result<serde_json::Value> {
        // 构建算法输入
        let algorithm_input = self.build_algorithm_input(plugin_name, input_data, parameters)?;

        // 执行算法
        let output = self.executor.execute_algorithm(&algorithm_input);

        // 处理结果
        if output.success {
            let result: serde_json::Value = serde_json::from_str(&output.result_json)
                .unwrap_or_else(|_| json!({"result": "parse_error"}));
            Ok(result)
        } else {
            Err(format!("{} execution failed: {}", plugin_name, output.error_message).into())
        }
    }

    /// 执行健康评估插件
    async fn execute_evaluation_plugin(&self,
                                      plugin_name: &str,
                                      input_data: &serde_json::Value,
                                      parameters: HashMap<String, String>) -> Result<serde_json::Value> {
        // 构建算法输入
        let algorithm_input = self.build_algorithm_input(plugin_name, input_data, parameters)?;

        // 执行算法
        let output = self.executor.execute_algorithm(&algorithm_input);

        // 处理结果
        if output.success {
            let result: serde_json::Value = serde_json::from_str(&output.result_json)
                .unwrap_or_else(|_| json!({"result": "parse_error"}));
            Ok(result)
        } else {
            Err(format!("{} execution failed: {}", plugin_name, output.error_message).into())
        }
    }

    /// 执行事件处理插件
    async fn execute_event_plugin(&self,
                                 plugin_name: &str,
                                 input_data: &serde_json::Value,
                                 parameters: HashMap<String, String>) -> Result<serde_json::Value> {
        // 构建算法输入
        let algorithm_input = self.build_algorithm_input(plugin_name, input_data, parameters)?;

        // 执行算法
        let output = self.executor.execute_algorithm(&algorithm_input);

        // 处理结果
        if output.success {
            let result: serde_json::Value = serde_json::from_str(&output.result_json)
                .unwrap_or_else(|_| json!({"result": "parse_error"}));
            Ok(result)
        } else {
            Err(format!("{} execution failed: {}", plugin_name, output.error_message).into())
        }
    }

    /// 执行通用插件
    async fn execute_generic_plugin(&self,
                                   plugin_name: &str,
                                   input_data: &serde_json::Value,
                                   parameters: HashMap<String, String>) -> Result<serde_json::Value> {
        // 构建算法输入
        let algorithm_input = self.build_algorithm_input(plugin_name, input_data, parameters)?;

        // 执行算法
        let output = self.executor.execute_algorithm(&algorithm_input);

        // 处理结果
        if output.success {
            let result: serde_json::Value = serde_json::from_str(&output.result_json)
                .unwrap_or_else(|_| json!({"result": "parse_error"}));
            Ok(result)
        } else {
            Err(format!("{} execution failed: {}", plugin_name, output.error_message).into())
        }
    }

    /// 构建算法输入 - 通用方法
    fn build_algorithm_input(&self,
                            algorithm_name: &str,
                            input_data: &serde_json::Value,
                            parameters: HashMap<String, String>) -> Result<ffi::AlgorithmInput> {
        // 合并输入数据和参数
        let mut combined_data = input_data.clone();

        // 添加参数到数据中
        if let Some(obj) = combined_data.as_object_mut() {
            for (key, value) in &parameters {
                obj.insert(key.clone(), serde_json::Value::String(value.clone()));
            }
        }

        let parameters_json = serde_json::to_string(&combined_data)
            .map_err(|e| format!("Failed to serialize parameters: {}", e))?;

        // 从数据中提取设备ID
        let device_id = input_data.get("device_id")
            .and_then(|v| v.as_str())
            .unwrap_or("default_device")
            .to_string();

        // 获取当前时间戳
        let timestamp_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        Ok(ffi::AlgorithmInput {
            algorithm_name: algorithm_name.to_string(),
            parameters_json,
            device_id,
            timestamp_ms,
        })
    }

    /// 获取可用插件列表
    pub fn get_available_plugins(&self) -> Result<Vec<String>> {
        if !self.initialized {
            return Err("CppAlgorithmExecutor not initialized".into());
        }
        Ok(self.executor.get_available_plugins())
    }

    /// 获取插件信息
    pub fn get_plugin_info(&self, plugin_name: &str) -> Result<String> {
        if !self.initialized {
            return Err("CppAlgorithmExecutor not initialized".into());
        }
        Ok(self.executor.get_plugin_info(plugin_name))
    }

    /// 加载插件
    pub fn load_plugin(&self, plugin_name: &str) -> Result<bool> {
        if !self.initialized {
            return Err("CppAlgorithmExecutor not initialized".into());
        }
        Ok(self.executor.load_plugin(plugin_name))
    }

    /// 卸载插件
    pub fn unload_plugin(&self, plugin_name: &str) -> Result<bool> {
        if !self.initialized {
            return Err("CppAlgorithmExecutor not initialized".into());
        }
        Ok(self.executor.unload_plugin(plugin_name))
    }

    /// 创建算法输入 - 增强版支持cpp_plugins
    fn create_algorithm_input(&self, request: &ComputeRequest) -> Result<ffi::AlgorithmInput> {
        let parameters_json = serde_json::to_string(&request.parameters)
            .map_err(|e| format!("Failed to serialize parameters: {}", e))?;

        // 从参数中提取设备ID，如果没有则使用默认值
        let device_id = request.parameters.get("device_id")
            .and_then(|v| v.as_str())
            .unwrap_or("default_device")
            .to_string();

        // 获取当前时间戳
        let timestamp_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        Ok(ffi::AlgorithmInput {
            algorithm_name: request.algorithm.clone(),
            parameters_json,
            device_id,
            timestamp_ms,
        })
    }
}

/// 便捷的C++算法调用函数 - 支持cpp_plugins架构的通用实现
pub async fn execute_cpp_algorithm(name: &str, params: &serde_json::Value) -> Result<serde_json::Value> {
    // 创建执行器实例
    let mut executor = CppAlgorithmExecutor::new()?;

    // 初始化执行器
    if !executor.initialize()? {
        return Err("Failed to initialize C++ executor".into());
    }

    // 检查是否是cpp_plugins中的插件
    let available_plugins = executor.get_available_plugins()?;
    if available_plugins.contains(&name.to_string()) {
        // 使用通用插件执行方法
        let parameters = extract_parameters_from_json(params);
        return executor.execute_plugin(name, params.clone(), parameters).await;
    }

    // 处理内置简单算法（向后兼容）
    match name {
        "add" => {
            if let (Some(a), Some(b)) = (
                params.get("a").and_then(|v| v.as_f64()),
                params.get("b").and_then(|v| v.as_f64()),
            ) {
                let output = ffi::simple_math_add(a, b);
                if output.success {
                    Ok(serde_json::from_str(&output.result_json)?)
                } else {
                    Err(output.error_message.into())
                }
            } else {
                Err("Invalid parameters for add algorithm".into())
            }
        }
        "multiply" => {
            let a = params.get("a").and_then(|v| v.as_f64()).unwrap_or(1.0);
            let b = params.get("b").and_then(|v| v.as_f64()).unwrap_or(1.0);
            let output = ffi::simple_math_multiply(a, b);
            if output.success {
                Ok(serde_json::from_str(&output.result_json)?)
            } else {
                Err(output.error_message.into())
            }
        }
        "reverse" => {
            let text = params.get("text")
                .and_then(|v| v.as_str())
                .unwrap_or("hello");
            let output = ffi::string_reverse(text);
            if output.success {
                Ok(serde_json::from_str(&output.result_json)?)
            } else {
                Err(output.error_message.into())
            }
        }
        "sort" => {
            let data = vec![3, 1, 4, 1, 5];
            let output = ffi::data_sort_integers(&data);
            if output.success {
                Ok(serde_json::from_str(&output.result_json)?)
            } else {
                Err(output.error_message.into())
            }
        }
        _ => {
            // 尝试作为通用插件执行
            let parameters = extract_parameters_from_json(params);
            executor.execute_plugin(name, params.clone(), parameters).await
        }
    }
}

/// 从JSON参数中提取插件参数
fn extract_parameters_from_json(params: &serde_json::Value) -> HashMap<String, String> {
    let mut parameters = HashMap::new();

    if let Some(obj) = params.as_object() {
        for (key, value) in obj {
            // 只提取非数据字段的参数
            if !matches!(key.as_str(),
                "wave_data" | "speed_data" | "sampling_rate" |
                "device_id" | "timestamp" | "data" | "features") {
                match value {
                    serde_json::Value::String(s) => {
                        parameters.insert(key.clone(), s.clone());
                    }
                    serde_json::Value::Number(n) => {
                        if let Some(i) = n.as_i64() {
                            parameters.insert(key.clone(), i.to_string());
                        } else if let Some(f) = n.as_f64() {
                            parameters.insert(key.clone(), f.to_string());
                        }
                    }
                    serde_json::Value::Bool(b) => {
                        parameters.insert(key.clone(), b.to_string());
                    }
                    _ => {}
                }
            }
        }
    }

    parameters
}

/// 创建插件管理器实例
pub fn create_plugin_manager() -> Result<CppAlgorithmExecutor> {
    let mut executor = CppAlgorithmExecutor::new()?;
    executor.initialize()?;
    Ok(executor)
}

/// 获取所有可用插件信息
pub fn get_plugin_registry() -> Result<serde_json::Value> {
    let executor = create_plugin_manager()?;
    let plugins = executor.get_available_plugins()?;

    let mut plugin_info = serde_json::Map::new();

    for plugin_name in plugins {
        if let Ok(info) = executor.get_plugin_info(&plugin_name) {
            if let Ok(info_json) = serde_json::from_str::<serde_json::Value>(&info) {
                plugin_info.insert(plugin_name, info_json);
            }
        }
    }

    Ok(serde_json::Value::Object(plugin_info))
}

/// C++算法接口
pub trait CppAlgorithm {
    /// 执行算法
    fn execute(&self, name: &str, params: &str) -> Result<String>;
}

/// 默认C++算法实现
pub struct DefaultCppAlgorithm;

impl CppAlgorithm for DefaultCppAlgorithm {
    fn execute(&self, name: &str, params: &str) -> Result<String> {
        // 这里可以调用实际的C++算法
        tracing::info!("Executing C++ algorithm: {} with params: {}", name, params);
        Ok(format!("Algorithm {} executed with params: {}", name, params))
    }
}
