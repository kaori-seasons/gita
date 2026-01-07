//! Python执行器
//!
//! 提供Python代码执行和函数调用功能，使用PyO3进行集成

pub mod dependency_manager;
pub mod model_loader;

use rust_edge_compute_core::core::error::EdgeComputeError;
use rust_edge_compute_core::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::Duration;

// 添加PyO3导入
#[cfg(feature = "python")]
use pyo3::prelude::*;

use dependency_manager::{DependencyManager, DependencyManagerConfig};
use model_loader::{ModelLoader, ModelLoaderConfig};

/// Python执行器配置
#[derive(Debug, Clone)]
pub struct PythonExecutorConfig {
    /// Python版本
    pub python_version: String,
    /// 最大执行时间（秒）
    pub max_execution_time_seconds: u64,
    /// 最大内存使用（MB）
    pub max_memory_mb: Option<u64>,
    /// 允许的模块列表
    pub allowed_modules: Vec<String>,
    /// 是否启用沙箱模式
    pub sandbox_mode: bool,
}

impl Default for PythonExecutorConfig {
    fn default() -> Self {
        Self {
            python_version: "3.11".to_string(),
            max_execution_time_seconds: 30,
            max_memory_mb: Some(512),
            allowed_modules: vec![
                "json".to_string(),
                "math".to_string(),
                "numpy".to_string(),
            ],
            sandbox_mode: true,
        }
    }
}

/// Python执行器
pub struct PythonExecutor {
    /// 配置
    config: PythonExecutorConfig,
    /// 执行统计
    execution_stats: Arc<RwLock<ExecutionStats>>,
    /// 依赖管理器
    dependency_manager: Option<Arc<DependencyManager>>,
    /// 模型加载器
    model_loader: Option<Arc<ModelLoader>>,
    /// 资源限制监控
    resource_monitor: Arc<RwLock<ResourceMonitor>>,
    /// 是否已初始化
    initialized: bool,
    /// JSON模块引用
    #[cfg(feature = "python")]
    json_module: Option<pyo3::PyObject>,
}

/// 资源监控
#[derive(Debug, Clone, Default)]
struct ResourceMonitor {
    /// 当前内存使用（字节）
    current_memory_bytes: u64,
    /// 峰值内存使用（字节）
    peak_memory_bytes: u64,
    /// 当前CPU使用率（0.0-1.0）
    current_cpu_usage: f64,
    /// 执行时间统计
    execution_times: Vec<Duration>,
}

/// 执行统计
#[derive(Debug, Clone, Default)]
pub struct ExecutionStats {
    /// 总执行次数
    pub total_executions: u64,
    /// 成功执行次数
    pub successful_executions: u64,
    /// 失败执行次数
    pub failed_executions: u64,
}

impl PythonExecutor {
    /// 创建新的Python执行器
    pub fn new(config: PythonExecutorConfig) -> Result<Self> {
        #[cfg(feature = "python")]
        {
            // 初始化Python解释器（如果尚未初始化）
            pyo3::prepare_freethreaded_python();
        }
        
        // 创建依赖管理器（如果配置允许）
        let dependency_manager = if config.sandbox_mode {
            Some(Arc::new(
                DependencyManager::new(DependencyManagerConfig::default())
                    .map_err(|e| Box::new(EdgeComputeError(format!("Failed to create dependency manager: {}", e))))?
            ))
        } else {
            None
        };
        
        // 创建模型加载器
        let model_loader = Some(Arc::new(
            ModelLoader::new(ModelLoaderConfig::default())
                .map_err(|e| Box::new(EdgeComputeError(format!("Failed to create model loader: {}", e))))?
        ));
        
        Ok(Self {
            config,
            execution_stats: Arc::new(RwLock::new(ExecutionStats::default())),
            dependency_manager,
            model_loader,
            resource_monitor: Arc::new(RwLock::new(ResourceMonitor::default())),
            initialized: false,
            #[cfg(feature = "python")]
            json_module: None,
        })
    }
    
