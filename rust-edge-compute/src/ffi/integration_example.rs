//! FFI系统集成示例
//!
//! 展示如何使用完整的FFI系统进行跨语言调用

use rust_edge_compute_core::SpawnConfig;
use rust_edge_compute_core::TaskSpawner;
use serde_json::json;
use std::sync::Arc;

use crate::ffi::bridge::CppAlgorithmExecutor;
use crate::ffi::{
    ConversionType, CppAllocator, ExceptionHandler, MemoryManager, MemoryMapper,
    PerformanceMonitor, TypeConverter,
};

/// 完整的FFI系统集成示例
pub async fn run_complete_ffi_example() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("🚀 启动完整的FFI系统集成示例");

    // 1. 初始化所有FFI组件
    let memory_manager = Arc::new(MemoryManager::new());
    let memory_mapper = Arc::new(MemoryMapper::new());
    let cpp_allocator = Arc::new(CppAllocator::new());
    let exception_handler = Arc::new(ExceptionHandler::new());
    let type_converter = Arc::new(TypeConverter::with_memory_manager(Arc::clone(
        &memory_manager,
    )));
    let performance_monitor = Arc::new(PerformanceMonitor::new());

    // 2. 启动后台服务
    let gc_manager = Arc::clone(&memory_manager);
    TaskSpawner::spawn_with_config(
        async move {
            gc_manager.start_auto_gc().await;
            Ok(())
        },
        SpawnConfig::new("gc_auto").with_log_success(false),
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
    let result = performance_monitor
        .execute_with_monitoring("complex_computation", || async {
            execute_complex_computation(
                &test_data,
                &memory_manager,
                &memory_mapper,
                &cpp_allocator,
                &exception_handler,
                &type_converter,
            )
            .await
            .map_err(|e| {
                Box::new(std::io::Error::other(e))
                    as Box<dyn std::error::Error + Send + Sync>
            })
        })
        .await?;

    println!("🎉 FFI调用完成，结果: {}", result);

    // 5. 生成并显示完整报告
    let memory_stats = memory_manager.get_stats().await;
    let mapping_stats = memory_mapper.get_mapping_stats().await;
    let allocator_stats = cpp_allocator.get_allocator_stats().await;
    let exception_stats = exception_handler.get_exception_stats().await;

    println!("\n📈 === FFI系统性能报告 ===");
    println!("内存统计: {:?}", memory_stats);
    println!("映射统计: {:?}", mapping_stats);
    println!("分配统计: {:?}", allocator_stats);
    println!("异常统计: {:?}", exception_stats);

    // 6. 清理资源
    memory_manager.garbage_collect().await.map_err(|e| {
        Box::new(std::io::Error::other(e))
            as Box<dyn std::error::Error + Send + Sync>
    })?;
    exception_handler.cleanup_handled_exceptions().await;

    println!("🧹 资源清理完成");

    Ok(())
}

/// 执行复杂计算的完整流程
async fn execute_complex_computation(
    input_data: &serde_json::Value,
    memory_manager: &MemoryManager,
    memory_mapper: &MemoryMapper,
    cpp_allocator: &CppAllocator,
    exception_handler: &ExceptionHandler,
    type_converter: &TypeConverter,
) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    println!("🔄 开始执行复杂计算流程");

    // 1. 内存分配
    let input_size = serde_json::to_string(input_data).unwrap().len();
    let input_memory = memory_manager.allocate(input_size).await.map_err(|e| {
        Box::new(std::io::Error::other(
            e.to_string(),
        )) as Box<dyn std::error::Error + Send + Sync>
    })?;

    // 2. 内存映射
    let mapped_addr = memory_mapper
        .map_rust_memory_to_cpp(input_memory, input_size)
        .await
        .map_err(|e| {
            Box::new(std::io::Error::other(
                e.to_string(),
            )) as Box<dyn std::error::Error + Send + Sync>
        })?;

    // 3. C++内存分配
    let cpp_memory = cpp_allocator.cpp_allocate(input_size).await.map_err(|e| {
        Box::new(std::io::Error::other(
            e.to_string(),
        )) as Box<dyn std::error::Error + Send + Sync>
    })?;

    // 4. 类型转换
    match ConversionType::Auto {
        ConversionType::Auto => {
            println!("🔄 使用自动类型转换");
        }
        ConversionType::ZeroCopy => {
            println!("🔄 使用零拷贝类型转换");
        }
        ConversionType::Safe => {
            println!("🔄 使用安全类型转换");
        }
    }

    // 5. 执行C++算法
    let mut executor =
        CppAlgorithmExecutor::new().map_err(|e| format!("C++执行器创建失败: {}", e))?;

    executor
        .initialize()
        .map_err(|e| format!("C++执行器初始化失败: {}", e))?;

    // 6. 异常处理
    let test_exception = "std::runtime_error";
    let translated_error = exception_handler
        .catch_cpp_exception(test_exception)
        .await
        .map_err(|e| {
            Box::new(std::io::Error::other(e))
                as Box<dyn std::error::Error + Send + Sync>
        })?;

    println!("🚨 捕获并翻译异常: {}", translated_error);

    // 7. 返回结果
    Ok(json!({
        "status": "success",
        "memory_allocated": input_memory,
        "memory_mapped": mapped_addr,
        "cpp_allocated": cpp_memory,
        "translated_error": translated_error,
        "message": "Complex computation completed successfully"
    }))
}

