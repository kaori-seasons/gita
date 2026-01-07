//! 跨语言调用集成测试
//!
//! 本测试套件验证Rust与C++之间的FFI通信是否正常工作
//! 包括初始化、数据转换、算法执行等全流程

use rust_edge_compute::ffi::bridge::CppAlgorithmExecutor;
use serde_json::json;
use std::collections::HashMap;

/// 测试1: 验证C++执行器初始化
#[tokio::test]
async fn test_cpp_executor_initialization() {
    println!("\n========================================");
    println!("测试1: C++执行器初始化");
    println!("========================================");

    match CppAlgorithmExecutor::new() {
        Ok(mut executor) => {
            println!("✓ 创建执行器成功");

            match executor.initialize() {
                Ok(result) => {
                    if result {
                        println!("✓ 初始化成功");
                        assert!(executor.is_initialized(), "执行器应该标记为已初始化");
                        println!("✓ 状态标记正确");
                    } else {
                        println!("✗ 初始化失败: initialize()返回false");
                        panic!("初始化失败");
                    }
                }
                Err(e) => {
                    println!("✗ 初始化错误: {}", e);
                    panic!("初始化过程出现错误: {}", e);
                }
            }
        }
        Err(e) => {
            println!("✗ 创建执行器失败: {}", e);
            panic!("创建执行器失败: {}", e);
        }
    }

    println!("✓ 测试1通过");
}

/// 测试2: 验证vibrate31插件调用
#[tokio::test]
async fn test_vibrate31_plugin_call() {
    println!("\n========================================");
    println!("测试2: Vibrate31插件调用");
    println!("========================================");

    let mut executor = CppAlgorithmExecutor::new().expect("创建执行器失败");
    executor.initialize().expect("初始化失败");
    println!("✓ 执行器初始化成功");

    // 准备测试数据
    let input_data = json!({
        "wave_data": vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0],
        "speed_data": vec![100.0, 150.0, 200.0, 250.0],
        "sampling_rate": 1000,
        "device_id": "test_device_001"
    });

    println!("📊 输入数据: {}", serde_json::to_string_pretty(&input_data).unwrap());

    let parameters = HashMap::new();

    // 执行vibrate31插件
    match executor.execute_plugin("vibrate31", input_data.clone(), parameters).await {
        Ok(result) => {
            println!("✓ Vibrate31执行成功");
            println!("📄 结果: {}", serde_json::to_string_pretty(&result).unwrap());

            // 验证结果结构
            assert!(
                result.is_object(),
                "结果应该是JSON对象"
            );
            println!("✓ 结果格式正确");
        }
        Err(e) => {
            println!("✗ Vibrate31执行失败: {}", e);
            panic!("Vibrate31执行失败: {}", e);
        }
    }

    println!("✓ 测试2通过");
}

/// 测试3: 验证数据序列化和反序列化
#[tokio::test]
async fn test_data_serialization() {
    println!("\n========================================");
    println!("测试3: 数据序列化/反序列化");
    println!("========================================");

    let input_data = json!({
        "wave_data": vec![1.0, 2.0, 3.0],
        "speed_data": vec![100.0, 150.0],
        "sampling_rate": 1000,
        "device_id": "device_123"
    });

    println!("📄 原始数据: {}", serde_json::to_string_pretty(&input_data).unwrap());

    // 序列化为字符串
    let serialized = serde_json::to_string(&input_data)
        .expect("序列化失败");
    println!("✓ 序列化成功 (长度: {} 字节)", serialized.len());

    // 反序列化回来
    let deserialized: serde_json::Value = serde_json::from_str(&serialized)
        .expect("反序列化失败");
    println!("✓ 反序列化成功");

    // 验证数据一致性
    assert_eq!(input_data, deserialized, "序列化/反序列化应该保持数据一致");
    println!("✓ 数据一致性验证通过");

    println!("✓ 测试3通过");
}

/// 测试4: 验证多次连续调用
#[tokio::test]
async fn test_multiple_sequential_calls() {
    println!("\n========================================");
    println!("测试4: 多次连续调用");
    println!("========================================");

    let mut executor = CppAlgorithmExecutor::new().expect("创建执行器失败");
    executor.initialize().expect("初始化失败");
    println!("✓ 执行器初始化成功");

    let input_data = json!({
        "wave_data": vec![1.0, 2.0, 3.0],
        "speed_data": vec![100.0, 150.0],
        "sampling_rate": 1000,
        "device_id": "test_device"
    });

    // 执行多次调用
    for i in 1..=3 {
        println!("\n第{}次调用...", i);
        match executor.execute_plugin("vibrate31", input_data.clone(), HashMap::new()).await {
            Ok(result) => {
                println!("✓ 第{}次调用成功", i);
                assert!(result.is_object(), "结果应该是JSON对象");
            }
            Err(e) => {
                println!("✗ 第{}次调用失败: {}", i, e);
                panic!("第{}次调用失败: {}", i, e);
            }
        }
    }

    println!("\n✓ 测试4通过");
}