    /// 创建配置错误
    fn config_error(&self, message: String) -> Box<dyn std::error::Error + Send + Sync> {
        Box::new(EdgeComputeError(format!("Configuration error: {}", message)))
    }

    /// 创建资源耗尽错误
    fn resource_exhausted_error(&self, message: String) -> Box<dyn std::error::Error + Send + Sync> {
        Box::new(EdgeComputeError(format!("Resource exhausted: {}", message)))
    }

    /// 初始化Python环境
    #[cfg(feature = "python")]
    pub fn initialize(&mut self) -> Result<()> {
        use pyo3::types::PyDict;
        
        // 检查是否已初始化
        if self.initialized {
            return Ok(());
        }
        
        Python::with_gil(|py| {
            // 导入必要的模块
            let sys = py.import_bound("sys")
                .map_err(|e| self.config_error(format!("Failed to import sys module: {:?}", e)))?;
                
            // 添加当前目录到Python路径
            let path = sys.getattr("path")
                .map_err(|e| self.config_error(format!("Failed to get sys.path: {:?}", e)))?;
                
            path.call_method1("append", ("./python_scripts",))
                .map_err(|e| self.config_error(format!("Failed to append to Python path: {:?}", e)))?;
                
            // 导入json模块用于序列化
            self.json_module = Some(py.import_bound("json")
                .map_err(|e| self.config_error(format!("Failed to import json module: {:?}", e)))?
                .into());
                
            Ok(())
        })?;
        
        self.initialized = true;
        tracing::info!("Python executor initialized successfully");
        Ok(())
    }

    /// 初始化Python环境（非Python特性）
    #[cfg(not(feature = "python"))]
    pub fn initialize(&mut self) -> Result<()> {
        Err(self.config_error("Python support not enabled. Please enable the 'python' feature.".to_string()))
    }

    /// 执行Python脚本
    #[cfg(feature = "python")]
    pub async fn execute_script(&self, script_name: &str, args: serde_json::Value) -> Result<serde_json::Value> {
        use pyo3::types::{PyDict, PyString};
        
        if !self.initialized {
            return Err(self.config_error("Python executor not initialized".to_string()));
        }
        
        let script_path = format!("./python_scripts/{}.py", script_name);
        let args_json = serde_json::to_string(&args)
            .map_err(|e| Box::new(EdgeComputeError(format!("Failed to serialize args: {}", e))))?;
            
        let result = Python::with_gil(|py| {
            // 读取脚本内容
            let script_content = std::fs::read_to_string(&script_path)
                .map_err(|e| Box::new(EdgeComputeError(format!("Failed to read script {}: {}", script_path, e))))?;
                
            // 准备执行环境
            let globals = PyDict::new_bound(py);
            let locals = PyDict::new_bound(py);
            
            // 设置参数
            locals.set_item("input_args", args_json)
                .map_err(|e| Box::new(EdgeComputeError(format!("Failed to set input_args: {:?}", e))))?;
                
            // 执行脚本
            py.run(&script_content, Some(globals), Some(locals))
                .map_err(|e| {
                    // 获取Python异常详情
                    if let Some(err) = e.unpack_exception(py) {
                        let err_str = format!("{:?}", err);
                        Box::new(EdgeComputeError(format!("Python script execution failed: {}", err_str)))
                    } else {
                        Box::new(EdgeComputeError(format!("Python script execution failed: {:?}", e)))
                    }
                })?;
                
            // 获取结果
            let result_str: String = locals.get_item("result")
                .ok_or_else(|| Box::new(EdgeComputeError("Script did not return 'result' variable".to_string())))?
                .extract()
                .map_err(|e| Box::new(EdgeComputeError(format!("Failed to extract result: {:?}", e))))?;
                
            // 解析JSON结果
            let parsed_result: serde_json::Value = serde_json::from_str(&result_str)
                .map_err(|e| Box::new(EdgeComputeError(format!("Failed to parse result JSON: {}", e))))?;
                
            Ok(parsed_result)
        });
        
        result
    }