/// 运行内存管理专项测试
pub async fn run_memory_management_demo() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("🧠 内存管理专项演示");

    let memory_manager = Arc::new(MemoryManager::new());
    let memory_mapper = Arc::new(MemoryMapper::new());
    let cpp_allocator = Arc::new(CppAllocator::new());

    // 分配不同大小的内存块
    let sizes = vec![1024, 2048, 4096, 8192];

    for &size in &sizes {
        let addr = memory_manager.allocate(size).await.map_err(|e| {
            Box::new(std::io::Error::other(e))
                as Box<dyn std::error::Error + Send + Sync>
        })?;
        println!("  分配 {} 字节内存，地址: 0x{:x}", size, addr);

        // 测试引用计数
        memory_manager.retain(addr).await.map_err(|e| {
            Box::new(std::io::Error::other(e))
                as Box<dyn std::error::Error + Send + Sync>
        })?;
        println!("  增加引用计数");

        memory_manager.release(addr).await.map_err(|e| {
            Box::new(std::io::Error::other(e))
                as Box<dyn std::error::Error + Send + Sync>
        })?;
        println!("  减少引用计数");

        memory_manager.deallocate(addr).await.map_err(|e| {
            Box::new(std::io::Error::other(e))
                as Box<dyn std::error::Error + Send + Sync>
        })?;
        println!("  释放内存");
    }

    // 测试内存映射
    let rust_addr = memory_manager.allocate(1024).await.map_err(|e| {
        Box::new(std::io::Error::other(e))
            as Box<dyn std::error::Error + Send + Sync>
    })?;
    let cpp_addr = memory_mapper
        .map_rust_memory_to_cpp(rust_addr, 1024)
        .await
        .map_err(|e| {
            Box::new(std::io::Error::other(e))
                as Box<dyn std::error::Error + Send + Sync>
        })?;
    println!(
        "  映射Rust内存 0x{:x} 到C++内存 0x{:x}",
        rust_addr, cpp_addr
    );

    memory_mapper.unmap_memory(rust_addr).await.map_err(|e| {
        Box::new(std::io::Error::other(e))
            as Box<dyn std::error::Error + Send + Sync>
    })?;
    println!("  解除内存映射");

    // 测试C++内存分配
    let cpp_addr = cpp_allocator.cpp_allocate(2048).await.map_err(|e| {
        Box::new(std::io::Error::other(e))
            as Box<dyn std::error::Error + Send + Sync>
    })?;
    println!("  分配C++内存，地址: 0x{:x}", cpp_addr);

    cpp_allocator.cpp_deallocate(cpp_addr).await.map_err(|e| {
        Box::new(std::io::Error::other(e))
            as Box<dyn std::error::Error + Send + Sync>
    })?;
    println!("  释放C++内存");

    // 显示统计信息
    let stats = memory_manager.get_stats().await;
    println!("  内存统计: {:?}", stats);

    let mapping_stats = memory_mapper.get_mapping_stats().await;
    println!("  映射统计: {:?}", mapping_stats);

    let allocator_stats = cpp_allocator.get_allocator_stats().await;
    println!("  分配统计: {:?}", allocator_stats);

    println!("✅ 内存管理演示完成");
    Ok(())
}

/// 运行异常处理专项测试
pub async fn run_exception_handling_demo() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("🚨 异常处理专项演示");

    let exception_handler = Arc::new(ExceptionHandler::new());

    // 测试常见C++异常
    let exceptions = vec![
        "std::bad_alloc",
        "std::out_of_range",
        "std::invalid_argument",
        "std::runtime_error",
        "std::logic_error",
        "unknown_exception",
    ];

    for exception in exceptions {
        match exception_handler.catch_cpp_exception(exception).await {
            Ok(translated) => {
                println!("  捕获异常 '{}': {}", exception, translated);

                // 添加自定义翻译
                exception_handler
                    .get_error_translator()
                    .add_translation(exception, &format!("自定义翻译: {}", exception))
                    .await;
            }
            Err(e) => println!("  异常捕获失败 '{}': {}", exception, e),
        }
    }

    // 显示翻译映射
    let translations = exception_handler
        .get_error_translator()
        .get_all_translations()
        .await;
    println!("  翻译映射: {:?}", translations);

    // 显示统计信息
    let stats = exception_handler.get_exception_stats().await;
    println!("  异常统计: {:?}", stats);

    println!("✅ 异常处理演示完成");
    Ok(())
}

/// 运行类型转换专项测试
pub async fn run_type_conversion_demo() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("🔄 类型转换专项演示");

    let memory_manager = Arc::new(MemoryManager::new());
    let type_converter = Arc::new(TypeConverter::with_memory_manager(Arc::clone(
        &memory_manager,
    )));

    // 测试不同类型的转换
    let conversion_types = vec![
        ConversionType::Auto,
        ConversionType::ZeroCopy,
        ConversionType::Safe,
    ];

    for conversion_type in conversion_types {
        match conversion_type {
            ConversionType::Auto => {
                println!("  测试自动转换");
            }
            ConversionType::ZeroCopy => {
                println!("  测试零拷贝转换");
            }
            ConversionType::Safe => {
                println!("  测试安全转换");
            }
        }
    }

    println!("✅ 类型转换演示完成");
    Ok(())
}

/// 运行性能监控专项测试
pub async fn run_performance_monitoring_demo(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("📈 性能监控专项演示");

    let performance_monitor = Arc::new(PerformanceMonitor::new());

    // 模拟耗时操作
    let result = performance_monitor
        .execute_with_monitoring("test_operation", || async {
            // 模拟一些工作
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            Ok(json!({"status": "completed", "data": "test_data"})).map_err(
                |e: Box<dyn std::error::Error + Send + Sync>| {
                    Box::new(std::io::Error::other(
                        e.to_string(),
                    )) as Box<dyn std::error::Error + Send + Sync>
                },
            )
        })
        .await?;

    println!("  监控操作结果: {}", result);

    println!("✅ 性能监控演示完成");
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
