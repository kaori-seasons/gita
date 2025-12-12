// 生产级集成测试 - DAG 算法执行通过 FFI 调用 cpp_plugins

use rust_edge_compute_cpp::CppAlgorithmExecutorBridge;
use serde_json::{json, Value};

#[test]
fn test_executor_creation() {
    // 测试：执行器创建
    let result = CppAlgorithmExecutorBridge::new();
    assert!(result.is_ok(), "Failed to create executor");
    
    let executor = result.unwrap();
    assert!(!executor.is_initialized(), "Executor should not be initialized on creation");
}

#[test]
fn test_executor_initialization() {
    // 测试：执行器初始化
    let mut executor = CppAlgorithmExecutorBridge::new()
        .expect("Failed to create executor");
    
    let result = executor.initialize();
    assert!(result.is_ok(), "Failed to initialize executor");
    assert!(result.unwrap(), "Initialize should return true");
    assert!(executor.is_initialized(), "Executor should be initialized");
}

#[test]
fn test_get_available_plugins() {
    // 测试：获取可用插件列表
    let mut executor = CppAlgorithmExecutorBridge::new()
        .expect("Failed to create executor");
    
    executor.initialize().expect("Failed to initialize");
    
    let plugins = executor.get_available_plugins()
        .expect("Failed to get available plugins");
    
    // 验证所有 10 个生产级插件都在列表中
    assert_eq!(plugins.len(), 10, "Should have 10 plugins");
    
    let expected_plugins = vec![
        "vibrate31",
        "motor97",
        "current_feature_extractor",
        "temperature_feature_extractor",
        "audio_feature_extractor",
        "universal_classify1",
        "comp_realtime_health34",
        "error18",
        "score_alarm5",
        "status_alarm4",
    ];
    
    for expected in expected_plugins {
        assert!(plugins.contains(&expected.to_string()), 
                "Plugin {} should be in available plugins list", expected);
    }
}

#[test]
fn test_get_plugin_info() {
    // 测试：获取插件信息
    let mut executor = CppAlgorithmExecutorBridge::new()
        .expect("Failed to create executor");
    
    executor.initialize().expect("Failed to initialize");
    
    let vibrate_info = executor.get_plugin_info("vibrate31")
        .expect("Failed to get plugin info");
    
    assert!(vibrate_info.contains("vibrate31"), "Plugin name should be in info");
    assert!(vibrate_info.contains("version"), "Version should be in info");
    
    // 解析为 JSON 验证结构
    let _json: Value = serde_json::from_str(&vibrate_info)
        .expect("Plugin info should be valid JSON");
}

#[test]
fn test_vibrate31_algorithm_execution() {
    // 测试：执行 vibrate31 算法（特征提取）
    let mut executor = CppAlgorithmExecutorBridge::new()
        .expect("Failed to create executor");
    
    executor.initialize().expect("Failed to initialize");
    
    let parameters = json!({
        "device_id": "device_001",
        "sensor_data": [1.0, 2.0, 3.0],
        "sample_rate": 1000
    });
    
    let result = executor.execute_algorithm("vibrate31", &parameters)
        .expect("Failed to execute algorithm");
    
    // 验证返回结构
    assert_eq!(result["status"], "executed", "Status should be executed");
    assert_eq!(result["algorithm"], "vibrate31", "Algorithm name should match");
    assert_eq!(result["device_id"], "device_001", "Device ID should be preserved");
    assert_eq!(result["source"], "cpp_plugins", "Source should indicate cpp_plugins");
    
    // 验证结果包含算法特定的字段
    assert!(result["result"]["vibration_level"].is_string(), "Should have vibration_level");
    assert!(result["result"]["frequency_hz"].is_number(), "Should have frequency_hz");
}

#[test]
fn test_motor97_algorithm_execution() {
    // 测试：执行 motor97 算法（电机状态判断）
    let mut executor = CppAlgorithmExecutorBridge::new()
        .expect("Failed to create executor");
    
    executor.initialize().expect("Failed to initialize");
    
    let parameters = json!({
        "device_id": "motor_001",
        "current": 2.5,
        "voltage": 220.0
    });
    
    let result = executor.execute_algorithm("motor97", &parameters)
        .expect("Failed to execute algorithm");
    
    assert_eq!(result["algorithm"], "motor97");
    assert_eq!(result["device_id"], "motor_001");
    assert!(result["result"]["motor_state"].is_string());
    assert!(result["result"]["rpm"].is_number());
}