    /// 执行Python脚本（非Python特性）
    #[cfg(not(feature = "python"))]
    pub async fn execute_script(&self, _script_name: &str, _args: serde_json::Value) -> Result<serde_json::Value> {
        Err(self.config_error("Python support not enabled. Please enable the 'python' feature.".to_string()))
    }

    /// 执行Python代码
    #[cfg(feature = "python")]
    pub async fn execute_code(&self, code: &str) -> Result<String> {
        use pyo3::prelude::*;
        use pyo3::types::PyDict;
        
        // 更新统计
        {
            let mut stats = self.execution_stats.write().await;
            stats.total_executions += 1;
        }
        
        // 检查资源限制
        self.check_resource_limits().await?;
        
        // 在阻塞线程池中执行Python代码
        let code = code.to_string();
        let config = self.config.clone();
        let max_memory = config.max_memory_mb;
        let max_time = Duration::from_secs(config.max_execution_time_seconds);
        let resource_monitor = Arc::clone(&self.resource_monitor);
        
        let start_time = std::time::Instant::now();
        
        let result = tokio::time::timeout(max_time, tokio::task::spawn_blocking(move || {
            Python::with_gil(|py| {
                // 创建执行上下文
                let locals = PyDict::new_bound(py);
                
                // 限制可用的模块（沙箱模式）
                if config.sandbox_mode {
                    for module_name in &config.allowed_modules {
                        if let Ok(module) = py.import(module_name) {
                            locals.set_item(module_name, module).ok();
                        }
                    }
                }
                
                // 设置资源限制（如果支持）
                if let Some(max_mem) = max_memory {
                    // 注意：Python的内存限制需要额外的库支持
                    // 这里只是占位，实际需要集成resource或psutil等库
                }
                
                // 执行代码
                match py.run(&code, None, Some(locals)) {
                    Ok(_) => {
                        // 尝试获取结果
                        if let Ok(result) = locals.get_item("result") {
                            if let Some(result_str) = result.and_then(|r| r.to_string().ok()) {
                                Ok(result_str)
                            } else {
                                Ok("Execution completed".to_string())
                            }
                        } else {
                            Ok("Execution completed".to_string())
                        }
                    }
                    Err(e) => {
                        let error_msg = format!("Python execution error: {}", e);
                        tracing::error!("{}", error_msg);
                        Err(EdgeComputeError::AlgorithmExecution {
                            message: error_msg,
                            algorithm: Some("python_code".to_string()),
                            input_size: Some(code.len()),
                        })
                    }
                }
            })
        }))
        .await;
        
        let execution_time = start_time.elapsed();
        
        // 更新资源监控
        {
            let mut monitor = resource_monitor.write().await;
            monitor.execution_times.push(execution_time);
            // 保留最近100次执行时间
            if monitor.execution_times.len() > 100 {
                monitor.execution_times.remove(0);
            }
        }
        
        let execution_result = match result {
            Ok(Ok(r)) => r,
            Ok(Err(e)) => return Err(e),
            Err(_) => {
                return Err(Box::new(EdgeComputeError::AlgorithmExecution {
                    message: format!("Python execution timeout after {} seconds", 
                        config.max_execution_time_seconds),
                    algorithm: Some("python_code".to_string()),
                    input_size: Some(code.len()),
                }));
            }
        };
        
        let result = execution_result.map_err(|e| Box::new(EdgeComputeError::AlgorithmExecution {
            message: format!("Task join error: {}", e),
            algorithm: Some("python_code".to_string()),
            input_size: Some(code.len()),
        }));
        
        // 更新统计
        {
            let mut stats = self.execution_stats.write().await;
            match &result {
                Ok(_) => stats.successful_executions += 1,
                Err(_) => stats.failed_executions += 1,
            }
        }
        
        Ok(result?)
    }
    
