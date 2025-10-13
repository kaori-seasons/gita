//! 优雅关机模块
//!
//! 提供信号处理、资源清理和状态保存的机制

use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};
use tokio::time::{timeout, Duration};
use tokio_util::sync::CancellationToken;

/// 关机信号类型
#[derive(Debug, Clone)]
pub enum ShutdownSignal {
    /// 正常关机
    Graceful,
    /// 强制关机
    Force,
    /// 超时关机
    Timeout,
}

/// 关机配置
#[derive(Debug, Clone)]
pub struct ShutdownConfig {
    /// 优雅关机超时时间（秒）
    pub graceful_timeout_seconds: u64,
    /// 强制关机超时时间（秒）
    pub force_timeout_seconds: u64,
    /// 关机前保存状态
    pub save_state_on_shutdown: bool,
}

impl Default for ShutdownConfig {
    fn default() -> Self {
        Self {
            graceful_timeout_seconds: 30,
            force_timeout_seconds: 10,
            save_state_on_shutdown: true,
        }
    }
}

/// 关机管理器
pub struct ShutdownManager {
    /// 广播发送器，用于通知所有组件关机
    shutdown_tx: broadcast::Sender<ShutdownSignal>,
    /// 取消令牌，用于取消正在运行的任务
    cancellation_token: CancellationToken,
    /// 活跃组件计数
    active_components: Arc<Mutex<std::collections::HashMap<String, bool>>>,
    /// 关机配置
    config: ShutdownConfig,
    /// 是否正在关机
    shutting_down: Arc<Mutex<bool>>,
}

impl ShutdownManager {
    /// 创建新的关机管理器
    pub fn new(config: ShutdownConfig) -> Self {
        let (shutdown_tx, _) = broadcast::channel(16);

        Self {
            shutdown_tx,
            cancellation_token: CancellationToken::new(),
            active_components: Arc::new(Mutex::new(std::collections::HashMap::new())),
            config,
            shutting_down: Arc::new(Mutex::new(false)),
        }
    }

    /// 注册组件
    pub async fn register_component(&self, component_name: &str) {
        let mut components = self.active_components.lock().await;
        components.insert(component_name.to_string(), true);
        tracing::info!("Component '{}' registered for shutdown management", component_name);
    }

    /// 取消注册组件
    pub async fn unregister_component(&self, component_name: &str) {
        let mut components = self.active_components.lock().await;
        components.remove(component_name);
        tracing::info!("Component '{}' unregistered from shutdown management", component_name);
    }

    /// 组件完成关机
    pub async fn component_shutdown_complete(&self, component_name: &str) {
        let mut components = self.active_components.lock().await;
        if let Some(component) = components.get_mut(component_name) {
            *component = false;
        }
        tracing::info!("Component '{}' shutdown completed", component_name);
    }

    /// 获取关机接收器
    pub fn subscribe(&self) -> broadcast::Receiver<ShutdownSignal> {
        self.shutdown_tx.subscribe()
    }

    /// 获取取消令牌
    pub fn cancellation_token(&self) -> CancellationToken {
        self.cancellation_token.clone()
    }

    /// 检查是否正在关机
    pub async fn is_shutting_down(&self) -> bool {
        *self.shutting_down.lock().await
    }

