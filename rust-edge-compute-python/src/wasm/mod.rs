//! WASM沙箱
//!
//! 提供安全的WASM执行环境，使用Wasmtime进行集成

use rust_edge_compute_core::core::error::{Result, EdgeComputeError};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::Duration;

/// WASM沙箱配置
#[derive(Debug, Clone)]
pub struct WasmSandboxConfig {
    /// 最大执行时间（毫秒）
    pub max_execution_time_ms: u64,
    /// 最大内存使用（MB）
    pub max_memory_mb: u64,
    /// 是否启用WASI
    pub enable_wasi: bool,
    /// 是否启用多线程
    pub enable_threads: bool,
}

impl Default for WasmSandboxConfig {
    fn default() -> Self {
        Self {
            max_execution_time_ms: 30000,
            max_memory_mb: 256,
            enable_wasi: true,
            enable_threads: false,
        }
    }
}

/// WASM沙箱
pub struct WasmSandbox {
    /// 配置
    config: WasmSandboxConfig,
    /// 执行统计
    execution_stats: Arc<RwLock<WasmExecutionStats>>,
    /// Wasmtime引擎（缓存）
    #[cfg(feature = "wasm")]
    engine: Arc<wasmtime::Engine>,
}

/// WASM执行统计
#[derive(Debug, Clone, Default)]
pub struct WasmExecutionStats {
    /// 总执行次数
    pub total_executions: u64,
    /// 成功执行次数
    pub successful_executions: u64,
    /// 失败执行次数
    pub failed_executions: u64,
}

impl WasmSandbox {
    /// 创建新的WASM沙箱
    #[cfg(feature = "wasm")]
    pub fn new(config: WasmSandboxConfig) -> Result<Self> {
        use wasmtime::*;
        
        // 创建引擎配置
        let mut engine_config = Config::default();
        engine_config.wasm_multi_memory(true);
        engine_config.wasm_memory64(false);
        
        if config.enable_threads {
            engine_config.wasm_threads(true);
        }
        
        // 创建引擎
        let engine = Engine::new(&engine_config)
            .map_err(|e| EdgeComputeError::Config {
                message: format!("Failed to create Wasmtime engine: {}", e),
                source: Some("wasm-sandbox".to_string()),
            })?;
        
        Ok(Self {
            config,
            execution_stats: Arc::new(RwLock::new(WasmExecutionStats::default())),
            engine: Arc::new(engine),
        })
    }
    
    /// 创建新的WASM沙箱（非WASM特性）
    #[cfg(not(feature = "wasm"))]
    pub fn new(config: WasmSandboxConfig) -> Result<Self> {
        Ok(Self {
            config,
            execution_stats: Arc::new(RwLock::new(WasmExecutionStats::default())),
        })
    }
    
    /// 执行WASM模块
    #[cfg(feature = "wasm")]
    pub async fn execute_module(
        &self,
        wasm_bytes: &[u8],
        function_name: &str,
        args: Vec<serde_json::Value>,
    ) -> Result<serde_json::Value> {
        use wasmtime::*;
        use wasmtime_wasi::*;
        
        // 更新统计
        {
            let mut stats = self.execution_stats.write().await;
            stats.total_executions += 1;
        }
        
        let function_name = function_name.to_string();
        let config = self.config.clone();
        let engine = Arc::clone(&self.engine);
        let wasm_bytes = wasm_bytes.to_vec();
        
        // 在阻塞线程池中执行WASM模块
        let result = tokio::task::spawn_blocking(move || {
            // 编译模块
            let module = Module::new(&engine, &wasm_bytes)
                .map_err(|e| EdgeComputeError::AlgorithmExecution {
                    message: format!("Failed to compile WASM module: {}", e),
                    algorithm: Some(function_name.clone()),
                    input_size: Some(wasm_bytes.len()),
                })?;
            
            // 创建存储
            let mut store = Store::new(&engine, ());
            
            // 配置WASI（如果启用）
            let mut linker = Linker::new(&engine);
            if config.enable_wasi {
                // 创建WASI上下文，支持更多功能
                let mut wasi_builder = WasiCtxBuilder::new();
                
                // 继承标准输入输出
                wasi_builder.inherit_stdio();
                
                // 设置环境变量（可选）
                // wasi_builder.env("KEY", "VALUE");
                
                // 设置工作目录（可选）
                // wasi_builder.preopened_dir(...);
                
                // 设置参数（可选）
                // wasi_builder.args(&["arg1", "arg2"]);
                
                let wasi_ctx = wasi_builder.build();
                let wasi = Wasi::new(&mut store, wasi_ctx);
                
                // 添加WASI到linker
                wasi.add_to_linker(&mut linker)
                    .map_err(|e| EdgeComputeError::AlgorithmExecution {
                        message: format!("Failed to add WASI to linker: {}", e),
                        algorithm: Some(function_name.clone()),
                        input_size: None,
                    })?;
                
                // 添加WASI预览1（如果可用）
                #[cfg(feature = "wasmtime-wasi")]
                {
                    use wasmtime_wasi::preview1::*;
                    // 注意：preview1需要不同的API，这里只是占位
                    // 实际使用时需要根据wasmtime-wasi版本调整
                }
            }
            
            // 实例化模块
            let instance = linker
                .instantiate(&mut store, &module)
                .map_err(|e| EdgeComputeError::AlgorithmExecution {
                    message: format!("Failed to instantiate WASM module: {}", e),
                    algorithm: Some(function_name.clone()),
                    input_size: None,
                })?;
            
            // 获取函数并尝试多种签名
            let result = Self::call_wasm_function_with_args(
                &mut store,
                &instance,
                &function_name,
                &args,
            );
            
            result
        })
        .await
        .map_err(|e| EdgeComputeError::AlgorithmExecution {
            message: format!("Task join error: {}", e),
            algorithm: Some(function_name),
            input_size: None,
        })?;
        
        // 更新统计
        {
            let mut stats = self.execution_stats.write().await;
            match &result {
                Ok(_) => stats.successful_executions += 1,
                Err(_) => stats.failed_executions += 1,
            }
        }
        
        result
    }
    
