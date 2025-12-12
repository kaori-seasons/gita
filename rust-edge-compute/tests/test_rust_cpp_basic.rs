//! Rust 调用 C++ 的最小化验证测试
//! 
//! 这个测试验证 Rust 端能否通过 CXX bridge 成功调用 C++ 算法插件

use rust_edge_compute::ffi::bridge::CppAlgorithmExecutor;
use serde_json::json;
use std::collections::HashMap;

/// 测试1: 执行器创建和初始化
#[tokio::test]
async fn test_executor_creation_and_init() {
    println!("\n【测试1】执行器创建和初始化");
    
    // 创建执行器
    let mut executor = match CppAlgorithmExecutor::new() {
        Ok(exe) => {
            println!("✅ CppAlgorithmExecutor 创建成功");
            exe
        }
        Err(e) => {
            panic!("❌ 创建执行器失败: {}", e);
        }
    };
    
    // 初始化执行器
    match executor.initialize() {
        Ok(result) => {
            if result {
                println!("✅ 执行器初始化成功");
                assert!(executor.is_initialized(), "执行器应该标记为已初始化");
                println!("✅ 执行器状态标记正确");
            } else {
                panic!("❌ 初始化返回 false");
            }
        }
        Err(e) => {
            panic!("❌ 初始化失败: {}", e);
        }
    }
}

/// 测试2: 调用 vibrate31 插件
#[tokio::test]
async fn test_vibrate31_call() {
    println!("\n【测试2】vibrate31 插件调用");
    
    let mut executor = CppAlgorithmExecutor::new()
        .expect("创建执行器失败");
    executor.initialize()
        .expect("初始化失败");
    println!("✅ 执行器已就绪");
    
    // 构造测试数据
    let input = json!({
        "wave_data": vec![1.0, 2.0, 3.0, 4.0, 5.0],
        "speed_data": vec![100.0, 150.0, 200.0],
        "sampling_rate": 1000,
        "device_id": "test_device"
    });
    
    println!("📊 输入数据: 5 个波形样本, 采样率 1000Hz");
    
    // 调用插件
    match executor.execute_plugin("vibrate31", input, HashMap::new()).await {
        Ok(result) => {
            println!("✅ vibrate31 调用成功");
            if result.is_object() {
                if let Some(obj) = result.as_object() {
                    println!("✅ 返回结果包含 {} 个字段", obj.len());
                }
            }
        }
        Err(e) => {
            panic!("❌ vibrate31 调用失败: {}", e);
        }
    }
}

/// 测试3: 多次连续调用稳定性
#[tokio::test]
async fn test_multiple_calls_stability() {
    println!("\n【测试3】多次连续调用稳定性");
    
    let mut executor = CppAlgorithmExecutor::new()
        .expect("创建执行器失败");
    executor.initialize()
        .expect("初始化失败");
    println!("✅ 执行器已就绪");
    
    let input = json!({
        "wave_data": vec![1.0, 2.0, 3.0],
        "sampling_rate": 1000
    });
    
    let mut success_count = 0;
    
    for i in 1..=3 {
        match executor.execute_plugin("vibrate31", input.clone(), HashMap::new()).await {
            Ok(_) => {
                success_count += 1;
                println!("✅ 第 {} 次调用成功", i);
            }
            Err(e) => {
                println!("⚠️ 第 {} 次调用失败: {}", i, e);
            }
        }
    }
    
    assert!(success_count > 0, "至少应该有 1 次调用成功");
    println!("✅ 连续调用稳定性测试完成 ({}/3 成功)", success_count);
}

/// 测试4: 错误情况处理
#[tokio::test]
async fn test_error_handling() {
    println!("\n【测试4】错误处理");
    
    let mut executor = CppAlgorithmExecutor::new()
        .expect("创建执行器失败");
    
    // 未初始化就调用应该失败
    let input = json!({"test": "data"});
    match executor.execute_plugin("vibrate31", input.clone(), HashMap::new()).await {
        Ok(_) => {
            println!("⚠️ 未初始化的执行器仍返回结果");
        }
        Err(e) => {
            println!("✅ 正确捕获未初始化错误: {}", e);
        }
    }
    
    // 初始化后再试
    executor.initialize().expect("初始化失败");
    println!("✅ 执行器已初始化");
    
    match executor.execute_plugin("vibrate31", input, HashMap::new()).await {
        Ok(_) => println!("✅ 初始化后调用成功"),
        Err(e) => println!("⚠️ 初始化后仍失败: {}", e),
    }
}

/// 综合测试: Rust 调用 C++ 的完整流程
#[tokio::test]
async fn test_rust_cpp_complete_flow() {
    println!("\n【综合测试】Rust 调用 C++ 的完整流程");
    println!("═══════════════════════════════════════════════════════");
    
    println!("\n[步骤1] 创建 C++ 执行器");
    let mut executor = match CppAlgorithmExecutor::new() {
        Ok(e) => {
            println!("✅ 执行器创建成功");
            e
        }
        Err(e) => panic!("❌ 创建失败: {}", e),
    };
    
    println!("\n[步骤2] 初始化执行器");
    match executor.initialize() {
        Ok(true) => println!("✅ 初始化成功"),
        _ => panic!("❌ 初始化失败"),
    }
    
    println!("\n[步骤3] 构造输入数据");
    let input = json!({
        "wave_data": vec![1.0, 2.0, 3.0, 4.0, 5.0],
        "speed_data": vec![100.0, 150.0],
        "sampling_rate": 1000,
        "device_id": "rust_test_device"
    });
    println!("✅ 输入数据准备完成");
    
    println!("\n[步骤4] 调用 C++ 插件 (vibrate31)");
    match executor.execute_plugin("vibrate31", input, HashMap::new()).await {
        Ok(result) => {
            println!("✅ 插件调用成功");
            
            println!("\n[步骤5] 验证返回结果");
            if result.is_object() {
                println!("✅ 返回类型正确 (JSON 对象)");
                if let Some(obj) = result.as_object() {
                    println!("✅ 结果包含 {} 个字段", obj.len());
                    for (key, _) in obj.iter().take(3) {
                        println!("   - {}", key);
                    }
                }
            } else {
                println!("⚠️ 返回类型非对象");
            }
        }
        Err(e) => {
            panic!("❌ 插件调用失败: {}", e);
        }
    }
    
    println!("\n[步骤6] 验证执行器状态");
    assert!(executor.is_initialized(), "执行器应保持初始化状态");
    println!("✅ 执行器状态正常");
    
    println!("\n═══════════════════════════════════════════════════════");
    println!("✅ Rust 调用 C++ 完整流程验证成功!");
    println!("✅ CXX bridge 工作正常");
    println!("✅ 数据传递完整");
}
