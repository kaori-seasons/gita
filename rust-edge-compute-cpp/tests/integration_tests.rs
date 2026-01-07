// 生产级集成测试 - DAG 算法执行通过 FFI 调用 cpp_plugins

use rust_edge_compute_cpp::CppAlgorithmExecutorBridge;
use serde_json::json;

#[test]
fn test_executor_creation() {
    // 测试：执行器创建
    let result = CppAlgorithmExecutorBridge::new();
    assert!(result.is_ok(), "Failed to create executor");

    let executor = result.unwrap();
    assert!(
        !executor.is_initialized(),
        "Executor should not be initialized on creation"
    );
}

#[test]
fn test_executor_initialization() {
    // 测试：执行器初始化
    let mut executor = CppAlgorithmExecutorBridge::new().expect("Failed to create executor");

    let result = executor.initialize();
    assert!(result.is_ok(), "Failed to initialize executor");
    assert!(result.unwrap(), "Initialize should return true");
    assert!(executor.is_initialized(), "Executor should be initialized");
}

#[test]
fn test_get_available_plugins() {
    // 测试：获取可用插件列表
    let mut executor = CppAlgorithmExecutorBridge::new().expect("Failed to create executor");

    executor.initialize().expect("Failed to initialize");

    let plugins = executor
        .get_available_plugins()
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
        assert!(
            plugins.contains(&expected.to_string()),
            "Plugin {} should be in available plugins list",
            expected
        );
    }
}

#[test]
fn test_get_plugin_info() {
    // 测试：获取插件信息
    let mut executor = CppAlgorithmExecutorBridge::new().expect("Failed to create executor");

    executor.initialize().expect("Failed to initialize");

    let vibrate_info = executor
        .get_plugin_info("vibrate31")
        .expect("Failed to get plugin info");

    // 检查Value对象中的字段
    assert!(
        vibrate_info.get("name").is_some(),
        "Plugin should have name field"
    );
    assert!(
        vibrate_info.get("version").is_some(),
        "Plugin should have version field"
    );

    // 验证结构（已经是Value类型，不需要再次解析）
    assert!(
        vibrate_info.is_object(),
        "Plugin info should be a JSON object"
    );
    assert_eq!(
        vibrate_info["name"], "vibrate31",
        "Plugin name should match"
    );
}

#[test]
fn test_vibrate31_algorithm_execution() {
    // 测试：执行 vibrate31 算法（振动分析）
    let mut executor = CppAlgorithmExecutorBridge::new().expect("Failed to create executor");

    executor.initialize().expect("Failed to initialize");

    let parameters = json!({
        "device_id": "device_001",
        "sensor_data": [1.0, 2.0, 3.0],
        "sample_rate": 1000
    });

    let result = executor
        .execute_algorithm("vibrate31", &parameters, "device_001")
        .expect("Failed to execute algorithm");

    // 验证返回结构
    assert_eq!(
        result.result_json["status"], "executed",
        "Status should be executed"
    );
    assert_eq!(
        result.result_json["algorithm"], "vibrate31",
        "Algorithm name should match"
    );
    assert_eq!(
        result.result_json["device_id"], "device_001",
        "Device ID should be preserved"
    );
    assert_eq!(
        result.result_json["source"], "cpp_plugins",
        "Source should indicate cpp_plugins"
    );

    // 验证结果包含算法特定的字段
    assert!(
        result.result_json["result"]["vibration_level"].is_string(),
        "Should have vibration_level"
    );
    assert!(
        result.result_json["result"]["frequency_hz"].is_number(),
        "Should have frequency_hz"
    );
}

#[test]
fn test_motor97_algorithm_execution() {
    // 测试：执行 motor97 算法（电机状态判断）
    let mut executor = CppAlgorithmExecutorBridge::new().expect("Failed to create executor");

    executor.initialize().expect("Failed to initialize");

    let parameters = json!({
        "device_id": "motor_001",
        "current": 2.5,
        "voltage": 220.0
    });

    let result = executor
        .execute_algorithm("motor97", &parameters, "motor_001")
        .expect("Failed to execute algorithm");

    assert_eq!(result.result_json["algorithm"], "motor97");
    assert_eq!(result.result_json["device_id"], "motor_001");
    assert!(result.result_json["result"]["motor_state"].is_string());
    assert!(result.result_json["result"]["rpm"].is_number());
}

#[test]
fn test_current_feature_extractor() {
    // 测试：电流特征提取
    let mut executor = CppAlgorithmExecutorBridge::new().expect("Failed to create executor");

    executor.initialize().expect("Failed to initialize");

    let parameters = json!({
        "device_id": "device_002",
        "current_readings": [2.4, 2.5, 2.6, 2.5]
    });

    let result = executor
        .execute_algorithm("current_feature_extractor", &parameters, "device_002")
        .expect("Failed to execute algorithm");

    assert_eq!(result.result_json["algorithm"], "current_feature_extractor");
    assert!(result.result_json["result"]["mean_current"].is_number());
    assert!(result.result_json["result"]["std_dev"].is_number());
}