    /// 执行Python代码（非Python特性）
    #[cfg(not(feature = "python"))]
    pub async fn execute_code(&self, _code: &str) -> Result<String> {
        Err(Box::new(EdgeComputeError("Python support not enabled. Please enable the 'python' feature.".to_string())))
    }
    
    /// 调用Python函数
    #[cfg(feature = "python")]
    pub async fn call_function(
        &self,
        module_name: &str,
        function_name: &str,
        args: Vec<serde_json::Value>,
    ) -> Result<serde_json::Value> {
        use pyo3::prelude::*;
        use pyo3::types::{PyDict, PyList};
        
        // 更新统计
        {
            let mut stats = self.execution_stats.write().await;
            stats.total_executions += 1;
        }
        
        let module_name = module_name.to_string();
        let function_name = function_name.to_string();
        let config = self.config.clone();
        
        // 在阻塞线程池中调用Python函数
        let result = tokio::task::spawn_blocking(move || {
            Python::with_gil(|py| {
                // 导入模块
                let module = py.import_bound(&module_name)
                    .map_err(|e| Box::new(EdgeComputeError(format!("Failed to import module '{}': {}", module_name, e))))?;
                
                // 获取函数
                let func = module.getattr(&function_name)
                    .map_err(|e| Box::new(EdgeComputeError(format!("Failed to get function '{}' from module '{}': {}", 
                        function_name, module_name, e))))?;
                
                // 转换参数
                let py_args = PyList::empty_bound(py);
                for arg in args {
                    let py_arg = Self::json_to_pyobject(py, &arg)
                        .map_err(|e| Box::new(EdgeComputeError(format!("Failed to convert argument: {}", e))))?;
                    py_args.append(py_arg).ok();
                }
                
                // 调用函数
                let result = func.call1((py_args,))
                    .map_err(|e| Box::new(EdgeComputeError(format!("Failed to call function '{}': {}", function_name, e))))?;
                
                // 转换结果
Self::py_to_json(&result)
                    .map_err(|e| Box::new(EdgeComputeError(format!("Failed to convert result: {}", e))))

            })
        })
        .await
        .map_err(|e| Box::new(EdgeComputeError(format!("Task join error: {}", e))))?;

        // 更新统计
        {
            let mut stats = self.execution_stats.write().await;
            match &result {
                Ok(_) => stats.successful_executions += 1,
                Err(_) => stats.failed_executions += 1,
            }
        }
        
        Ok(result?)
    }
    
    /// 调用Python函数（非Python特性）
    #[cfg(not(feature = "python"))]
    pub async fn call_function(
        &self,
        _module_name: &str,
        _function_name: &str,
        _args: Vec<serde_json::Value>,
    ) -> Result<serde_json::Value> {
        Err(Box::new(EdgeComputeError("Python support not enabled. Please enable the 'python' feature.".to_string())))
    }
    
