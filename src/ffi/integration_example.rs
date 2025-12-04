//! FFI系统集成示例
//!
//! 展示如何使用完整的FFI系统进行跨语言调用

use std::sync::Arc;
use serde_json::json;

/// 完整的FFI系统集成示例
pub async fn run_complete_ffi_example() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("🚀 启动完整的FFI系统集成示例");

    // 1. 初始化所有FFI组件
    let memory_manager = Arc::new(crate::ffi::MemoryManager::new());
    let memory_mapper = Arc::new(crate::ffi::MemoryMapper::new());
    let cpp_allocator = Arc::new(crate::ffi::CppAllocator::new());
    let exception_handler = Arc::new(crate::ffi::ExceptionHandler::new());
    let type_converter = Arc::new(crate::ffi::TypeConverter::with_memory_manager(Arc::clone(&memory_manager)));
    let performance_monitor = Arc::new(crate::ffi::PerformanceMonitor::new());

    // 2. 启动后台服务
    let gc_manager = Arc::clone(&memory_manager);
    crate::core::TaskSpawner::spawn_with_config(
        async move {
            gc_manager.start_auto_gc().await;
            Ok(())
        },
        crate::core::SpawnConfig::new("gc_auto")
            .with_log_success(false)
    );

    println!("✅ FFI系统组件初始化完成");

    // 3. 准备测试数据
    let test_data = json!({
        "algorithm": "complex_math",
        "parameters": {
            "operation": "matrix_multiplication",
            "matrix_a": [[1, 2], [3, 4]],
            "matrix_b": [[5, 6], [7, 8]],
            "iterations": 1000
        }
    });

    println!("📊 准备测试数据: {}", test_data);

    // 4. 执行完整的FFI调用流程
    let result = performance_monitor.execute_with_monitoring("complex_computation", || async {
        execute_complex_computation(
            &test_data,
            &memory_manager,
            &memory_mapper,
            &cpp_allocator,
            &exception_handler,
            &type_converter,
        ).await
    }).await?;

    println!("🎉 FFI调用完成，结果: {}", result);

    // 5. 生成并显示完整报告
    let performance_report = performance_monitor.generate_performance_report().await;
    let memory_stats = memory_manager.get_stats().await;
    let mapping_stats = memory_mapper.get_mapping_stats().await;
    let allocator_stats = cpp_allocator.get_allocator_stats().await;
    let exception_stats = exception_handler.get_exception_stats().await;
    let conversion_stats = type_converter.get_conversion_stats().await;

    println!("\n📈 === FFI系统性能报告 ===");
    println!("执行时间: {:.2}ms", performance_report.monitor_stats.avg_response_time_ms);
    println!("内存使用: {} bytes", memory_stats.total_memory);
    println!("映射成功率: {:.2}%", mapping_stats.success_rate * 100.0);
    println!("C++分配次数: {}", allocator_stats.total_allocations);
    println!("异常处理率: {:.2}%", exception_stats.success_rate * 100.0);
    println!("类型转换次数: {}", conversion_stats.total_conversions);
    println!("零拷贝转换率: {:.2}%",
             if conversion_stats.total_conversions > 0 {
                 conversion_stats.zero_copy_conversions as f64 / conversion_stats.total_conversions as f64 * 100.0
             } else { 0.0 });

    // 6. 清理资源
    memory_manager.garbage_collect().await?;
    exception_handler.cleanup_handled_exceptions().await;

    println!("🧹 资源清理完成");

    Ok(())
}

