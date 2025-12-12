// Cap'n Proto 跨语言调用服务实现
// 通过RPC调用Rust-C++算法执行器

use capnp::capability::Promise;
use std::sync::Arc;
use serde_json::Value;
use std::collections::HashMap;
use async_trait::async_trait;

// 包含从algorithm.capnp生成的类型定义
include!(concat!(env!("OUT_DIR"), "/algorithm_capnp.rs"));

use crate::ffi::bridge::CppAlgorithmExecutor;

// ============================================================================
// AlgorithmService 实现 - 处理跨语言RPC调用
// ============================================================================

pub struct AlgorithmServiceImpl {
    /// 持有C++侧的执行器引用
    cpp_executor: Arc<CppAlgorithmExecutor>,
    
    /// 统计信息
    stats: Arc<tokio::sync::Mutex<ServiceStats>>,
}

#[derive(Debug, Clone, Default)]
struct ServiceStats {
    total_requests: u64,
    successful_requests: u64,
    failed_requests: u64,
    total_execution_time_ms: u64,
}

impl AlgorithmServiceImpl {
    /// 创建新的服务实例
    pub fn new(executor: Arc<CppAlgorithmExecutor>) -> Self {
        Self {
            cpp_executor: executor,
            stats: Arc::new(tokio::sync::Mutex::new(ServiceStats::default())),
        }
    }

    /// 根据插件类型和名称执行算法
    async fn execute_plugin_internal(
        &self,
        algorithm_name: &str,
        plugin_type: algorithm_capnp::PluginType,
        params_json: &str,
    ) -> Result<Value, String> {
        let start = std::time::Instant::now();
        
        // 解析参数JSON
        let params: Value = serde_json::from_str(params_json)
            .map_err(|e| format!("JSON parse error: {}", e))?;

        // 构建参数映射
        let mut param_map = HashMap::new();
        if let Some(obj) = params.as_object() {
            for (key, value) in obj {
                if let Some(s) = value.as_str() {
                    param_map.insert(key.clone(), s.to_string());
                }
            }
        }

        // 根据插件类型分发执行
        let result = match plugin_type {
            algorithm_capnp::PluginType::Feature => {
                self.cpp_executor
                    .execute_feature_plugin(algorithm_name, &params, &param_map)
                    .await
            }
            algorithm_capnp::PluginType::Decision => {
                self.cpp_executor
                    .execute_decision_plugin(algorithm_name, &params, &param_map)
                    .await
            }
            algorithm_capnp::PluginType::Evaluation => {
                self.cpp_executor
                    .execute_evaluation_plugin(algorithm_name, &params, &param_map)
                    .await
            }
            algorithm_capnp::PluginType::Event => {
                self.cpp_executor
                    .execute_event_plugin(algorithm_name, &params, &param_map)
                    .await
            }
            algorithm_capnp::PluginType::Unknown => {
                Err("Unknown plugin type".to_string())
            }
        }?;

        // 记录执行统计
        let elapsed = start.elapsed().as_millis() as u64;
        {
            let mut stats = self.stats.lock().await;
            stats.total_requests += 1;
            stats.successful_requests += 1;
            stats.total_execution_time_ms += elapsed;
        }

        Ok(result)
    }
}

// ============================================================================
// 实现 AlgorithmService RPC 接口
// ============================================================================