#[test]
fn test_temperature_feature_extractor() {
    // 测试：温度特征提取
    let mut executor = CppAlgorithmExecutorBridge::new().expect("Failed to create executor");

    executor.initialize().expect("Failed to initialize");

    let parameters = json!({
        "device_id": "device_003",
        "temperature_readings": [60.0, 62.0, 65.0, 70.0]
    });

    let result = executor
        .execute_algorithm("temperature_feature_extractor", &parameters, "device_003")
        .expect("Failed to execute algorithm");

    assert_eq!(
        result.result_json["algorithm"],
        "temperature_feature_extractor"
    );
    assert!(result.result_json["result"]["mean_temp"].is_number());
    assert!(result.result_json["result"]["max_temp"].is_number());
}

#[test]
fn test_audio_feature_extractor() {
    // 测试：音频特征提取
    let mut executor = CppAlgorithmExecutorBridge::new().expect("Failed to create executor");

    executor.initialize().expect("Failed to initialize");

    let parameters = json!({
        "device_id": "device_004",
        "audio_data": [0.1, 0.2, 0.15]
    });

    let result = executor
        .execute_algorithm("audio_feature_extractor", &parameters, "device_004")
        .expect("Failed to execute algorithm");

    assert_eq!(result.result_json["algorithm"], "audio_feature_extractor");
    assert!(result.result_json["result"]["mfcc_mean"].is_number());
    assert!(result.result_json["result"]["energy"].is_number());
}

#[test]
fn test_universal_classify1_algorithm() {
    // 测试：通用分类器
    let mut executor = CppAlgorithmExecutorBridge::new().expect("Failed to create executor");

    executor.initialize().expect("Failed to initialize");

    let parameters = json!({
        "device_id": "device_005",
        "features": [1.0, 2.0, 3.0, 4.0]
    });

    let result = executor
        .execute_algorithm("universal_classify1", &parameters, "device_005")
        .expect("Failed to execute algorithm");

    assert_eq!(result.result_json["algorithm"], "universal_classify1");
    assert!(result.result_json["result"]["class"].is_string());
    assert!(result.result_json["result"]["confidence"].is_number());

    let confidence = result.result_json["result"]["confidence"].as_f64().unwrap();
    assert!(
        (0.0..=1.0).contains(&confidence),
        "Confidence should be between 0 and 1"
    );
}

#[test]
fn test_health_estimation() {
    // 测试：实时健康估计
    let mut executor = CppAlgorithmExecutorBridge::new().expect("Failed to create executor");

    executor.initialize().expect("Failed to initialize");

    let parameters = json!({
        "device_id": "device_006",
        "health_indicators": {
            "vibration": 0.8,
            "temperature": 0.7
        }
    });

    let result = executor
        .execute_algorithm("comp_realtime_health34", &parameters, "device_006")
        .expect("Failed to execute algorithm");

    assert_eq!(result.result_json["algorithm"], "comp_realtime_health34");
    assert!(result.result_json["result"]["health_score"].is_number());
    assert!(result.result_json["result"]["status"].is_string());
}

#[test]
fn test_error_detection() {
    // 测试：错误检测
    let mut executor = CppAlgorithmExecutorBridge::new().expect("Failed to create executor");

    executor.initialize().expect("Failed to initialize");

    let parameters = json!({
        "device_id": "device_007",
        "error_indicators": [0.1, 0.2, 0.15]
    });

    let result = executor
        .execute_algorithm("error18", &parameters, "device_007")
        .expect("Failed to execute algorithm");

    assert_eq!(result.result_json["algorithm"], "error18");
    assert!(result.result_json["result"]["error_detected"].is_boolean());
    assert!(result.result_json["result"]["error_code"].is_number());
}

#[test]
fn test_score_alarm() {
    // 测试：分数警报
    let mut executor = CppAlgorithmExecutorBridge::new().expect("Failed to create executor");

    executor.initialize().expect("Failed to initialize");

    let parameters = json!({
        "device_id": "device_008",
        "score": 0.3
    });

    let result = executor
        .execute_algorithm("score_alarm5", &parameters, "device_008")
        .expect("Failed to execute algorithm");

    assert_eq!(result.result_json["algorithm"], "score_alarm5");
    assert!(result.result_json["result"]["alarm_triggered"].is_boolean());
    assert!(result.result_json["result"]["score"].is_number());
}

#[test]
fn test_status_alarm() {
    // 测试：状态警报
    let mut executor = CppAlgorithmExecutorBridge::new().expect("Failed to create executor");

    executor.initialize().expect("Failed to initialize");

    let parameters = json!({
        "device_id": "device_009",
        "status": "warning"
    });

    let result = executor
        .execute_algorithm("status_alarm4", &parameters, "device_009")
        .expect("Failed to execute algorithm");

    assert_eq!(result.result_json["algorithm"], "status_alarm4");
    assert!(result.result_json["result"]["alarm_triggered"].is_boolean());
    assert!(result.result_json["result"]["status_level"].is_string());
}