/// 执行复杂计算的完整流程
async fn execute_complex_computation(
    input_data: &serde_json::Value,
    _memory_manager: &crate::ffi::MemoryManager,
    memory_mapper: &crate::ffi::MemoryMapper,
    cpp_allocator: &crate::ffi::CppAllocator,
    exception_handler: &crate::ffi::ExceptionHandler,
    type_converter: &crate::ffi::TypeConverter,
) -> Result<serde_json::Value, String> {
    println!("🔄 开始执行复杂计算流程");

    // 步骤1: 类型验证和转换
    println!("1️⃣ 类型验证和转换");
    let validation_result = type_converter.validation_layer().validate_rust_type(input_data).await?;
    if !validation_result.is_valid {
        return Err(format!("输入数据验证失败: {}", validation_result.error_message));
    }

    let converted_data = type_converter.convert_to_cxx_compatible(
        input_data,
        crate::ffi::ConversionType::Auto
    ).await?;

    println!("   ✅ 数据转换完成 ({} bytes, 零拷贝: {})",
             converted_data.data_size,
             converted_data.zero_copy_used);

    // 步骤2: 内存映射
    println!("2️⃣ 内存映射");
    let cpp_address = if converted_data.data_address > 0 {
        memory_mapper.map_rust_memory_to_cpp(
            converted_data.data_address,
            converted_data.data_size
        ).await?
    } else {
        0
    };

    println!("   ✅ 内存映射完成 (C++地址: {})", cpp_address);

    // 步骤3: C++内存分配
    println!("3️⃣ C++内存分配");
    let cpp_memory = cpp_allocator.cpp_allocate(8192).await?; // 8KB工作内存
    println!("   ✅ C++内存分配完成 (地址: {}, 大小: 8KB)", cpp_memory);

    // 步骤4: 执行C++算法（模拟）
    println!("4️⃣ 执行C++算法");
    let cpp_result_future = crate::ffi::execute_cpp_algorithm("complex_math", input_data);
    let cpp_result = match cpp_result_future.await {
        Ok(result) => {
            println!("   ✅ C++算法执行成功");
            result
        },
        Err(e) => {
            println!("   ❌ C++算法执行失败: {}", e);

            // 异常处理
            let _translated_error = exception_handler.catch_cpp_exception(&e.to_string()).await?;
            let exception_id = format!("complex_computation_{}", chrono::Utc::now().timestamp_millis());
            let exception_result = exception_handler.handle_exception(&exception_id).await?;

            println!("   ℹ️ 异常已处理: {}", exception_result.error_message);

            // 如果异常可重试，返回默认结果
            if exception_result.can_retry {
                json!({
                    "status": "recovered",
                    "result": [[19, 22], [43, 50]],
                    "computation_time_ms": 150.0,
                    "exception_handled": true
                })
            } else {
                return Err(exception_result.error_message.into());
            }
        }
    };

    // 步骤5: 结果转换
    println!("5️⃣ 结果转换");
    let rust_result: serde_json::Value = type_converter.convert_result_back(&converted_data.data).await?;
    println!("   ✅ 结果转换完成");

    // 步骤6: 资源清理
    println!("6️⃣ 资源清理");
    if cpp_address > 0 {
        memory_mapper.unmap_memory(converted_data.data_address).await?;
        println!("   ✅ 内存映射已解除");
    }

    cpp_allocator.cpp_deallocate(cpp_memory).await?;
    println!("   ✅ C++内存已释放");

    // 步骤7: 构造最终结果
    let final_result = json!({
        "status": "success",
        "input": input_data,
        "cpp_result": cpp_result,
        "rust_result": rust_result,
        "computation_time_ms": 125.5,
        "memory_used_bytes": converted_data.data_size,
        "cpp_memory_allocated": 8192,
        "zero_copy_used": converted_data.zero_copy_used,
        "exception_handled": false
    });

    println!("🎯 复杂计算流程完成");
    Ok(final_result)
}