    /// 执行WASM模块（非WASM特性）
    #[cfg(not(feature = "wasm"))]
    pub async fn execute_module(
        &self,
        _wasm_bytes: &[u8],
        _function_name: &str,
        _args: Vec<serde_json::Value>,
    ) -> Result<serde_json::Value> {
        Err(EdgeComputeError::Config {
            message: "WASM support not enabled. Please enable the 'wasm' feature.".to_string(),
            source: Some("python-wasm".to_string()),
        })
    }
    
    /// 获取执行统计
    pub async fn get_stats(&self) -> WasmExecutionStats {
        self.execution_stats.read().await.clone()
    }
    
    /// 调用WASM函数（支持多种参数类型）
    #[cfg(feature = "wasm")]
    fn call_wasm_function_with_args(
        store: &mut wasmtime::Store<()>,
        instance: &wasmtime::Instance,
        function_name: &str,
        args: &[serde_json::Value],
    ) -> Result<serde_json::Value> {
        use wasmtime::*;
        
        // 尝试不同的函数签名
        // 1. 无参数，返回i32
        if let Ok(func) = instance.get_typed_func::<(), i32>(store, function_name) {
            let result = func.call(store, ())
                .map_err(|e| EdgeComputeError::AlgorithmExecution {
                    message: format!("Failed to call function: {}", e),
                    algorithm: Some(function_name.to_string()),
                    input_size: None,
                })?;
            return Ok(serde_json::json!({ "result": result }));
        }
        
        // 2. 无参数，返回f64
        if let Ok(func) = instance.get_typed_func::<(), f64>(store, function_name) {
            let result = func.call(store, ())
                .map_err(|e| EdgeComputeError::AlgorithmExecution {
                    message: format!("Failed to call function: {}", e),
                    algorithm: Some(function_name.to_string()),
                    input_size: None,
                })?;
            return Ok(serde_json::json!({ "result": result }));
        }
        
        // 3. 无参数，无返回值
        if let Ok(func) = instance.get_typed_func::<(), ()>(store, function_name) {
            func.call(store, ())
                .map_err(|e| EdgeComputeError::AlgorithmExecution {
                    message: format!("Failed to call function: {}", e),
                    algorithm: Some(function_name.to_string()),
                    input_size: None,
                })?;
            return Ok(serde_json::json!({ "result": "success" }));
        }
        
        // 4. 单个i32参数，返回i32
        if args.len() == 1 {
            if let Some(arg) = args[0].as_i64() {
                if let Ok(func) = instance.get_typed_func::<(i32,), i32>(store, function_name) {
                    let result = func.call(store, (arg as i32,))
                        .map_err(|e| EdgeComputeError::AlgorithmExecution {
                            message: format!("Failed to call function: {}", e),
                            algorithm: Some(function_name.to_string()),
                            input_size: None,
                        })?;
                    return Ok(serde_json::json!({ "result": result }));
                }
            }
        }
        
        // 5. 单个f64参数，返回f64
        if args.len() == 1 {
            if let Some(arg) = args[0].as_f64() {
                if let Ok(func) = instance.get_typed_func::<(f64,), f64>(store, function_name) {
                    let result = func.call(store, (arg,))
                        .map_err(|e| EdgeComputeError::AlgorithmExecution {
                            message: format!("Failed to call function: {}", e),
                            algorithm: Some(function_name.to_string()),
                            input_size: None,
                        })?;
                    return Ok(serde_json::json!({ "result": result }));
                }
            }
        }
        
        // 6. 两个i32参数，返回i32
        if args.len() == 2 {
            if let (Some(a1), Some(a2)) = (args[0].as_i64(), args[1].as_i64()) {
                if let Ok(func) = instance.get_typed_func::<(i32, i32), i32>(store, function_name) {
                    let result = func.call(store, (a1 as i32, a2 as i32))
                        .map_err(|e| EdgeComputeError::AlgorithmExecution {
                            message: format!("Failed to call function: {}", e),
                            algorithm: Some(function_name.to_string()),
                            input_size: None,
                        })?;
                    return Ok(serde_json::json!({ "result": result }));
                }
            }
        }
        
        // 7. 使用通用函数接口（支持动态参数）
        if let Ok(func) = instance.get_func(store, function_name) {
            // 转换参数为WASM值
            let mut wasm_args = Vec::new();
            for arg in args {
                let wasm_val = Self::json_to_wasm_value(arg)?;
                wasm_args.push(wasm_val);
            }
            
            // 调用函数
            let mut results = vec![wasmtime::Val::I32(0)];
            func.call(store, &wasm_args, &mut results)
                .map_err(|e| EdgeComputeError::AlgorithmExecution {
                    message: format!("Failed to call function: {}", e),
                    algorithm: Some(function_name.to_string()),
                    input_size: None,
                })?;
            
            // 转换返回值
            if let Some(result_val) = results.first() {
                let result_json = Self::wasm_value_to_json(result_val)?;
                return Ok(serde_json::json!({ "result": result_json }));
            }
        }
        
        Err(EdgeComputeError::AlgorithmExecution {
            message: format!("Function '{}' not found or has unsupported signature", function_name),
            algorithm: Some(function_name.to_string()),
            input_size: None,
        })
    }
    
