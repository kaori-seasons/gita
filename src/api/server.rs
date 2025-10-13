//! HTTP服务器实现

use std::net::SocketAddr;
use tokio::sync::mpsc;

use super::{handlers::*, routes::create_routes};
use super::super::core::ComputeResponse;

// 定义Result类型
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// 服务器配置
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// 监听地址
    pub host: String,
    /// 监听端口
    pub port: u16,
    /// 任务队列大小
    pub task_queue_size: usize,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 3000,
            task_queue_size: 1000,
        }
    }
}

/// HTTP服务器
pub struct HttpServer {
    config: ServerConfig,
    state: AppState,
}

impl HttpServer {
    /// 创建新的HTTP服务器
    pub fn new(config: ServerConfig, state: AppState) -> Self {
        Self { config, state }
    }

    /// 启动服务器
    pub async fn start(self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let addr: SocketAddr = format!("{}:{}", self.config.host, self.config.port)
            .parse()
            .map_err(|e| format!("Invalid address: {}", e))?;

        // 创建路由
        let app = create_routes(self.state.clone());

        tracing::info!("Starting HTTP server on {}", addr);

        // 启动HTTP服务器
        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;

        Ok(())
    }
}