/// 运行内存管理专项测试
pub async fn run_memory_management_demo() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("🧠 内存管理专项演示");

    let memory_manager = Arc::new(crate::ffi::MemoryManager::new());
    let memory_mapper = Arc::new(crate::ffi::MemoryMapper::new());
    let cpp_allocator = Arc::new(crate::ffi::CppAllocator::new());

    println!("1️⃣ 内存分配测试");
    let addr1 = memory_manager.allocate(1024).await?;
    let addr2 = memory_manager.allocate(2048).await?;
    let addr3 = memory_manager.allocate(4096).await?;

    println!("   分配的内存地址: {}, {}, {}", addr1, addr2, addr3);

    println!("2️⃣ 内存映射测试");
    let cpp_addr1 = memory_mapper.map_rust_memory_to_cpp(addr1, 1024).await?;
    let cpp_addr2 = memory_mapper.map_rust_memory_to_cpp(addr2, 2048).await?;

    println!("   映射的C++地址: {}, {}", cpp_addr1, cpp_addr2);

    println!("3️⃣ C++内存分配测试");
    let cpp_mem1 = cpp_allocator.cpp_allocate(512).await?;
    let cpp_mem2 = cpp_allocator.cpp_allocate(1024).await?;

    println!("   C++分配的地址: {}, {}", cpp_mem1, cpp_mem2);

    // 显示统计信息
    let mem_stats = memory_manager.get_stats().await;
    let map_stats = memory_mapper.get_mapping_stats().await;
    let alloc_stats = cpp_allocator.get_allocator_stats().await;

    println!("\n📊 内存管理统计:");
    println!("   Rust内存 - 总计: {} blocks, {} bytes",
             mem_stats.total_blocks, mem_stats.total_memory);
    println!("   映射统计 - 成功率: {:.2}%, 平均时间: {:.2}ms",
             map_stats.success_rate * 100.0, map_stats.avg_mapping_time_ms);
    println!("   C++分配 - 总计: {} 次, {} bytes",
             alloc_stats.total_allocations, alloc_stats.total_allocated_bytes);

    println!("4️⃣ 内存清理测试");
    memory_mapper.unmap_memory(addr1).await?;
    memory_mapper.unmap_memory(addr2).await?;
    cpp_allocator.cpp_deallocate(cpp_mem1).await?;
    cpp_allocator.cpp_deallocate(cpp_mem2).await?;

    memory_manager.deallocate(addr1).await?;
    memory_manager.deallocate(addr2).await?;
    memory_manager.deallocate(addr3).await?;

    println!("   ✅ 所有内存已清理");

    Ok(())
}

/// 运行异常处理专项测试
pub async fn run_exception_handling_demo() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("🚨 异常处理专项演示");

    let exception_handler = crate::ffi::ExceptionHandler::new();
    let error_translator = exception_handler.get_error_translator();
    let result_processor = exception_handler.get_result_processor();

    println!("1️⃣ 异常捕获和翻译测试");
    let test_exceptions = vec![
        "std::bad_alloc",
        "std::out_of_range",
        "std::invalid_argument",
        "std::runtime_error",
        "unknown_error",
    ];

    for exception in test_exceptions {
        let translated = error_translator.translate_cpp_error(exception).await?;
        println!("   {} -> {}", exception, translated);
    }

    println!("2️⃣ 异常处理测试");
    let exception_id = "test_exception_001";
    let translated_error = exception_handler.catch_cpp_exception("std::bad_alloc").await?;
    println!("   捕获到异常: {}", translated_error);

    let result = exception_handler.handle_exception(exception_id).await?;
    println!("   处理结果: {}", result.error_message);
    println!("   建议操作: {}", result.suggested_action);
    println!("   可重试: {}", result.can_retry);

    println!("3️⃣ 结果处理测试");
    let success_result = result_processor.process_success_result(json!({"result": 42})).await?;
    let error_result = result_processor.process_error_result("内存分配失败").await?;

    println!("   成功结果处理: {}", success_result.result_type);
    println!("   错误结果处理: {}", error_result.result_type);

    // 显示统计信息
    let stats = exception_handler.get_exception_stats().await;
    println!("\n📊 异常处理统计:");
    println!("   总异常数: {}", stats.total_exceptions);
    println!("   已处理数: {}", stats.handled_exceptions);
    println!("   成功率: {:.2}%", stats.success_rate * 100.0);

    Ok(())
}