#[async_trait]
impl algorithm_capnp::algorithm_service::Server for AlgorithmServiceImpl {
    /// 执行算法 - 跨语言RPC调用入口
    async fn execute(
        &mut self,
        params: algorithm_capnp::algorithm_service::ExecuteParams,
        mut results: algorithm_capnp::algorithm_service::ExecuteResults,
    ) -> capnp::capability::Promise<(), capnp::Error> {
        // 获取请求参数
        let request = match params.get().and_then(|p| p.get_request()) {
            Ok(r) => r,
            Err(e) => {
                tracing::error!("Failed to get request: {:?}", e);
                return Promise::err(e);
            }
        };

        let request_id = match request.get_id() {
            Ok(id) => id.to_string(),
            Err(_) => "unknown".to_string(),
        };

        let algorithm_name = match request.get_algorithm_name() {
            Ok(name) => name.to_string(),
            Err(e) => {
                tracing::error!("Failed to get algorithm name: {:?}", e);
                return Promise::err(e);
            }
        };

        let plugin_type = request.get_plugin_type();
        
        let params_json = match request.get_parameters_json() {
            Ok(p) => p.to_string(),
            Err(e) => {
                tracing::error!("Failed to get parameters: {:?}", e);
                return Promise::err(e);
            }
        };

        tracing::info!(
            "[AlgorithmService] Received RPC request: {} for algorithm: {} (type: {:?})",
            request_id,
            algorithm_name,
            plugin_type
        );

        // 执行算法
        let (success, result_json, error_message, exec_time) = match self
            .execute_plugin_internal(&algorithm_name, plugin_type, &params_json)
            .await
        {
            Ok(result) => {
                tracing::info!("[AlgorithmService] Execution succeeded for {}", request_id);
                (
                    true,
                    serde_json::to_string(&result).unwrap_or_default(),
                    String::new(),
                    0,
                )
            }
            Err(e) => {
                tracing::error!("[AlgorithmService] Execution failed: {}", e);
                {
                    let mut stats = self.stats.lock().await;
                    stats.failed_requests += 1;
                }
                (false, String::new(), e, 0)
            }
        };

        // 构建响应
        let response_result = results.get().map_err(|e| {
            tracing::error!("Failed to build response: {:?}", e);
            e
        });

        match response_result {
            Ok(mut response) => {
                let mut resp = match response.init_response() {
                    Ok(r) => r,
                    Err(e) => return Promise::err(e),
                };
                
                resp.set_id(&request_id);
                resp.set_success(success);
                resp.set_result_json(&result_json);
                resp.set_error_message(&error_message);
                resp.set_execution_time_ms(exec_time);
                
                Promise::ok(())
            }
            Err(e) => Promise::err(e),
        }
    }

    /// 获取所有可用插件列表
    async fn list_plugins(
        &mut self,
        _params: algorithm_capnp::algorithm_service::ListPluginsParams,
        mut results: algorithm_capnp::algorithm_service::ListPluginsResults,
    ) -> capnp::capability::Promise<(), capnp::Error> {
        tracing::info!("[AlgorithmService] Listing plugins");

        // 获取可用插件列表（从C++执行器）
        let plugins = match self.cpp_executor.get_available_plugins().await {
            Ok(plugins) => plugins,
            Err(e) => {
                tracing::error!("Failed to get plugins: {}", e);
                return Promise::err(capnp::Error {
                    kind: capnp::error::Kind::Failed,
                    description: e.to_string(),
                });
            }
        };

        let response_result = results.get().map_err(|e| {
            tracing::error!("Failed to build response: {:?}", e);
            e
        });

        match response_result {
            Ok(mut response) => {
                match response.init_plugins(plugins.len() as u32) {
                    Ok(mut plugin_list) => {
                        for (i, plugin) in plugins.iter().enumerate() {
                            if let Ok(mut p) = plugin_list.reborrow().get(i as u32) {
                                p.set_name(&plugin.name);
                                p.set_version(&plugin.version);
                                p.set_description(&plugin.description);
                                // plugin.plugin_type 需要映射到 PluginType enum
                            }
                        }
                        Promise::ok(())
                    }
                    Err(e) => Promise::err(e),
                }
            }
            Err(e) => Promise::err(e),
        }
    }

    /// 获取特定插件的详细信息
    async fn get_plugin_info(
        &mut self,
        params: algorithm_capnp::algorithm_service::GetPluginInfoParams,
        mut results: algorithm_capnp::algorithm_service::GetPluginInfoResults,
    ) -> capnp::capability::Promise<(), capnp::Error> {
        let plugin_name = match params.get().and_then(|p| p.get_plugin_name()) {
            Ok(n) => n.to_string(),
            Err(e) => return Promise::err(e),
        };

        tracing::info!("[AlgorithmService] Getting info for plugin: {}", plugin_name);

        match self.cpp_executor.get_plugin_info(&plugin_name).await {
            Ok(info) => {
                match results.get() {
                    Ok(mut response) => {
                        response.set_found(true);
                        if let Ok(mut metadata) = response.init_metadata() {
                            metadata.set_name(&info.name);
                            metadata.set_version(&info.version);
                            metadata.set_description(&info.description);
                        }
                        Promise::ok(())
                    }
                    Err(e) => Promise::err(e),
                }
            }
            Err(_) => {
                match results.get() {
                    Ok(mut response) => {
                        response.set_found(false);
                        Promise::ok(())
                    }
                    Err(e) => Promise::err(e),
                }
            }
        }
    }

