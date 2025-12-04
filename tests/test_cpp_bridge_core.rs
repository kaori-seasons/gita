//! 测试C++ FFI Bridge核心功能
//! 
//! 验证：
//! 1. CppAlgorithmExecutor初始化
//! 2. execute_algorithm核心方法调用
//! 3. 插件执行（vibrate31, error18, evaluation）
//! 4. 数据序列化和反序列化

use rust_edge_compute::ffi::bridge::CppAlgorithmExecutor;
use serde_json::json;
use std::collections::HashMap;

#[tokio::test]
async fn test_cpp_executor_initialization() {
    println!("\n=== 测试1: CppAlgorithmExecutor初始化 ===");
    
    // 创建执行器
    let mut executor = CppAlgorithmExecutor::new()
        .expect("Failed to create CppAlgorithmExecutor");
    
    // 初始化
    let init_result = executor.initialize()
        .expect("Failed to initialize");
    
    assert!(init_result, "Executor should initialize successfully");
    assert!(executor.is_initialized(), "Executor should be marked as initialized");
    
    println!("✓ 初始化成功");
}

#[tokio::test]
async fn test_vibrate31_plugin() {
    println!("\n=== 测试2: Vibrate31插件执行 ===");
    
    let mut executor = CppAlgorithmExecutor::new().unwrap();
    executor.initialize().unwrap();
    
    // 准备振动数据
    let input_data = json!({
        "wave_data": vec![1.0, 2.0, 3.0, 4.0, 5.0],
        "speed_data": vec![100.0, 150.0, 200.0],
        "sampling_rate": 1000,
        "device_id": "test_device_001"
    });
    
    let parameters = HashMap::new();
    
    // 执行vibrate31插件
    let result = executor.execute_plugin("vibrate31", input_data, parameters)
        .await
        .expect("Vibrate31 plugin execution failed");
    
    println!("Vibrate31结果: {}", serde_json::to_string_pretty(&result).unwrap());
    
    assert!(result.get("success").is_some() || result.get("status").is_some(), 
            "Result should contain success or status field");
    
    println!("✓ Vibrate31插件执行成功");
}

#[tokio::test]
async fn test_error18_plugin() {
    println!("\n=== 测试3: Error18插件执行 ===");
    
    let mut executor = CppAlgorithmExecutor::new().unwrap();
    executor.initialize().unwrap();
    
    // 准备特征数据
    let input_data = json!({
        "features": {
            "mean_hf": 0.5,
            "mean_lf": 0.3,
            "peak_freq": 1500.0
        },
        "device_id": "test_device_001"
    });
    
    let parameters = HashMap::new();
    
    // 执行error18插件
    let result = executor.execute_plugin("error18", input_data, parameters)
        .await
        .expect("Error18 plugin execution failed");
    
    println!("Error18结果: {}", serde_json::to_string_pretty(&result).unwrap());
    
    assert!(result.is_object(), "Result should be a JSON object");
    
    println!("✓ Error18插件执行成功");
}

#[tokio::test]
async fn test_evaluation_plugin() {
    println!("\n=== 测试4: Evaluation插件执行 ===");
    
    let mut executor = CppAlgorithmExecutor::new().unwrap();
    executor.initialize().unwrap();
    
    // 准备评估数据
    let input_data = json!({
        "health_score": 85.0,
        "error_count": 2,
        "status": "normal",
        "device_id": "test_device_001"
    });
    
    let parameters = HashMap::new();
    
    // 执行evaluation插件
    let result = executor.execute_plugin("evaluation", input_data, parameters)
        .await
        .expect("Evaluation plugin execution failed");
    
    println!("Evaluation结果: {}", serde_json::to_string_pretty(&result).unwrap());
    
    assert!(result.is_object(), "Result should be a JSON object");
    
    println!("✓ Evaluation插件执行成功");
}

#[tokio::test]
async fn test_get_available_plugins() {
    println!("\n=== 测试5: 获取可用插件列表 ===");
    
    let mut executor = CppAlgorithmExecutor::new().unwrap();
    executor.initialize().unwrap();
    
    let plugins = executor.get_available_plugins()
        .expect("Failed to get available plugins");
    
    println!("可用插件: {:?}", plugins);
    
    assert!(plugins.contains(&"vibrate31".to_string()), "Should include vibrate31");
    assert!(plugins.contains(&"error18".to_string()), "Should include error18");
    assert!(plugins.contains(&"evaluation".to_string()), "Should include evaluation");
    
    println!("✓ 插件列表获取成功");
}

#[tokio::test]
async fn test_multiple_sequential_calls() {
    println!("\n=== 测试6: 多次连续调用 ===");
    
    let mut executor = CppAlgorithmExecutor::new().unwrap();
    executor.initialize().unwrap();
    
    let input_data = json!({
        "wave_data": vec![1.0, 2.0, 3.0],
        "speed_data": vec![100.0, 150.0],
        "sampling_rate": 1000,
        "device_id": "test_device"
    });
    
    // 执行多次调用
    for i in 1..=3 {
        println!("  第{}次调用...", i);
        let result = executor.execute_plugin("vibrate31", input_data.clone(), HashMap::new())
            .await
            .expect(&format!("Call {} failed", i));
        
        assert!(result.is_object(), "Each call should return valid result");
    }
    
    println!("✓ 多次连续调用成功");
}