#[test]
fn test_unknown_algorithm_handling() {
    // 测试：未知算法的处理
    let mut executor = CppAlgorithmExecutorBridge::new().expect("Failed to create executor");

    executor.initialize().expect("Failed to initialize");

    let parameters = json!({ "test": "data" });

    let result = executor.execute_algorithm("unknown_algorithm", &parameters, "test_device");

    // 应该返回Err而不是panic
    assert!(result.is_err(), "Unknown algorithm should return error");
    println!("Unknown algorithm handled correctly: {:?}", result);
}

#[test]
fn test_algorithm_with_empty_parameters() {
    // 测试：空参数处理
    let mut executor = CppAlgorithmExecutorBridge::new().expect("Failed to create executor");

    executor.initialize().expect("Failed to initialize");

    let parameters = json!({});

    let result = executor.execute_algorithm("vibrate31", &parameters, "test_device");

    // 即使参数为空也应该正常处理
    assert!(
        result.is_ok(),
        "Empty parameters should be handled gracefully"
    );
    let output = result.unwrap();
    assert!(
        output.success,
        "Execution should succeed even with empty params"
    );
}

#[test]
fn test_execution_time_measurement() {
    // 测试：执行时间测量
    let mut executor = CppAlgorithmExecutorBridge::new().expect("Failed to create executor");

    executor.initialize().expect("Failed to initialize");

    let parameters = json!({
        "device_id": "timing_test_device",
        "data": vec![1.0; 1000] // 较大的数据集
    });

    let result = executor
        .execute_algorithm("vibrate31", &parameters, "timing_test_device")
        .expect("Failed to execute algorithm");

    assert!(
        result.execution_time_ms > 0,
        "Execution time should be measured"
    );
    assert!(
        result.execution_time_ms < 5000,
        "Execution should complete within reasonable time (< 5s)"
    );
    println!("Execution completed in {} ms", result.execution_time_ms);
}

#[test]
fn test_unknown_algorithm_error() {
    // 测试：未知算法错误处理
    let mut executor = CppAlgorithmExecutorBridge::new().expect("Failed to create executor");

    executor.initialize().expect("Failed to initialize");

    let parameters = json!({ "test": "data" });

    let result = executor.execute_algorithm("unknown_algorithm", &parameters, "test_device");

    // 应该返回Err而不是panic
    assert!(result.is_err(), "Unknown algorithm should return error");
    println!("Unknown algorithm handled correctly");
}

#[test]
fn test_algorithm_execution_without_initialization() {
    // 测试：未初始化执行器的处理
    let executor = CppAlgorithmExecutorBridge::new().expect("Failed to create executor");

    // 注意：这里没有调用 initialize()

    let parameters = json!({ "test": "data" });

    let result = executor.execute_algorithm("vibrate31", &parameters, "test_device");

    // 应该返回Err而不是panic
    assert!(
        result.is_err(),
        "Uninitialized executor should return error"
    );
    println!("Uninitialized executor handled correctly");
}

#[test]
fn test_single_algorithm_execution_with_full_validation() {
    // 测试：单个算法执行并完整验证返回结构
    let mut executor = CppAlgorithmExecutorBridge::new().expect("Failed to create executor");

    executor.initialize().expect("Failed to initialize");

    let parameters = json!({ "device_id": "device_014" });

    let result = executor
        .execute_algorithm("vibrate31", &parameters, "device_014")
        .expect("Failed to execute algorithm");

    // 验证完整的结构
    assert!(
        result.result_json.is_object(),
        "Result should be JSON object"
    );
    assert!(
        result.result_json["status"].is_string(),
        "Should have status"
    );
    assert!(
        result.result_json["algorithm"].is_string(),
        "Should have algorithm"
    );
    assert!(
        result.result_json["device_id"].is_string(),
        "Should have device_id"
    );
    assert!(
        result.result_json["timestamp_ms"].is_number(),
        "Should have timestamp_ms"
    );
    assert!(
        result.result_json["source"].is_string(),
        "Should have source"
    );
    assert!(
        result.result_json["result"].is_object(),
        "Should have result object"
    );
}

#[test]
fn test_all_10_plugins_execution() {
    // 测试：所有 10 个插件都能成功执行
    let mut executor = CppAlgorithmExecutorBridge::new().expect("Failed to create executor");

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
        let result = executor
            .execute_algorithm(algorithm, &parameters, "test_device")
            .unwrap_or_else(|_| panic!("Failed to execute {}", algorithm));

        assert_eq!(
            result.result_json["status"], "executed",
            "Status should be executed for {}",
            algorithm
        );
        assert_eq!(
            result.result_json["algorithm"], algorithm,
            "Algorithm name should match"
        );
        assert_eq!(
            result.result_json["source"], "cpp_plugins",
            "Source should be cpp_plugins for {}",
            algorithm
        );

        println!("✓ {} executed successfully", algorithm);
    }
}