    /// 健康检查
    async fn health_check(
        &mut self,
        _params: algorithm_capnp::algorithm_service::HealthCheckParams,
        mut results: algorithm_capnp::algorithm_service::HealthCheckResults,
    ) -> capnp::capability::Promise<(), capnp::Error> {
        tracing::debug!("[AlgorithmService] Health check");

        let stats = self.stats.lock().await;
        let healthy = stats.failed_requests < stats.total_requests / 10; // 如果失败率 > 10%，认为不健康

        match results.get() {
            Ok(mut response) => {
                response.set_healthy(healthy);
                let message = if healthy { "System is healthy" } else { "System degraded" };
                response.set_message(message);
                Promise::ok(())
            }
            Err(e) => Promise::err(e),
        }
    }

    /// 获取系统指标
    async fn get_system_metrics(
        &mut self,
        _params: algorithm_capnp::algorithm_service::GetSystemMetricsParams,
        mut results: algorithm_capnp::algorithm_service::GetSystemMetricsResults,
    ) -> capnp::capability::Promise<(), capnp::Error> {
        let stats = self.stats.lock().await;
        let avg_time = if stats.successful_requests > 0 {
            (stats.total_execution_time_ms as f32) / (stats.successful_requests as f32)
        } else {
            0.0
        };

        match results.get() {
            Ok(mut response) => {
                match response.init_metrics() {
                    Ok(mut metrics) => {
                        metrics.set_total_calls(stats.total_requests);
                        metrics.set_successful_calls(stats.successful_requests);
                        metrics.set_failed_calls(stats.failed_requests);
                        metrics.set_avg_execution_time_ms(avg_time);
                        metrics.set_timestamp_ms(
                            std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_millis() as u64,
                        );
                        Promise::ok(())
                    }
                    Err(e) => Promise::err(e),
                }
            }
            Err(e) => Promise::err(e),
        }
    }
}

// ============================================================================
// RPC 服务器启动函数
// ============================================================================

/// 启动Cap'n Proto RPC服务器，供多语言客户端连接
pub async fn run_capnp_service(
    executor: Arc<CppAlgorithmExecutor>,
    addr: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use tokio::net::TcpListener;
    use capnp_rpc::rpc_twoparty_main;

    let listener = TcpListener::bind(addr).await?;
    tracing::info!("[CapnP Server] Started listening on {}", addr);

    loop {
        let (socket, peer_addr) = listener.accept().await?;
        tracing::info!("[CapnP Server] Accepted connection from {}", peer_addr);

        let executor_clone = executor.clone();

        tokio::spawn(async move {
            let stream = match socket.into_std() {
                Ok(s) => {
                    match s.set_nonblocking(false) {
                        Ok(()) => s,
                        Err(e) => {
                            tracing::error!("[CapnP Server] Failed to set nonblocking: {}", e);
                            return;
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("[CapnP Server] Failed to convert socket: {}", e);
                    return;
                }
            };

            let (reader, writer) = match stream.into_split() {
                Ok((r, w)) => (r, w),
                Err(e) => {
                    tracing::error!("[CapnP Server] Failed to split socket: {}", e);
                    return;
                }
            };

            let service = AlgorithmServiceImpl::new(executor_clone);
            
            match rpc_twoparty_main::run_server(
                Box::new(reader),
                Box::new(writer),
                service,
                Default::default(),
            )
            .await
            {
                Ok(_) => {
                    tracing::info!("[CapnP Server] Connection closed normally");
                }
                Err(e) => {
                    tracing::error!("[CapnP Server] RPC error: {}", e);
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_service_initialization() {
        let _ = tracing_subscriber::fmt::try_init();
        // 实际测试需要完整的CppAlgorithmExecutor实例
    }
}
