//! Rust Edge Compute Framework - 主程序入口

// 直接导入API模块
mod api;
mod container;
mod core;
mod ffi;

use rust_edge_compute_core::SpawnConfig;
use rust_edge_compute_core::TaskSpawner;

// 定义Result类型
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    init_logging()?;

    tracing::info!("Starting Rust Edge Compute Framework");

    // 创建持久化管理器
    let persistence_manager = Arc::new(core::PersistenceManager::new("./data/db").unwrap_or_else(
        |e| {
            tracing::warn!(
                "Failed to create persistence manager: {}, using in-memory storage",
                e
            );
            core::PersistenceManager::default()
        },
    ));

    // 创建错误处理器并设置持久化存储
    let error_handler = Arc::new(
        core::ErrorHandler::new().with_persistence_store(Arc::clone(&persistence_manager.store())),
    );

    tracing::info!("Error handler with persistence initialized");

    // 创建任务调度器
    let scheduler = Arc::new(core::TaskScheduler::new(core::SchedulerConfig {
        max_concurrent_tasks: 10,
        queue_size: 1000,          // 默认值
        task_timeout_seconds: 300, // 默认值
        default_max_retries: 3,
        intelligent_scheduling_enabled: false,
        load_balancer_config: rust_edge_compute_core::load_balancer::LoadBalancerConfig::default(),
    }));

    tracing::info!("Task scheduler created with max_concurrent_tasks: {}", 10);

    // 启动内存监控任务
    let metrics = core::metrics_collector::GLOBAL_METRICS.clone();

    // [根据配置设置指标收集器开关 - 简化版本禁用配置]
    metrics.set_enabled(false);
    // metrics.set_enabled(settings.monitoring.metrics_enabled);

    // [以下简化实现空实现]
    // if settings.monitoring.metrics_enabled {
    // [指标收集代码已注释 - 简化版本]
    // if settings.monitoring.metrics_enabled {

    // 启动调度器
    let scheduler_clone = Arc::clone(&scheduler);
    let error_handler_clone = Arc::clone(&error_handler);
    TaskSpawner::spawn_with_config(
        async move {
            if let Err(e) = scheduler_clone.start().await {
                let error = core::EdgeComputeError::TaskScheduling {
                    message: format!("Failed to start scheduler: {}", e),
                    task_id: None,
                    queue_size: None,
                };
                let _ = error_handler_clone.handle_error(error).await;
            }
            Ok(())
        },
        SpawnConfig::new("scheduler_start")
            .with_timeout(300)
            .with_detailed_errors(true),
    );

    // 创建服务器配置（简化版）
    let server_config = api::server::ServerConfig {
        host: "127.0.0.1".to_string(),
        port: 8080,
        task_queue_size: 1000,
    };

    // [关机管理代码已注释 - 使用简化版本以快速编译]
    // let shutdown_manager = Arc::new(core::ShutdownManager::new(core::ShutdownConfig {
    //     graceful_timeout_seconds: 30,
    //     force_timeout_seconds: 10,
    //     save_state_on_shutdown: true,
    // }));

    // [信号处理器代码已注释 - 简化版本]
    // let signal_handler = Arc::new(core::SignalHandler::new(Arc::clone(&shutdown_manager)));
    // if let Err(e) = signal_handler.start_listening().await {
    //     tracing::error!("Failed to start signal handler: {}", e);
    //     return Err(e.into());
    // }

    // 创建应用状态
    let app_state = api::handlers::AppState {
        scheduler: Arc::clone(&scheduler),
        error_handler: Arc::clone(&error_handler),
    };

    // 创建HTTP服务器
    let server = api::server::HttpServer::new(server_config, app_state);

    tracing::info!("HTTP server configured on 127.0.0.1:8080");

    // [启动服务器的简化版本]
    let server_future = server.start();

    // 简单运行服务器而不等待关机信号
    if let Err(e) = server_future.await {
        tracing::error!("Server error: {}", e);
        return Err(e);
    }

    tracing::info!("Application shutdown completed");
    Ok(())
}

/// 初始化日志系统
fn init_logging() -> Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rust_edge_compute=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    Ok(())
}