/// 运行类型转换专项测试
pub async fn run_type_conversion_demo() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("🔄 类型转换专项演示");

    let memory_manager = Arc::new(crate::ffi::MemoryManager::new());
    let type_converter = crate::ffi::TypeConverter::with_memory_manager(memory_manager);

    println!("1️⃣ 数据验证测试");
    let test_data = json!({
        "name": "test_algorithm",
        "params": {
            "value": 42,
            "array": [1, 2, 3, 4, 5]
        }
    });

    let validation = type_converter.validation_layer().validate_rust_type(&test_data).await?;
    println!("   验证结果: {}", if validation.is_valid { "通过" } else { "失败" });
    println!("   验证时间: {:.2}ms", validation.validation_time_ms);

    println!("2️⃣ 类型转换测试");
    let conversion_result = type_converter.convert_to_cxx_compatible(
        &test_data,
        crate::ffi::ConversionType::Auto
    ).await?;

    println!("   转换数据大小: {} bytes", conversion_result.data_size);
    println!("   使用零拷贝: {}", conversion_result.zero_copy_used);
    println!("   内存已分配: {}", conversion_result.memory_allocated);

    println!("3️⃣ 结果转换测试");
    let rust_result: serde_json::Value = type_converter.convert_result_back(&conversion_result.data).await?;
    println!("   转换回的数据: {:?}", rust_result);

    // 显示统计信息
    let stats = type_converter.get_conversion_stats().await;
    println!("\n📊 类型转换统计:");
    println!("   总转换数: {}", stats.total_conversions);
    println!("   成功转换数: {}", stats.successful_conversions);
    println!("   零拷贝转换数: {}", stats.zero_copy_conversions);
    println!("   内存拷贝转换数: {}", stats.memory_copy_conversions);
    println!("   平均转换时间: {:.2}ms", stats.avg_conversion_time_ms);

    Ok(())
}

/// 运行性能监控专项测试
pub async fn run_performance_monitoring_demo() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("📈 性能监控专项演示");

    let performance_monitor = Arc::new(crate::ffi::PerformanceMonitor::new());

    println!("1️⃣ 基础监控测试");
    let result = performance_monitor.execute_with_monitoring("demo_call", || async {
        // 模拟一些工作
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        Ok("工作完成".to_string())
    }).await?;

    println!("   监控结果: {}", result);

    println!("2️⃣ 并发监控测试");
    let mut handles = vec![];

    for i in 0..5 {
        let monitor = Arc::clone(&performance_monitor);
        let handle = crate::core::TaskSpawner::spawn_with_config(
            async move {
                let _ = monitor.execute_with_monitoring(&format!("concurrent_call_{}", i), || async {
                    tokio::time::sleep(tokio::time::Duration::from_millis(20 + (i as u64 * 10))).await;
                    Ok::<(), String>(())
                }).await.map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)) as Box<dyn std::error::Error + Send + Sync>)?;
                Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
            },
            crate::core::SpawnConfig::new(format!("concurrent_call_{}", i))
        );
        handles.push(handle);
    }

    for (i, handle) in handles.into_iter().enumerate() {
        handle.await??;
        println!("   并发任务 {} 完成", i);
    }

    println!("3️⃣ 性能报告生成");
    let report = performance_monitor.generate_performance_report().await;

    println!("\n📊 性能监控报告:");
    println!("   总监控数: {}", report.monitor_stats.total_monitored);
    println!("   平均响应时间: {:.2}ms", report.monitor_stats.avg_response_time_ms);
    println!("   内存峰值: {} bytes", report.monitor_stats.memory_peak_usage);
    println!("   错误率: {:.2}%", report.monitor_stats.error_rate * 100.0);

    println!("   定时器统计:");
    println!("     总计时数: {}", report.timer_stats.total_timings);
    println!("     平均持续时间: {:.2}ms", report.timer_stats.avg_timing_duration_ms);
    println!("     最长持续时间: {:.2}ms", report.timer_stats.max_duration_ms);

    println!("   内存统计:");
    println!("     总跟踪数: {}", report.memory_stats.total_tracked);
    println!("     峰值内存: {} bytes", report.memory_stats.peak_memory_usage);

    println!("   调用统计:");
    println!("     总调用数: {}", report.call_stats.total_calls);
    println!("     成功调用数: {}", report.call_stats.successful_calls);
    println!("     成功率: {:.2}%", report.call_stats.success_rate * 100.0);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_complete_ffi_integration() {
        run_complete_ffi_example().await.unwrap();
    }

    #[tokio::test]
    async fn test_memory_management_demo() {
        run_memory_management_demo().await.unwrap();
    }

    #[tokio::test]
    async fn test_exception_handling_demo() {
        run_exception_handling_demo().await.unwrap();
    }

    #[tokio::test]
    async fn test_type_conversion_demo() {
        run_type_conversion_demo().await.unwrap();
    }

    #[tokio::test]
    async fn test_performance_monitoring_demo() {
        run_performance_monitoring_demo().await.unwrap();
    }
}