#[test]
fn test_current_feature_extractor() {
    // 测试：电流特征提取
    let mut executor = CppAlgorithmExecutorBridge::new()
        .expect("Failed to create executor");
    
    executor.initialize().expect("Failed to initialize");
    
    let parameters = json!({
        "device_id": "device_002",
        "current_readings": [2.4, 2.5, 2.6, 2.5]
    });
    
    let result = executor.execute_algorithm("current_feature_extractor", &parameters)
        .expect("Failed to execute algorithm");
    
    assert_eq!(result["algorithm"], "current_feature_extractor");
    assert!(result["result"]["mean_current"].is_number());
    assert!(result["result"]["std_dev"].is_number());
}

#[test]
fn test_temperature_feature_extractor() {
    // 测试：温度特征提取
    let mut executor = CppAlgorithmExecutorBridge::new()
        .expect("Failed to create executor");
    
    executor.initialize().expect("Failed to initialize");
    
    let parameters = json!({
        "device_id": "device_003",
        "temperature_readings": [60.0, 62.0, 65.0, 70.0]
    });
    
    let result = executor.execute_algorithm("temperature_feature_extractor", &parameters)
        .expect("Failed to execute algorithm");
    
    assert_eq!(result["algorithm"], "temperature_feature_extractor");
    assert!(result["result"]["mean_temp"].is_number());
    assert!(result["result"]["max_temp"].is_number());
}

#[test]
fn test_audio_feature_extractor() {
    // 测试：音频特征提取
    let mut executor = CppAlgorithmExecutorBridge::new()
        .expect("Failed to create executor");
    
    executor.initialize().expect("Failed to initialize");
    
    let parameters = json!({
        "device_id": "device_004",
        "audio_data": [0.1, 0.2, 0.15]
    });
    
    let result = executor.execute_algorithm("audio_feature_extractor", &parameters)
        .expect("Failed to execute algorithm");
    
    assert_eq!(result["algorithm"], "audio_feature_extractor");
    assert!(result["result"]["mfcc_mean"].is_number());
    assert!(result["result"]["energy"].is_number());
}

#[test]
fn test_universal_classify1_algorithm() {
    // 测试：通用分类器
    let mut executor = CppAlgorithmExecutorBridge::new()
        .expect("Failed to create executor");
    
    executor.initialize().expect("Failed to initialize");
    
    let parameters = json!({
        "device_id": "device_005",
        "features": [1.0, 2.0, 3.0, 4.0]
    });
    
    let result = executor.execute_algorithm("universal_classify1", &parameters)
        .expect("Failed to execute algorithm");
    
    assert_eq!(result["algorithm"], "universal_classify1");
    assert!(result["result"]["class"].is_string());
    assert!(result["result"]["confidence"].is_number());
    
    let confidence = result["result"]["confidence"].as_f64().unwrap();
    assert!(confidence >= 0.0 && confidence <= 1.0, "Confidence should be between 0 and 1");
}

#[test]
fn test_health_estimation() {
    // 测试：实时健康估计
    let mut executor = CppAlgorithmExecutorBridge::new()
        .expect("Failed to create executor");
    
    executor.initialize().expect("Failed to initialize");
    
    let parameters = json!({
        "device_id": "device_006",
        "health_indicators": {
            "vibration": 0.8,
            "temperature": 0.7
        }
    });
    
    let result = executor.execute_algorithm("comp_realtime_health34", &parameters)
        .expect("Failed to execute algorithm");
    
    assert_eq!(result["algorithm"], "comp_realtime_health34");
    assert!(result["result"]["health_score"].is_number());
    assert!(result["result"]["status"].is_string());
}

#[test]
fn test_error_detection() {
    // 测试：错误检测
    let mut executor = CppAlgorithmExecutorBridge::new()
        .expect("Failed to create executor");
    
    executor.initialize().expect("Failed to initialize");
    
    let parameters = json!({
        "device_id": "device_007",
        "error_indicators": [0.1, 0.2, 0.15]
    });
    
    let result = executor.execute_algorithm("error18", &parameters)
        .expect("Failed to execute algorithm");
    
    assert_eq!(result["algorithm"], "error18");
    assert!(result["result"]["error_detected"].is_boolean());
    assert!(result["result"]["error_code"].is_number());
}

