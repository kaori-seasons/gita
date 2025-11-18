//! Rust Edge Compute Framework - 主程序入口

use rust_edge_compute_core::config::{load_default_config, Settings};
use rust_edge_compute_core::core::{
    ExecutorRegistry, PersistenceManager, ErrorHandler, TaskScheduler, 
    SchedulerConfig, ShutdownManager, ShutdownConfig, ShutdownHook, ShutdownSignal,
    ShutdownError, ShutdownHooks, SignalHandler,
};
use rust_edge_compute_core::api::{handlers::AppState, server::{HttpServer, ServerConfig}};

// 条件编译：根据features启用不同的executor
#[cfg(feature = "cpp")]
use rust_edge_compute_cpp::CppExecutor;
#[cfg(feature = "ml")]
use rust_edge_compute_ml::CandleMlExecutor;
#[cfg(feature = "python")]
use rust_edge_compute_python::PythonWasmExecutor;

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use std::sync::Arc;
use async_trait::async_trait;

// 定义Result类型
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

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

    // 创建Executor注册表
    let executor_registry = Arc::new(ExecutorRegistry::new());

    // 注册executor（根据features）
    #[cfg(feature = "cpp")]
    {
        let cpp_executor = Arc::new(CppExecutor::new());
        executor_registry.register(cpp_executor).await?;
        tracing::info!("C++ Executor registered");
    }

    #[cfg(feature = "ml")]
    {
        let ml_executor = Arc::new(CandleMlExecutor::new()?);
        executor_registry.register(ml_executor).await?;
        tracing::info!("Candle ML Executor registered");
    }

    #[cfg(feature = "python")]
    {
        let python_executor = Arc::new(PythonWasmExecutor::new()?);
        executor_registry.register(python_executor).await?;
        tracing::info!("Python WASM Executor registered");
    }

    // 创建持久化管理器
    let persistence_manager = Arc::new(PersistenceManager::new("./data/db")
        .unwrap_or_else(|e| {
            tracing::warn!("Failed to create persistence manager: {}, using in-memory storage", e);
            PersistenceManager::default()
        }));

    // 创建错误处理器并设置持久化存储
    let error_handler = Arc::new(ErrorHandler::new()
        .with_persistence_store(Arc::clone(&persistence_manager.store())));

    tracing::info!("Error handler with persistence initialized");

    // 创建任务调度器（使用executor注册表）
    let scheduler = Arc::new(TaskScheduler::new(SchedulerConfig {
        max_concurrent_tasks: 10,
        queue_size: settings.server.task_queue_size,
        task_timeout_seconds: settings.server.request_timeout_seconds,
        default_max_retries: 3,
    })
    .with_error_handler(Arc::clone(&error_handler))
    .with_executor_registry(Arc::clone(&executor_registry)));

    tracing::info!("Task scheduler created with max_concurrent_tasks: {}", 10);

    // 启动调度器
    let scheduler_clone = Arc::clone(&scheduler);
    let error_handler_clone = Arc::clone(&error_handler);
    tokio::spawn(async move {
        if let Err(e) = scheduler_clone.start().await {
            let error = rust_edge_compute_core::core::EdgeComputeError::TaskScheduling {
                message: format!("Failed to start scheduler: {}", e),
                task_id: None,
                queue_size: None,
            };
            let _ = error_handler_clone.handle_error(error).await;
        }
    });

    // 创建服务器配置
    let server_config = ServerConfig {
        host: settings.server.host.clone(),
        port: settings.server.port,
        task_queue_size: settings.server.task_queue_size,
    };

    // 创建优雅关机管理器
    let shutdown_manager = Arc::new(ShutdownManager::new(ShutdownConfig {
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
    let shutdown_hooks = Arc::new(ShutdownHooks::new());

    // 添加状态保存钩子
    let persistence_clone = Arc::clone(&persistence_manager);
    let scheduler_clone = Arc::clone(&scheduler);
    let error_handler_clone = Arc::clone(&error_handler);

    struct StateSaveHook {
        persistence: Arc<PersistenceManager>,
        scheduler: Arc<TaskScheduler>,
        error_handler: Arc<ErrorHandler>,
    }

    #[async_trait]
    impl ShutdownHook for StateSaveHook {
        async fn on_shutdown(&self, _signal: ShutdownSignal) -> Result<(), ShutdownError> {
            tracing::info!("Saving application state before shutdown...");

            // 保存错误统计
            let error_stats = self.error_handler.get_stats().await;
            if let Err(e) = self.persistence.store().store_error_stats(&error_stats).await {
                tracing::error!("Failed to save error stats: {}", e);
            }

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
    let signal_handler = Arc::new(SignalHandler::new(Arc::clone(&shutdown_manager)));
    if let Err(e) = signal_handler.start_listening().await {
        tracing::error!("Failed to start signal handler: {}", e);
        return Err(e.into());
    }

    // 创建应用状态
    let app_state = AppState {
        scheduler: Arc::clone(&scheduler),
        error_handler: Arc::clone(&error_handler),
        executor_registry: Arc::clone(&executor_registry),
    };

    // 创建HTTP服务器
    let server = HttpServer::new(server_config, app_state);

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