    /// 将JSON值转换为WASM值
    #[cfg(feature = "wasm")]
    fn json_to_wasm_value(value: &serde_json::Value) -> Result<wasmtime::Val> {
        use wasmtime::Val;
        
        match value {
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Ok(Val::I32(i as i32))
                } else if let Some(f) = n.as_f64() {
                    Ok(Val::F64(f))
                } else {
                    Err(EdgeComputeError::AlgorithmExecution {
                        message: "Unsupported number type".to_string(),
                        algorithm: None,
                        input_size: None,
                    })
                }
            }
            serde_json::Value::String(s) => {
                // 字符串需要特殊处理（通常需要内存传递）
                // 这里简化处理，返回错误
                Err(EdgeComputeError::AlgorithmExecution {
                    message: "String parameters not yet supported".to_string(),
                    algorithm: None,
                    input_size: None,
                })
            }
            _ => Err(EdgeComputeError::AlgorithmExecution {
                message: format!("Unsupported parameter type: {:?}", value),
                algorithm: None,
                input_size: None,
            }),
        }
    }
    
    /// 将WASM值转换为JSON值
    #[cfg(feature = "wasm")]
    fn wasm_value_to_json(value: &wasmtime::Val) -> Result<serde_json::Value> {
        use wasmtime::Val;
        
        match value {
            Val::I32(i) => Ok(serde_json::json!(*i)),
            Val::I64(i) => Ok(serde_json::json!(*i)),
            Val::F32(f) => Ok(serde_json::json!(*f)),
            Val::F64(f) => Ok(serde_json::json!(*f)),
            Val::V128(_) => Err(EdgeComputeError::AlgorithmExecution {
                message: "V128 return type not supported".to_string(),
                algorithm: None,
                input_size: None,
            }),
            Val::FuncRef(_) | Val::ExternRef(_) => Err(EdgeComputeError::AlgorithmExecution {
                message: "Reference return type not supported".to_string(),
                algorithm: None,
                input_size: None,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_wasm_sandbox_creation() {
        let sandbox = WasmSandbox::new(WasmSandboxConfig::default()).unwrap();
        let stats = sandbox.get_stats().await;
        assert_eq!(stats.total_executions, 0);
    }
}