#[test]
fn test_score_alarm() {
    // 测试：分数警报
    let mut executor = CppAlgorithmExecutorBridge::new()
        .expect("Failed to create executor");
    
    executor.initialize().expect("Failed to initialize");
    
    let parameters = json!({
        "device_id": "device_008",
        "score": 0.3
    });
    
    let result = executor.execute_algorithm("score_alarm5", &parameters)
        .expect("Failed to execute algorithm");
    
    assert_eq!(result["algorithm"], "score_alarm5");
    assert!(result["result"]["alarm_triggered"].is_boolean());
    assert!(result["result"]["score"].is_number());
}

#[test]
fn test_status_alarm() {
    // 测试：状态警报
    let mut executor = CppAlgorithmExecutorBridge::new()
        .expect("Failed to create executor");
    
    executor.initialize().expect("Failed to initialize");
    
    let parameters = json!({
        "device_id": "device_009",
        "status": "running"
    });
    
    let result = executor.execute_algorithm("status_alarm4", &parameters)
        .expect("Failed to execute algorithm");
    
    assert_eq!(result["algorithm"], "status_alarm4");
    assert!(result["result"]["alarm_triggered"].is_boolean());
    assert!(result["result"]["status"].is_string());
}

#[test]
fn test_unknown_algorithm_error() {
    // 测试：未知算法错误处理
    let mut executor = CppAlgorithmExecutorBridge::new()
        .expect("Failed to create executor");
    
    executor.initialize().expect("Failed to initialize");
    
    let parameters = json!({
        "device_id": "device_010"
    });
    
    let result = executor.execute_algorithm("unknown_algorithm", &parameters);
    
    assert!(result.is_err(), "Should return error for unknown algorithm");
    let error = result.unwrap_err();
    assert!(error.contains("Unknown algorithm"), "Error message should indicate unknown algorithm");
}

#[test]
fn test_uninitialized_executor_error() {
    // 测试：未初始化的执行器错误处理
    let executor = CppAlgorithmExecutorBridge::new()
        .expect("Failed to create executor");
    
    let parameters = json!({
        "device_id": "device_011"
    });
    
    let result = executor.execute_algorithm("vibrate31", &parameters);
    assert!(result.is_err(), "Should fail on uninitialized executor");
}

#[test]
fn test_timestamp_preservation() {
    // 测试：时间戳保存（生产级要求）
    let mut executor = CppAlgorithmExecutorBridge::new()
        .expect("Failed to create executor");
    
    executor.initialize().expect("Failed to initialize");
    
    let parameters = json!({
        "device_id": "device_012",
        "data": "test"
    });
    
    let result = executor.execute_algorithm("vibrate31", &parameters)
        .expect("Failed to execute algorithm");
    
    assert!(result["timestamp_ms"].is_number(), "Should have timestamp_ms");
    let timestamp = result["timestamp_ms"].as_u64().unwrap();
    assert!(timestamp > 0, "Timestamp should be positive");
}

#[test]
fn test_device_id_preservation() {
    // 测试：设备 ID 保存
    let mut executor = CppAlgorithmExecutorBridge::new()
        .expect("Failed to create executor");
    
    executor.initialize().expect("Failed to initialize");
    
    let device_ids = vec!["dev_001", "dev_002", "dev_003"];
    
    for device_id in device_ids {
        let parameters = json!({
            "device_id": device_id,
            "data": "test"
        });
        
        let result = executor.execute_algorithm("vibrate31", &parameters)
            .expect("Failed to execute algorithm");
        
        assert_eq!(result["device_id"].as_str().unwrap(), device_id, 
                   "Device ID should be preserved");
    }
}

