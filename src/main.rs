//! Rust Edge Compute Framework - 主程序入口

use rust_edge_compute::{
    config::{load_default_config, Settings},
};

// 直接导入API模块
mod api;
mod core;
mod container;
mod ffi;

// 定义Result类型
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    init_logging()?;

    tracing::info!("Starting Rust Edge Compute Framework");

    // 加载配置
    let settings = load_default_config().unwrap_or_else(|_| {
        tracing::warn!("Failed to load config, using defaults");
        Settings::default()
    });

    tracing::info!("Loaded configuration: {:?}", settings);

    // 创建持久化管理器
    let persistence_manager = Arc::new(core::PersistenceManager::new("./data/db")
        .unwrap_or_else(|e| {
            tracing::warn!("Failed to create persistence manager: {}, using in-memory storage", e);
            core::PersistenceManager::default()
        }));

    // 创建错误处理器并设置持久化存储
    let error_handler = Arc::new(core::ErrorHandler::new()
        .with_persistence_store(Arc::clone(&persistence_manager.store())));

    tracing::info!("Error handler with persistence initialized");

    // 创建任务调度器
    let scheduler = Arc::new(core::TaskScheduler::new(core::SchedulerConfig {
        max_concurrent_tasks: 10,
        queue_size: settings.server.task_queue_size,
        task_timeout_seconds: settings.server.request_timeout_seconds,
        default_max_retries: 3,
        intelligent_scheduling_enabled: false,
        load_balancer_config: core::LoadBalancerConfig::default(),
    }).with_error_handler(Arc::clone(&error_handler)));

    tracing::info!("Task scheduler created with max_concurrent_tasks: {}", 10);

    // 启动内存监控任务
    let metrics = core::metrics_collector::GLOBAL_METRICS.clone();
    crate::core::TaskSpawner::spawn_with_config(
        async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
            loop {
                interval.tick().await;

                // 更新 Rust 堆内存（从 GlobalAllocator）
                metrics.update_rust_heap_bytes_from_allocator();

                // 更新系统资源指标
                metrics.update_rss_from_system().await;
                metrics.update_vm_total_from_system().await;
                metrics.update_mapped_from_system().await;

                // 更新 CPU 使用率
                let cpu = core::metrics_collector::system_metrics::get_cpu_usage();
                metrics.set_cpu_usage(cpu).await;

                // 每 30 秒记录一次日志
                static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
                let count = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                if count % 6 == 0 {
                    tracing::debug!(
                        "Memory metrics: Rust={}MB, RSS={}MB, VmSize={}MB, Mapped={}MB",
                        metrics.get_rust_heap_bytes() / 1024 / 1024,
                        metrics.get_rss_bytes().await / 1024 / 1024,
                        metrics.get_vm_total_bytes().await / 1024 / 1024,
                        metrics.get_mapped_bytes().await / 1024 / 1024,
                    );
                }
            }
        },
        crate::core::SpawnConfig::new("memory_monitor")
            .with_detailed_errors(true)
    );

    // 启动调度器
    let scheduler_clone = Arc::clone(&scheduler);
    let error_handler_clone = Arc::clone(&error_handler);
    crate::core::TaskSpawner::spawn_with_config(
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
        crate::core::SpawnConfig::new("scheduler_start")
            .with_timeout(300)
            .with_detailed_errors(true)
    );

    // 创建服务器配置
    let server_config = api::server::ServerConfig {
        host: settings.server.host.clone(),
        port: settings.server.port,
        task_queue_size: settings.server.task_queue_size,
    };

    // 创建优雅关机管理器
    let shutdown_manager = Arc::new(core::ShutdownManager::new(core::ShutdownConfig {
        graceful_timeout_seconds: 30,
        force_timeout_seconds: 10,
        save_state_on_shutdown: true,
    }));

    tracing::info!("Shutdown manager initialized");

    // 注册组件
    shutdown_manager.register_component("scheduler").await;
    shutdown_manager.register_component("error_handler").await;
    shutdown_manager.register_component("persistence").await;
    shutdown_manager.register_component("http_server").await;

    // 创建关机钩子
    let shutdown_hooks = Arc::new(core::ShutdownHooks::new());

    // 添加状态保存钩子
    let persistence_clone = Arc::clone(&persistence_manager);
    let scheduler_clone = Arc::clone(&scheduler);
    let error_handler_clone = Arc::clone(&error_handler);

    struct StateSaveHook {
        persistence: Arc<core::PersistenceManager>,
        scheduler: Arc<core::TaskScheduler>,
        error_handler: Arc<core::ErrorHandler>,
    }

    #[async_trait::async_trait]
    impl core::ShutdownHook for StateSaveHook {
        async fn on_shutdown(&self, _signal: core::ShutdownSignal) -> std::result::Result<(), core::ShutdownError> {
            tracing::info!("Saving application state before shutdown...");

            // 保存错误统计
            let error_stats = self.error_handler.get_stats().await;
            if let Err(e) = self.persistence.store().store_error_stats(&error_stats).await {
                tracing::error!("Failed to save error stats: {}", e);
            }

            // 保存任务队列状态（如果需要）
            // 这里可以扩展保存更多状态

            tracing::info!("Application state saved successfully");
            Ok(())
        }
    }

    let state_save_hook = Box::new(StateSaveHook {
        persistence: Arc::clone(&persistence_clone),
        scheduler: Arc::clone(&scheduler_clone),
        error_handler: Arc::clone(&error_handler_clone),
    });

    shutdown_hooks.register_hook(state_save_hook).await;

    // 启动信号处理器
    let signal_handler = Arc::new(core::SignalHandler::new(Arc::clone(&shutdown_manager)));
    if let Err(e) = signal_handler.start_listening().await {
        tracing::error!("Failed to start signal handler: {}", e);
        return Err(e.into());
    }

    // 创建应用状态
    let app_state = api::handlers::AppState {
        scheduler: Arc::clone(&scheduler),
        error_handler: Arc::clone(&error_handler),
    };

    // 创建HTTP服务器
    let server = api::server::HttpServer::new(server_config, app_state);

    tracing::info!("HTTP server configured on {}:{}", settings.server.host, settings.server.port);

    // 启动服务器（带关机管理）
    let server_future = server.start();
    let shutdown_future = async {
        let mut shutdown_rx = shutdown_manager.subscribe();
        if let Ok(signal) = shutdown_rx.recv().await {
            tracing::info!("Received shutdown signal: {:?}", signal);

            // 执行关机钩子
            if let Err(e) = shutdown_hooks.execute_hooks(signal.clone()).await {
                tracing::error!("Shutdown hooks execution failed: {}", e);
            }

            // 标记服务器组件已完成关机
            shutdown_manager.component_shutdown_complete("http_server").await;
        }
    };

    // 并发运行服务器和关机监听
    tokio::select! {
        result = server_future => {
            if let Err(e) = result {
                tracing::error!("Server error: {}", e);
                let _ = shutdown_manager.initiate_graceful_shutdown().await;
                return Err(e);
            }
        }
        _ = shutdown_future => {
            tracing::info!("Shutdown signal received, stopping server...");
        }
    }

    // 等待所有组件完成关机
    if let Err(e) = shutdown_manager.initiate_graceful_shutdown().await {
        tracing::error!("Graceful shutdown failed: {}", e);
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
