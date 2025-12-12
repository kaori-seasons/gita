//! C++ FFI 相关的 HTTP 处理器

use axum::{
    extract::Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;
use std::sync::Arc;

use super::handlers::AppState;
use rust_edge_compute_cpp::CppAlgorithmExecutorBridge;

/// C++ FFI 算法执行请求
#[derive(Debug, serde::Deserialize)]
pub struct ExecuteCppAlgorithmRequest {
    /// 算法名称
    pub algorithm_name: String,
    /// 算法参数
    pub parameters: serde_json::Value,
    /// 设备ID
    #[serde(default = "default_device_id")]
    pub device_id: String,
}

fn default_device_id() -> String {
    "default_device".to_string()
}

/// 执行 C++ 算法处理器
pub async fn execute_cpp_algorithm(
    axum::extract::State(_state): axum::extract::State<AppState>,
    Json(request): Json<ExecuteCppAlgorithmRequest>,
) -> Response {
    tracing::info!(
        "Executing C++ algorithm: {} on device: {}",
        request.algorithm_name,
        request.device_id
    );

    // 创建执行器实例
    match CppAlgorithmExecutorBridge::new() {
        Ok(mut executor) => {
            // 初始化执行器
            match executor.initialize() {
                Ok(_) => {
                    // 执行算法
                    match executor.execute_algorithm(
                        &request.algorithm_name,
                        &request.parameters,
                        &request.device_id,
                    ) {
                        Ok(output) => {
                            tracing::info!("C++ algorithm execution completed");
                            (
                                StatusCode::OK,
                                Json(json!({
                                    "success": output.success,
                                    "algorithm": request.algorithm_name,
                                    "device_id": request.device_id,
                                    "result": output.result_json,
                                    "error_message": output.error_message,
                                    "execution_time_ms": output.execution_time_ms,
                                    "memory_used_bytes": output.memory_used_bytes,
                                })),
                            )
                                .into_response()
                        }
                        Err(e) => {
                            tracing::error!("C++ algorithm execution failed: {}", e);
                            (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(json!({
                                    "success": false,
                                    "algorithm": request.algorithm_name,
                                    "error": e,
                                })),
                            )
                                .into_response()
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to initialize C++ executor: {}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({
                            "error": "Executor initialization failed",
                            "details": e,
                        })),
                    )
                        .into_response()
                }
            }
        }
        Err(e) => {
            tracing::error!("Failed to create C++ executor: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Failed to create executor",
                    "details": e,
                })),
            )
                .into_response()
        }
    }
}

/// 获取 C++ 可用算法列表
pub async fn list_cpp_algorithms(
    _state: axum::extract::State<AppState>,
) -> Response {
    match CppAlgorithmExecutorBridge::new() {
        Ok(executor) => match executor.get_available_plugins() {
            Ok(algorithms) => (
                StatusCode::OK,
                Json(json!({
                    "algorithms": algorithms,
                    "count": algorithms.len(),
                    "source": "cpp_plugins",
                })),
            )
                .into_response(),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": format!("Failed to retrieve C++ algorithms: {}", e),
                })),
            )
                .into_response(),
        },
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": format!("Failed to create executor: {}", e),
            })),
        )
            .into_response(),
    }
}

/// 获取 C++ 算法信息
pub async fn get_cpp_algorithm_info(
    axum::extract::State(_state): axum::extract::State<AppState>,
    axum::extract::Path(algorithm_name): axum::extract::Path<String>,
) -> Response {
    match CppAlgorithmExecutorBridge::new() {
        Ok(executor) => match executor.get_plugin_info(&algorithm_name) {
            Ok(info) => (StatusCode::OK, Json(info)).into_response(),
            Err(_) => (
                StatusCode::NOT_FOUND,
                Json(json!({
                    "error": format!("C++ algorithm '{}' not found", algorithm_name),
                })),
            )
                .into_response(),
        },
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": format!("Failed to create executor: {}", e),
            })),
        )
            .into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execute_cpp_algorithm_request_parsing() {
        let json_str = r#"{
            "algorithm_name": "vibrate31",
            "parameters": {"threshold": 0.5},
            "device_id": "device_001"
        }"#;

        let request: ExecuteCppAlgorithmRequest = serde_json::from_str(json_str).unwrap();
        assert_eq!(request.algorithm_name, "vibrate31");
        assert_eq!(request.device_id, "device_001");
    }

    #[test]
    fn test_default_cpp_device_id() {
        let json_str = r#"{
            "algorithm_name": "vibrate31",
            "parameters": {}
        }"#;

        let request: ExecuteCppAlgorithmRequest = serde_json::from_str(json_str).unwrap();
        assert_eq!(request.device_id, "default_device");
    }
}