    /// 等待所有组件完成关机
    pub async fn wait_for_components(&self, timeout_duration: Duration) -> Result<(), ShutdownError> {
        let start_time = std::time::Instant::now();

        loop {
            let components = self.active_components.lock().await;
            let all_shutdown = components.values().all(|&active| !active);

            if all_shutdown {
                tracing::info!("All components have shutdown gracefully");
                return Ok(());
            }

            if start_time.elapsed() > timeout_duration {
                let active_components: Vec<String> = components
                    .iter()
                    .filter(|(_, &active)| active)
                    .map(|(name, _)| name.clone())
                    .collect();

                tracing::warn!("Shutdown timeout reached. Active components: {:?}", active_components);
                return Err(ShutdownError::Timeout {
                    active_components,
                    timeout_seconds: timeout_duration.as_secs(),
                });
            }

            // 每100ms检查一次
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    /// 发起优雅关机
    pub async fn initiate_graceful_shutdown(&self) -> Result<(), ShutdownError> {
        let mut shutting_down = self.shutting_down.lock().await;
        if *shutting_down {
            return Ok(()); // 已经在关机中
        }
        *shutting_down = true;

        tracing::info!("Initiating graceful shutdown...");

        // 发送优雅关机信号
        if let Err(e) = self.shutdown_tx.send(ShutdownSignal::Graceful) {
            tracing::error!("Failed to send graceful shutdown signal: {}", e);
        }

        // 取消所有任务
        self.cancellation_token.cancel();

        // 等待组件完成关机
        let timeout_duration = Duration::from_secs(self.config.graceful_timeout_seconds);
        match timeout(timeout_duration, self.wait_for_components(timeout_duration)).await {
            Ok(Ok(())) => {
                tracing::info!("Graceful shutdown completed successfully");
                Ok(())
            }
            Ok(Err(e)) => {
                tracing::error!("Graceful shutdown failed: {}", e);
                Err(e)
            }
            Err(_) => {
                tracing::warn!("Graceful shutdown timed out, initiating force shutdown");

                // 发送强制关机信号
                if let Err(e) = self.shutdown_tx.send(ShutdownSignal::Force) {
                    tracing::error!("Failed to send force shutdown signal: {}", e);
                }

                // 等待强制关机完成
                let force_timeout = Duration::from_secs(self.config.force_timeout_seconds);
                match timeout(force_timeout, self.wait_for_components(force_timeout)).await {
                    Ok(Ok(())) => {
                        tracing::info!("Force shutdown completed successfully");
                        Ok(())
                    }
                    _ => {
                        tracing::error!("Force shutdown also failed, initiating timeout shutdown");

                        // 发送超时关机信号
                        if let Err(e) = self.shutdown_tx.send(ShutdownSignal::Timeout) {
                            tracing::error!("Failed to send timeout shutdown signal: {}", e);
                        }

                        Err(ShutdownError::ForceShutdownFailed)
                    }
                }
            }
        }
    }

    /// 获取关机状态
    pub async fn get_shutdown_status(&self) -> ShutdownStatus {
        let components = self.active_components.lock().await;
        let shutting_down = *self.shutting_down.lock().await;

        let active_components: Vec<String> = components
            .iter()
            .filter(|(_, &active)| active)
            .map(|(name, _)| name.clone())
            .collect();

        ShutdownStatus {
            shutting_down,
            active_components,
            total_components: components.len(),
            graceful_timeout_seconds: self.config.graceful_timeout_seconds,
            force_timeout_seconds: self.config.force_timeout_seconds,
        }
    }
}

impl Default for ShutdownManager {
    fn default() -> Self {
        Self::new(ShutdownConfig::default())
    }
}

/// 关机状态
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ShutdownStatus {
    /// 是否正在关机
    pub shutting_down: bool,
    /// 活跃组件列表
    pub active_components: Vec<String>,
    /// 总组件数量
    pub total_components: usize,
    /// 优雅关机超时时间
    pub graceful_timeout_seconds: u64,
    /// 强制关机超时时间
    pub force_timeout_seconds: u64,
}

/// 关机错误
#[derive(Debug, thiserror::Error)]
pub enum ShutdownError {
    #[error("Shutdown timeout: active components {active_components:?}, timeout {timeout_seconds}s")]
    Timeout {
        active_components: Vec<String>,
        timeout_seconds: u64
    },

    #[error("Force shutdown failed")]
    ForceShutdownFailed,

    #[error("Component registration error: {message}")]
    ComponentError { message: String },
}

/// 关机钩子特质
#[async_trait::async_trait]
pub trait ShutdownHook: Send + Sync {
    /// 执行关机前的清理工作
    async fn on_shutdown(&self, signal: ShutdownSignal) -> Result<(), ShutdownError>;
}

/// 关机钩子注册器
pub struct ShutdownHooks {
    hooks: Arc<Mutex<Vec<Box<dyn ShutdownHook>>>>,
}

impl ShutdownHooks {
    /// 创建新的关机钩子注册器
    pub fn new() -> Self {
        Self {
            hooks: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// 注册关机钩子
    pub async fn register_hook(&self, hook: Box<dyn ShutdownHook>) {
        let mut hooks = self.hooks.lock().await;
        hooks.push(hook);
        tracing::info!("Shutdown hook registered, total hooks: {}", hooks.len());
    }

    /// 执行所有关机钩子
    pub async fn execute_hooks(&self, signal: ShutdownSignal) -> Result<(), ShutdownError> {
        let hooks = self.hooks.lock().await;
        tracing::info!("Executing {} shutdown hooks", hooks.len());

        for (i, hook) in hooks.iter().enumerate() {
            tracing::debug!("Executing shutdown hook {}", i);
            if let Err(e) = hook.on_shutdown(signal.clone()).await {
                tracing::error!("Shutdown hook {} failed: {}", i, e);
                // 继续执行其他钩子，不因单个钩子失败而停止
            }
        }

        Ok(())
    }
}

impl Default for ShutdownHooks {
    fn default() -> Self {
        Self::new()
    }
}

/// 信号处理器
pub struct SignalHandler {
    shutdown_manager: Arc<ShutdownManager>,
}

impl SignalHandler {
    /// 创建新的信号处理器
    pub fn new(shutdown_manager: Arc<ShutdownManager>) -> Self {
        Self { shutdown_manager }
    }

    /// 启动信号监听
    pub async fn start_listening(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        #[cfg(unix)]
        {
            use tokio::signal::unix::{signal, SignalKind};

            // 监听 SIGTERM
            let mut sigterm = signal(SignalKind::terminate())?;
            let shutdown_manager = Arc::clone(&self.shutdown_manager);

            tokio::spawn(async move {
                if let Some(_) = sigterm.recv().await {
                    tracing::info!("Received SIGTERM signal");
                    if let Err(e) = shutdown_manager.initiate_graceful_shutdown().await {
                        tracing::error!("Graceful shutdown failed: {}", e);
                        std::process::exit(1);
                    }
                }
            });

            // 监听 SIGINT (Ctrl+C)
            let mut sigint = signal(SignalKind::interrupt())?;
            let shutdown_manager = Arc::clone(&self.shutdown_manager);

            tokio::spawn(async move {
                if let Some(_) = sigint.recv().await {
                    tracing::info!("Received SIGINT signal");
                    if let Err(e) = shutdown_manager.initiate_graceful_shutdown().await {
                        tracing::error!("Graceful shutdown failed: {}", e);
                        std::process::exit(1);
                    }
                }
            });
        }

        #[cfg(windows)]
        {
            use tokio::signal::windows::{ctrl_break, ctrl_c};

            // 监听 Ctrl+C
            let mut ctrl_c_signal = ctrl_c()?;
            let shutdown_manager = Arc::clone(&self.shutdown_manager);

            tokio::spawn(async move {
                if let Some(_) = ctrl_c_signal.recv().await {
                    tracing::info!("Received Ctrl+C signal");
                    if let Err(e) = shutdown_manager.initiate_graceful_shutdown().await {
                        tracing::error!("Graceful shutdown failed: {}", e);
                        std::process::exit(1);
                    }
                }
            });

            // 监听 Ctrl+Break
            let mut ctrl_break_signal = ctrl_break()?;
            let shutdown_manager = Arc::clone(&self.shutdown_manager);

            tokio::spawn(async move {
                if let Some(_) = ctrl_break_signal.recv().await {
                    tracing::info!("Received Ctrl+Break signal");
                    if let Err(e) = shutdown_manager.initiate_graceful_shutdown().await {
                        tracing::error!("Graceful shutdown failed: {}", e);
                        std::process::exit(1);
                    }
                }
            });
        }

        tracing::info!("Signal handler started");
        Ok(())
    }
}

/// 便捷函数：创建默认的关机管理器
pub fn create_default_shutdown_manager() -> Arc<ShutdownManager> {
    Arc::new(ShutdownManager::new(ShutdownConfig {
        graceful_timeout_seconds: 30,
        force_timeout_seconds: 10,
        save_state_on_shutdown: true,
    }))
}