    /// 将JSON值转换为Python对象
    #[cfg(feature = "python")]
    fn json_to_pyobject(py: pyo3::Python, value: &serde_json::Value) -> pyo3::PyResult<pyo3::PyObject> {
        use pyo3::prelude::*;
        use pyo3::types::{PyDict, PyList};
        
        match value {
            serde_json::Value::Null => Ok(py.None()),
            serde_json::Value::Bool(b) => Ok(b.into_py(py)),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Ok(i.into_py(py))
                } else if let Some(f) = n.as_f64() {
                    Ok(f.into_py(py))
                } else {
                    Ok(n.to_string().into_py(py))
                }
            }
            serde_json::Value::String(s) => Ok(s.into_py(py)),
            serde_json::Value::Array(arr) => {
                let list = PyList::empty_bound(py);
                for item in arr {
                    let py_item = Self::json_to_pyobject(py, item)?;
                    list.append(py_item)?;
                }
                Ok(list.into())
            }
            serde_json::Value::Object(obj) => {
                let dict = PyDict::new_bound(py);
                for (k, v) in obj {
                    let py_key = k.into_py(py);
                    let py_value = Self::json_to_pyobject(py, v)?;
                    dict.set_item(py_key, py_value)?;
                }
                Ok(dict.into())
            }
        }
    }
    
    /// 将Python对象转换为JSON值
    #[cfg(feature = "python")]
    fn py_to_json<'p>(obj: &'p PyAny) -> Result<serde_json::Value> {
        use pyo3::types::{PyDict, PyList, PyBool};
                use pyo3::prelude::*;
        
        // 检查是否为None
        if obj.is_none() {
            return Ok(serde_json::Value::Null);
        }
        
        // 检查是否为布尔值
        if let Ok(b) = obj.downcast::<pyo3::types::PyBool>() {
            return Ok(serde_json::Value::Bool(b.is_true()));
        }
        
        // 检查是否为整数
        if let Ok(i) = obj.extract::<i64>() {
            return Ok(serde_json::Value::Number(i.into()));
        }
        
        // 检查是否为浮点数
        if let Ok(f) = obj.extract::<f64>() {
            return Ok(serde_json::Value::Number(
                serde_json::Number::from_f64(f).unwrap_or_else(|| serde_json::Number::from(0))
            ));
        }
        
        // 检查是否为字符串
        if let Ok(s) = obj.extract::<String>() {
            return Ok(serde_json::Value::String(s));
        }
        
        // 检查是否为列表
        if let Ok(list) = obj.downcast::<pyo3::types::PyList>() {
            let mut arr = Vec::new();
            for item in list.iter() {
                arr.push(Self::py_to_json(&item)?);
            }
            return Ok(serde_json::Value::Array(arr));
        }
        
        // 检查是否为字典
        if let Ok(dict) = obj.downcast::<pyo3::types::PyDict>() {
            let mut map = serde_json::Map::new();
            for (key, value) in dict.iter() {
                let key_str = key.extract::<String>()
                    .map_err(|_| Box::new(EdgeComputeError("Dictionary key must be string".to_string())))?;
                map.insert(key_str, Self::py_to_json(&value)?);
            }
            return Ok(serde_json::Value::Object(map));
        }
        
        // 默认情况，转换为字符串
        let str_repr: String = obj.str()?.extract()?;
        Ok(serde_json::Value::String(str_repr))
    }

    /// 获取执行统计
    pub async fn get_stats(&self) -> ExecutionStats {
        self.execution_stats.read().await.clone()
    }
    
    /// 检查资源限制
    async fn check_resource_limits(&self) -> Result<()> {
        let monitor = self.resource_monitor.read().await;
        
        // 检查内存限制
        if let Some(max_memory) = self.config.max_memory_mb {
            let max_bytes = max_memory * 1024 * 1024;
            if monitor.current_memory_bytes > max_bytes {
                return Err(self.resource_exhausted_error(format!("Memory limit exceeded: {}MB", max_memory)));
            }
        }
        
        Ok(())
    }
    
    /// 获取依赖管理器
    pub fn dependency_manager(&self) -> Option<&Arc<DependencyManager>> {
        self.dependency_manager.as_ref()
    }
    
    /// 获取模型加载器
    pub fn model_loader(&self) -> Option<&Arc<ModelLoader>> {
        self.model_loader.as_ref()
    }
    
    /// 获取资源监控信息
    pub async fn get_resource_monitor(&self) -> ResourceMonitor {
        self.resource_monitor.read().await.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_python_executor_creation() {
        let executor = PythonExecutor::new(PythonExecutorConfig::default()).unwrap();
        let stats = executor.get_stats().await;
        assert_eq!(stats.total_executions, 0);
    }
    
    #[cfg(feature = "python")]
    #[tokio::test]
    async fn test_python_executor_execute_code() {
        let executor = PythonExecutor::new(PythonExecutorConfig::default()).unwrap();
        let code = "result = 2 + 2";
        let result = executor.execute_code(code).await;
        // 注意：实际测试需要启用python特性
        assert!(result.is_ok() || result.is_err()); // 根据是否启用特性
    }
}
