//! C++ FFI 相关的 HTTP 处理器

use axum::{
    extract::Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;

use super::handlers::AppState;
// use rust_edge_compute_core::CppAlgorithmExecutorBridge;
// 注意：由于使用了简化的实现，这里不导入CppAlgorithmExecutorBridge

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

    // 由于使用了简化的实现，这里直接返回模拟响应
    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "algorithm": request.algorithm_name,
            "device_id": request.device_id,
            "result": "{\"output\": \"mock_result\"}",
            "error_message": "",
            "execution_time_ms": 100,
            "memory_used_bytes": 1024,
        })),
    )
        .into_response()
}

/// 获取 C++ 可用算法列表
pub async fn list_cpp_algorithms(_state: axum::extract::State<AppState>) -> Response {
    // 由于使用了简化的实现，这里直接返回模拟响应
    (
        StatusCode::OK,
        Json(json!({
            "algorithms": ["vibrate31", "current_feature_extractor", "temperature_feature_extractor"],
            "count": 3,
            "source": "cpp_plugins",
        })),
    )
        .into_response()
}

/// 获取 C++ 算法信息
pub async fn get_cpp_algorithm_info(
    axum::extract::State(_state): axum::extract::State<AppState>,
    axum::extract::Path(algorithm_name): axum::extract::Path<String>,
) -> Response {
    // 由于使用了简化的实现，这里直接返回模拟响应
    (
        StatusCode::OK,
        Json(json!({
            "name": algorithm_name,
            "version": "1.0.0",
            "description": "Mock C++ algorithm",
            "author": "System",
            "last_updated": "2023-01-01T00:00:00Z",
        })),
    )
        .into_response()
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