/// 测试5: 验证错误处理
#[tokio::test]
async fn test_error_handling() {
    println!("\n========================================");
    println!("测试5: 错误处理");
    println!("========================================");

    let mut executor = CppAlgorithmExecutor::new().expect("创建执行器失败");
    executor.initialize().expect("初始化失败");
    println!("✓ 执行器初始化成功");

    // 测试未初始化执行器的调用
    let mut uninitialized_executor = CppAlgorithmExecutor::new().expect("创建执行器失败");
    let input_data = json!({"test": "data"});

    match uninitialized_executor.execute_plugin("vibrate31", input_data, HashMap::new()).await {
        Ok(_) => {
            println!("✗ 未初始化的执行器应该失败");
            panic!("未初始化的执行器应该返回错误");
        }
        Err(e) => {
            println!("✓ 正确捕获未初始化错误: {}", e);
        }
    }

    // 测试非存在的插件
    let input_data = json!({"test": "data"});
    match executor.execute_plugin("non_existent_plugin", input_data, HashMap::new()).await {
        Ok(result) => {
            // 可能返回错误结构，而不是Err
            println!("⚠ 非存在插件的结果: {}", serde_json::to_string_pretty(&result).unwrap());
        }
        Err(e) => {
            println!("✓ 正确捕获非存在插件错误: {}", e);
        }
    }

    println!("✓ 测试5通过");
}

/// 测试6: 性能测试
#[tokio::test]
async fn test_performance() {
    println!("\n========================================");
    println!("测试6: 性能测试");
    println!("========================================");

    let mut executor = CppAlgorithmExecutor::new().expect("创建执行器失败");
    executor.initialize().expect("初始化失败");
    println!("✓ 执行器初始化成功");

    let input_data = json!({
        "wave_data": vec![1.0, 2.0, 3.0, 4.0, 5.0],
        "speed_data": vec![100.0, 150.0, 200.0],
        "sampling_rate": 1000,
        "device_id": "test_device"
    });

    let start_time = std::time::Instant::now();
    let iterations = 5;

    for _ in 0..iterations {
        let _ = executor.execute_plugin("vibrate31", input_data.clone(), HashMap::new()).await;
    }

    let elapsed = start_time.elapsed();
    let avg_time = elapsed.as_millis() / iterations as u128;

    println!("执行{}次调用耗时: {:?}ms", iterations, elapsed.as_millis());
    println!("平均每次调用耗时: {}ms", avg_time);

    assert!(avg_time < 5000, "单次调用不应超过5000ms");
    println!("✓ 性能测试通过");

    println!("✓ 测试6通过");
}

/// 综合测试：完整的FFI流程
#[tokio::test]
async fn test_complete_ffi_workflow() {
    println!("\n========================================");
    println!("综合测试: 完整FFI流程");
    println!("========================================");

    println!("\n【步骤1】创建执行器");
    let mut executor = match CppAlgorithmExecutor::new() {
        Ok(exec) => {
            println!("✓ 执行器创建成功");
            exec
        }
        Err(e) => {
            println!("✗ 执行器创建失败: {}", e);
            panic!("创建执行器失败");
        }
    };

    println!("\n【步骤2】初始化执行器");
    match executor.initialize() {
        Ok(true) => println!("✓ 初始化成功"),
        _ => panic!("初始化失败"),
    }

    println!("\n【步骤3】获取可用插件列表");
    match executor.get_available_plugins() {
        Ok(plugins) => {
            println!("✓ 获取插件列表成功");
            for plugin in &plugins {
                println!("  - {}", plugin);
            }
        }
        Err(e) => println!("⚠ 获取插件列表失败: {}", e),
    }

    println!("\n【步骤4】准备测试数据");
    let test_cases = vec![
        ("vibrate31", json!({
            "wave_data": vec![1.0, 2.0, 3.0, 4.0, 5.0],
            "speed_data": vec![100.0, 150.0, 200.0],
            "sampling_rate": 1000,
            "device_id": "device_001"
        })),
        ("error18", json!({
            "features": {
                "mean_hf": 0.5,
                "mean_lf": 0.3,
                "peak_freq": 1500.0
            },
            "device_id": "device_001"
        })),
    ];

    println!("✓ 准备了{}个测试用例", test_cases.len());

    println!("\n【步骤5】执行插件调用");
    for (plugin_name, input_data) in test_cases {
        println!("\n调用插件: {}", plugin_name);
        match executor.execute_plugin(plugin_name, input_data, HashMap::new()).await {
            Ok(result) => {
                println!("✓ 执行成功");
                println!("  结果类型: {}", if result.is_object() { "对象" } else { "其他" });
            }
            Err(e) => {
                println!("⚠ 执行失败: {}", e);
            }
        }
    }

    println!("\n【步骤6】验证执行状态");
    assert!(executor.is_initialized(), "执行器应该保持初始化状态");
    println!("✓ 执行器状态正常");

    println!("\n✓ 综合测试通过");
}

/// 测试跨语言调用的内存安全性
#[tokio::test]
async fn test_memory_safety() {
    println!("\n========================================");
    println!("测试7: 内存安全性");
    println!("========================================");

    let mut executor = CppAlgorithmExecutor::new().expect("创建执行器失败");
    executor.initialize().expect("初始化失败");
    println!("✓ 执行器初始化成功");

    // 创建大量数据测试内存管理
    let large_wave_data: Vec<f64> = (0..10000).map(|i| (i as f64).sin()).collect();
    let input_data = json!({
        "wave_data": large_wave_data,
        "speed_data": vec![100.0, 150.0, 200.0],
        "sampling_rate": 1000,
        "device_id": "test_device"
    });

    println!("📊 输入数据大小: ~40KB");

    match executor.execute_plugin("vibrate31", input_data, HashMap::new()).await {
        Ok(result) => {
            println!("✓ 大数据处理成功");
            assert!(result.is_object(), "结果应该是对象");
        }
        Err(e) => {
            println!("✗ 大数据处理失败: {}", e);
            panic!("大数据处理失败: {}", e);
        }
    }

    println!("✓ 内存安全性测试通过");
}
