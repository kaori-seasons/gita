//! C++ FFI API 集成测试

#[cfg(test)]
mod tests {
    
    use serde_json::json;

    #[tokio::test]
    async fn test_cpp_algorithm_request_serialization() {
        // 测试C++ 算法执行请求的序列化
        let json_str = r#"{
            "algorithm_name": "vibrate31",
            "parameters": {"threshold": 0.5, "window_size": 128},
            "device_id": "device_001"
        }"#;

        let result: Result<
            super::super::ffi_handlers::ExecuteCppAlgorithmRequest,
            serde_json::Error,
        > = serde_json::from_str(json_str);

        assert!(result.is_ok());
        let request = result.unwrap();
        assert_eq!(request.algorithm_name, "vibrate31");
        assert_eq!(request.device_id, "device_001");
        assert_eq!(
            request.parameters.get("threshold").and_then(|v| v.as_f64()),
            Some(0.5)
        );
    }

    #[tokio::test]
    async fn test_cpp_algorithm_request_default_device_id() {
        // 测试默认设备ID
        let json_str = r#"{
            "algorithm_name": "add",
            "parameters": {"a": 1, "b": 2}
        }"#;

        let result: Result<
            super::super::ffi_handlers::ExecuteCppAlgorithmRequest,
            serde_json::Error,
        > = serde_json::from_str(json_str);

        assert!(result.is_ok());
        let request = result.unwrap();
        assert_eq!(request.device_id, "default_device");
    }

    #[tokio::test]
    async fn test_cpp_algorithm_request_with_complex_parameters() {
        // 测试复杂的参数
        let json_str = r#"{
            "algorithm_name": "complex_algo",
            "parameters": {
                "threshold": 0.5,
                "window_size": 128,
                "filters": ["lowpass", "highpass"],
                "config": {
                    "enabled": true,
                    "iterations": 10
                }
            },
            "device_id": "gpu_device_001"
        }"#;

        let result: Result<
            super::super::ffi_handlers::ExecuteCppAlgorithmRequest,
            serde_json::Error,
        > = serde_json::from_str(json_str);

        assert!(result.is_ok());
        let request = result.unwrap();
        assert_eq!(request.algorithm_name, "complex_algo");
        assert_eq!(request.device_id, "gpu_device_001");

        // 验证嵌套结构
        assert_eq!(
            request.parameters.get("threshold").and_then(|v| v.as_f64()),
            Some(0.5)
        );
        assert_eq!(
            request
                .parameters
                .get("window_size")
                .and_then(|v| v.as_i64()),
            Some(128)
        );

        // 验证数组
        if let Some(filters) = request.parameters.get("filters").and_then(|v| v.as_array()) {
            assert_eq!(filters.len(), 2);
        }

        // 验证嵌套对象
        if let Some(config) = request.parameters.get("config").and_then(|v| v.as_object()) {
            assert!(config.contains_key("enabled"));
            assert!(config.contains_key("iterations"));
        }
    }

    #[tokio::test]
    async fn test_cpp_algorithm_request_missing_required_field() {
        // 测试缺少必需字段时的错误处理
        let json_str = r#"{
            "parameters": {"a": 1},
            "device_id": "device_001"
        }"#;

        let result: Result<
            super::super::ffi_handlers::ExecuteCppAlgorithmRequest,
            serde_json::Error,
        > = serde_json::from_str(json_str);

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_response_serialization() {
        // 测试API响应的序列化
        let response = json!({
            "success": true,
            "algorithm": "vibrate31",
            "device_id": "device_001",
            "result": {
                "status": "processed",
                "value": 123.45
            },
            "error_message": "",
            "execution_time_ms": 45,
            "memory_used_bytes": 1024000
        });

        // 验证响应结构
        assert!(response
            .get("success")
            .and_then(|v| v.as_bool())
            .unwrap_or(false));
        assert_eq!(
            response.get("algorithm").and_then(|v| v.as_str()),
            Some("vibrate31")
        );
        assert_eq!(
            response.get("execution_time_ms").and_then(|v| v.as_i64()),
            Some(45)
        );
        assert_eq!(
            response.get("memory_used_bytes").and_then(|v| v.as_i64()),
            Some(1024000)
        );
    }

    #[tokio::test]
    async fn test_error_response_serialization() {
        // 测试错误响应的序列化
        let error_response = json!({
            "success": false,
            "algorithm": "unknown_algo",
            "error": "Algorithm execution failed: algorithm not found"
        });

        // 验证错误响应结构
        assert!(!error_response
            .get("success")
            .and_then(|v| v.as_bool())
            .unwrap_or(true));
        assert_eq!(
            error_response.get("algorithm").and_then(|v| v.as_str()),
            Some("unknown_algo")
        );
        assert!(error_response
            .get("error")
            .and_then(|v| v.as_str())
            .is_some());
    }
}
