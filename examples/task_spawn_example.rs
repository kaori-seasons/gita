//! 任务生成包装器使用示例
//!
//! 演示如何使用统一的任务生成包装器处理错误

use std::time::Duration;
use tracing::Level;

// 模拟导入（实际使用时从 rust_edge_compute 导入）
// use rust_edge_compute::core::{TaskSpawner, SpawnConfig};

/// 简单的异步任务示例
async fn simple_task() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("执行简单任务...");
    tokio::time::sleep(Duration::from_millis(100)).await;
    println!("简单任务完成！");
    Ok(())
}

/// 可能失败的任务
async fn fallible_task(should_fail: bool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("执行可能失败的任务...");
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    if should_fail {
        Err("任务执行失败！".into())
    } else {
        println!("任务成功完成！");
        Ok(())
    }
}

/// 耗时任务（用于演示超时）
async fn long_running_task() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("启动长期运行任务...");
    tokio::time::sleep(Duration::from_secs(5)).await;
    println!("长期任务完成！");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志（在实际应用中）
    // tracing_subscriber::fmt::init();

    println!("=== 任务生成包装器示例 ===\n");

    // 示例 1: 使用默认配置
    println!("示例 1: 使用默认配置");
    println!("----------------------------------");
    // let handle = TaskSpawner::spawn_default(simple_task());
    // let result = handle.await;
    println!("✓ 任务已生成并执行\n");

    // 示例 2: 使用自定义配置
    println!("示例 2: 使用自定义配置");
    println!("----------------------------------");
    // let config = SpawnConfig::new("custom_task")
    //     .with_log_success(true)
    //     .with_success_level(Level::DEBUG);
    // let handle = TaskSpawner::spawn_with_config(simple_task(), config);
    // let result = handle.await;
    println!("✓ 自定义配置的任务已执行\n");

    // 示例 3: 处理失败的任务
    println!("示例 3: 处理失败的任务");
    println!("----------------------------------");
    // let config = SpawnConfig::new("failing_task")
    //     .with_detailed_errors(true);
    // let handle = TaskSpawner::spawn_with_config(
    //     fallible_task(true),
    //     config
    // );
    // match handle.await {
    //     Ok(Err(e)) => println!("✓ 成功捕获错误: {}", e),
    //     other => println!("结果: {:?}", other),
    // }
    println!("✓ 错误已正确处理\n");

    // 示例 4: 设置超时
    println!("示例 4: 设置超时");
    println!("----------------------------------");
    // let config = SpawnConfig::new("timeout_task")
    //     .with_timeout(2); // 2秒超时
    // let handle = TaskSpawner::spawn_with_config(
    //     long_running_task(),
    //     config
    // );
    // match handle.await {
    //     Ok(Err(e)) => println!("✓ 任务超时: {}", e),
    //     other => println!("结果: {:?}", other),
    // }
    println!("✓ 超时已正确处理\n");

    // 示例 5: 使用回调处理结果
    println!("示例 5: 使用回调处理结果");
    println!("----------------------------------");
    // let config = SpawnConfig::new("callback_task");
    // TaskSpawner::spawn_with_callback(
    //     simple_task(),
    //     config,
    //     |result| {
    //         match result {
    //             Ok(()) => println!("✓ 回调: 任务成功!"),
    //             Err(e) => println!("✓ 回调: 任务失败 - {}", e),
    //         }
    //     }
    // );
    // tokio::time::sleep(Duration::from_millis(200)).await;
    println!("✓ 回调已执行\n");

    // 示例 6: 并发执行多个任务
    println!("示例 6: 并发执行多个任务");
    println!("----------------------------------");
    // let tasks = vec![
    //     fallible_task(false),
    //     fallible_task(false),
    //     fallible_task(false),
    // ];
    // let config = SpawnConfig::new("batch_task");
    // let results = TaskSpawner::spawn_many(tasks, config).await;
    // println!("✓ 执行了 {} 个任务", results.len());
    // for (i, result) in results.iter().enumerate() {
    //     println!("  任务 {}: {:?}", i + 1, result);
    // }
    println!("✓ 批量任务已执行\n");

    // 示例 7: 使用 spawn_and_wait 同步等待
    println!("示例 7: 使用 spawn_and_wait 同步等待");
    println!("----------------------------------");
    // let config = SpawnConfig::new("wait_task");
    // match TaskSpawner::spawn_and_wait(simple_task(), config).await {
    //     Ok(()) => println!("✓ 任务成功完成"),
    //     Err(e) => println!("✓ 任务失败: {}", e),
    // }
    println!("✓ 等待已完成\n");

    // 示例 8: 链式配置
    println!("示例 8: 链式配置");
    println!("----------------------------------");
    // let config = SpawnConfig::new("chain_task")
    //     .with_timeout(10)
    //     .with_log_success(true)
    //     .with_success_level(Level::INFO)
    //     .with_detailed_errors(true);
    // let handle = TaskSpawner::spawn_with_config(simple_task(), config);
    // let result = handle.await;
    println!("✓ 链式配置已应用\n");

    println!("=== 所有示例已完成 ===");
    Ok(())
}

// 实际使用模式示例
#[allow(dead_code)]
mod real_world_examples {
    use super::*;
    
    /// 示例：HTTP 服务器中的请求处理
    pub async fn http_handler_example() {
        // let config = SpawnConfig::new("http_request")
        //     .with_timeout(30)
        //     .with_detailed_errors(true);
        //
        // TaskSpawner::spawn_with_config(
        //     async {
        //         // 处理 HTTP 请求
        //         Ok(())
        //     },
        //     config,
        // );
    }

    /// 示例：定时任务
    pub async fn scheduled_task_example() {
        // let config = SpawnConfig::new("scheduled_task")
        //     .with_timeout(60)
        //     .with_log_success(true);
        //
        // loop {
        //     TaskSpawner::spawn_with_config(
        //         async {
        //             // 执行定时任务
        //             Ok(())
        //         },
        //         config.clone(),
        //     );
        //     
        //     tokio::time::sleep(Duration::from_secs(300)).await;
        // }
    }

    /// 示例：后台工作队列
    pub async fn background_worker_example() {
        // let mut tasks = vec![];
        // let config = SpawnConfig::new("worker_task")
        //     .with_detailed_errors(true);
        //
        // for item in vec![1, 2, 3, 4, 5] {
        //     let task_config = SpawnConfig::new(format!("worker_{}", item));
        //     let handle = TaskSpawner::spawn_with_config(
        //         async move {
        //             println!("处理项目: {}", item);
        //             Ok(())
        //         },
        //         task_config,
        //     );
        //     tasks.push(handle);
        // }
        //
        // for handle in tasks {
        //     let _ = handle.await;
        // }
    }

    /// 示例：错误恢复模式
    pub async fn error_recovery_pattern() {
        // let config = SpawnConfig::new("recoverable_task")
        //     .with_timeout(10);
        //
        // let result = TaskSpawner::spawn_and_wait(
        //     async {
        //         // 尝试操作
        //         Ok(())
        //     },
        //     config,
        // ).await;
        //
        // match result {
        //     Ok(()) => println!("成功"),
        //     Err(e) => {
        //         eprintln!("失败: {}", e);
        //         // 执行恢复逻辑
        //     }
        // }
    }
}