#[test]
fn test_dag_pipeline_simulation() {
    // 测试：DAG 管道模拟
    // 模拟一个 DAG：vibrate31 -> universal_classify1 -> comp_realtime_health34
    
    let mut executor = CppAlgorithmExecutorBridge::new()
        .expect("Failed to create executor");
    
    executor.initialize().expect("Failed to initialize");
    
    let device_id = "pipeline_device";
    
    // 第一步：特征提取
    let feature_params = json!({
        "device_id": device_id,
        "sensor_data": [1.0, 2.0, 3.0]
    });
    
    let feature_result = executor.execute_algorithm("vibrate31", &feature_params)
        .expect("Feature extraction failed");
    
    assert_eq!(feature_result["status"], "executed");
    println!("Step 1 - Feature Extraction: {:?}", feature_result);
    
    // 第二步：分类
    let classify_params = json!({
        "device_id": device_id,
        "features": feature_result["result"].clone()
    });
    
    let classify_result = executor.execute_algorithm("universal_classify1", &classify_params)
        .expect("Classification failed");
    
    assert_eq!(classify_result["status"], "executed");
    println!("Step 2 - Classification: {:?}", classify_result);
    
    // 第三步：健康评估
    let health_params = json!({
        "device_id": device_id,
        "classification": classify_result["result"].clone()
    });
    
    let health_result = executor.execute_algorithm("comp_realtime_health34", &health_params)
        .expect("Health estimation failed");
    
    assert_eq!(health_result["status"], "executed");
    println!("Step 3 - Health Estimation: {:?}", health_result);
    
    // 验证完整的 DAG 管道
    assert_eq!(feature_result["device_id"], classify_result["device_id"]);
    assert_eq!(classify_result["device_id"], health_result["device_id"]);
}

#[test]
fn test_parameter_json_handling() {
    // 测试：JSON 参数处理
    let mut executor = CppAlgorithmExecutorBridge::new()
        .expect("Failed to create executor");
    
    executor.initialize().expect("Failed to initialize");
    
    let parameters = json!({
        "device_id": "device_013",
        "nested": {
            "level1": {
                "level2": "value"
            }
        },
        "array": [1, 2, 3],
        "number": 3.14,
        "boolean": true,
        "null_value": null
    });
    
    let result = executor.execute_algorithm("vibrate31", &parameters)
        .expect("Failed to execute algorithm");
    
    assert_eq!(result["status"], "executed");
    assert!(result["parameters"].is_object(), "Parameters should be preserved in result");
}

#[test]
fn test_result_json_structure() {
    // 测试：结果 JSON 结构完整性
    let mut executor = CppAlgorithmExecutorBridge::new()
        .expect("Failed to create executor");
    
    executor.initialize().expect("Failed to initialize");
    
    let parameters = json!({ "device_id": "device_014" });
    
    let result = executor.execute_algorithm("vibrate31", &parameters)
        .expect("Failed to execute algorithm");
    
    // 验证完整的结构
    assert!(result.is_object(), "Result should be JSON object");
    assert!(result["status"].is_string(), "Should have status");
    assert!(result["algorithm"].is_string(), "Should have algorithm");
    assert!(result["device_id"].is_string(), "Should have device_id");
    assert!(result["timestamp_ms"].is_number(), "Should have timestamp_ms");
    assert!(result["source"].is_string(), "Should have source");
    assert!(result["result"].is_object(), "Should have result object");
}

#[test]
fn test_all_10_plugins_execution() {
    // 测试：所有 10 个插件都能成功执行
    let mut executor = CppAlgorithmExecutorBridge::new()
        .expect("Failed to create executor");
    
    executor.initialize().expect("Failed to initialize");
    
    let algorithms = vec![
        "vibrate31",
        "motor97",
        "current_feature_extractor",
        "temperature_feature_extractor",
        "audio_feature_extractor",
        "universal_classify1",
        "comp_realtime_health34",
        "error18",
        "score_alarm5",
        "status_alarm4",
    ];
    
    let parameters = json!({
        "device_id": "test_device",
        "data": "test_data"
    });
    
    for algorithm in algorithms {
        let result = executor.execute_algorithm(algorithm, &parameters)
            .expect(&format!("Failed to execute {}", algorithm));
        
        assert_eq!(result["status"], "executed", "Status should be executed for {}", algorithm);
        assert_eq!(result["algorithm"], algorithm, "Algorithm name should match");
        assert_eq!(result["source"], "cpp_plugins", "Source should be cpp_plugins for {}", algorithm);
        
        println!("✓ {} executed successfully", algorithm);
    }
}
