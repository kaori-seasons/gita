//! ZeroMQ 数据源集成
//!
//! 从 ZeroMQ 接收测量点数据，支持按测量点分组
//! 提供高性能的消息接收和解析能力
//! 
//! 使用 zeromq crate 的异步 API，参考：
//! https://github.com/zeromq/zmq.rs/blob/master/tests/pub_sub.rs

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use serde::{Deserialize, Serialize};
use zeromq::prelude::*;
use zeromq::ZmqMessage;
use crate::core::error::Result;

/// ZeroMQ 消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZeroMQMessage {
    /// 测量点ID（如 "1464"）
    pub measurement_point_id: String,
    /// 位移（序列号，从上游保证有序）
    pub sequence: u64,
    /// 时间戳
    pub timestamp: u64,
    /// 数据负载
    pub payload: serde_json::Value,
    /// 元数据
    pub metadata: HashMap<String, String>,
}

/// ZeroMQ Socket 类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZeroMQSocketType {
    /// PULL socket（用于接收消息）
    Pull,
    /// SUB socket（用于订阅消息）
    Sub,
}

/// ZeroMQ 数据源配置
#[derive(Debug, Clone)]
pub struct ZeroMQConfig {
    /// ZeroMQ 连接地址
    pub endpoint: String,
    /// Socket 类型
    pub socket_type: ZeroMQSocketType,
    /// 接收超时（毫秒）
    pub receive_timeout_ms: u64,
    /// 最大缓冲区大小
    pub max_buffer_size: usize,
    /// 订阅过滤器（仅用于 SUB socket，空字符串表示订阅所有消息）
    pub subscribe_filter: Option<String>,
}

impl Default for ZeroMQConfig {
    fn default() -> Self {
        Self {
            endpoint: "tcp://localhost:5555".to_string(),
            socket_type: ZeroMQSocketType::Pull,
            receive_timeout_ms: 1000,
            max_buffer_size: 10000,
            subscribe_filter: None,
        }
    }
}

/// ZeroMQ 数据源统计信息
#[derive(Debug, Clone, Default)]
pub struct ZeroMQSourceStats {
    /// 总接收消息数
    pub messages_received: u64,
    /// 总处理消息数
    pub messages_processed: u64,
    /// 总错误数
    pub errors_count: u64,
    /// 当前缓冲区大小
    pub buffer_size: usize,
    /// 平均处理延迟（毫秒）
    pub avg_processing_latency_ms: f64,
    /// 最后处理时间
    pub last_message_timestamp: u64,
}

/// ZeroMQ 数据源
pub struct ZeroMQSource {
    config: ZeroMQConfig,
    sender: mpsc::Sender<ZeroMQMessage>,
    receiver: mpsc::Receiver<ZeroMQMessage>,
    is_running: Arc<RwLock<bool>>,
    stats: Arc<RwLock<ZeroMQSourceStats>>,
}

impl ZeroMQSource {
    /// 创建新的 ZeroMQ 数据源
    pub fn new(config: ZeroMQConfig) -> Result<Self> {
        let (sender, receiver) = mpsc::channel(config.max_buffer_size);
        
        Ok(Self {
            config,
            sender,
            receiver,
            is_running: Arc::new(RwLock::new(false)),
            stats: Arc::new(RwLock::new(ZeroMQSourceStats::default())),
        })
    }
    
    /// 启动 ZeroMQ 消费者
    pub async fn start(&self) -> Result<()> {
        let mut is_running = self.is_running.write().await;
        if *is_running {
            return Err("ZeroMQ source is already running".into());
        }
        
        tracing::info!("Starting ZeroMQ source with endpoint: {}", self.config.endpoint);
        
        *is_running = true;
        
        // 启动消费循环
        let config = self.config.clone();
        let sender = self.sender.clone();
        let stats = self.stats.clone();
        let is_running_clone = self.is_running.clone();
        
        tokio::spawn(async move {
            Self::consume_loop(
                config,
                sender,
                stats,
                is_running_clone,
            ).await;
        });
        
        tracing::info!("ZeroMQ source started successfully");
        Ok(())
    }
    
    /// 停止 ZeroMQ 消费者
    pub async fn stop(&self) -> Result<()> {
        let mut is_running = self.is_running.write().await;
        if !*is_running {
            return Ok(());
        }
        
        tracing::info!("Stopping ZeroMQ source");
        
        *is_running = false;
        
        // 等待一小段时间让消费循环停止
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        tracing::info!("ZeroMQ source stopped successfully");
        Ok(())
    }
    
    /// 订阅消息流
    pub fn subscribe(&self) -> mpsc::Receiver<ZeroMQMessage> {
        self.receiver.clone()
    }
    
    /// 获取统计信息
    pub async fn get_stats(&self) -> ZeroMQSourceStats {
        self.stats.read().await.clone()
    }
    
    /// 消费循环
    async fn consume_loop(
        config: ZeroMQConfig,
        sender: mpsc::Sender<ZeroMQMessage>,
        stats: Arc<RwLock<ZeroMQSourceStats>>,
        is_running: Arc<RwLock<bool>>,
    ) {
        tracing::info!("Starting ZeroMQ consumption loop");
        
        // 根据配置创建并启动 socket
        match config.socket_type {
            ZeroMQSocketType::Pull => {
                Self::consume_loop_pull(config, sender, stats, is_running).await;
            }
            ZeroMQSocketType::Sub => {
                Self::consume_loop_sub(config, sender, stats, is_running).await;
            }
        }
        
        tracing::info!("ZeroMQ consumption loop stopped");
    }
    
    /// PULL socket 消费循环
    async fn consume_loop_pull(
        config: ZeroMQConfig,
        sender: mpsc::Sender<ZeroMQMessage>,
        stats: Arc<RwLock<ZeroMQSourceStats>>,
        is_running: Arc<RwLock<bool>>,
    ) {
        let mut pull_socket = zeromq::PullSocket::new();
        
        // PULL socket 使用 bind
        pull_socket
            .bind(&config.endpoint)
            .await
            .unwrap_or_else(|e| {
                panic!("Failed to bind PULL socket to {}: {}", config.endpoint, e);
            });
        
        tracing::info!("ZeroMQ PULL socket bound to {}, starting message loop", config.endpoint);
        
        loop {
            // 检查是否应该停止
            if !*is_running.read().await {
                break;
            }
            
            // 接收消息（使用超时）
            let recv_result = tokio::time::timeout(
                Duration::from_millis(config.receive_timeout_ms),
                pull_socket.recv(),
            ).await;
            
            match recv_result {
                Ok(Ok(message)) => {
                    Self::handle_message(message, &sender, &stats).await;
                }
                Ok(Err(e)) => {
                    tracing::error!("ZeroMQ receive error: {}", e);
                    let mut stats = stats.write().await;
                    stats.errors_count += 1;
                    
                    // 如果是连接错误，等待一段时间再重试
                    if e.to_string().contains("Connection") {
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                }
                Err(_) => {
                    // 超时，继续循环
                    continue;
                }
            }
        }
    }
    
    /// SUB socket 消费循环
    async fn consume_loop_sub(
        config: ZeroMQConfig,
        sender: mpsc::Sender<ZeroMQMessage>,
        stats: Arc<RwLock<ZeroMQSourceStats>>,
        is_running: Arc<RwLock<bool>>,
    ) {
        let mut sub_socket = zeromq::SubSocket::new();
        
        // SUB socket 使用 connect
        sub_socket
            .connect(&config.endpoint)
            .await
            .unwrap_or_else(|e| {
                panic!("Failed to connect SUB socket to {}: {}", config.endpoint, e);
            });
        
        // 订阅消息
        let filter = config.subscribe_filter.as_deref().unwrap_or("");
        sub_socket
            .subscribe(filter)
            .await
            .expect("Failed to subscribe");
        
        tracing::info!("ZeroMQ SUB socket connected to {}, subscribed to '{}', starting message loop", config.endpoint, filter);
        
        loop {
            // 检查是否应该停止
            if !*is_running.read().await {
                break;
            }
            
            // 接收消息（使用超时）
            let recv_result = tokio::time::timeout(
                Duration::from_millis(config.receive_timeout_ms),
                sub_socket.recv(),
            ).await;
            
            match recv_result {
                Ok(Ok(message)) => {
                    Self::handle_message(message, &sender, &stats).await;
                }
                Ok(Err(e)) => {
                    tracing::error!("ZeroMQ receive error: {}", e);
                    let mut stats = stats.write().await;
                    stats.errors_count += 1;
                    
                    // 如果是连接错误，等待一段时间再重试
                    if e.to_string().contains("Connection") {
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                }
                Err(_) => {
                    // 超时，继续循环
                    continue;
                }
            }
        }
    }
    
    /// 处理接收到的消息
    async fn handle_message(
        message: ZmqMessage,
        sender: &mpsc::Sender<ZeroMQMessage>,
        stats: &Arc<RwLock<ZeroMQSourceStats>>,
    ) {
        let start_time = std::time::Instant::now();
        
        // 更新统计
        {
            let mut stats = stats.write().await;
            stats.messages_received += 1;
            stats.buffer_size = sender.len();
        }
        
        // 解析消息
        match Self::parse_message(&message) {
            Ok(zmq_message) => {
                // 发送到处理通道
                match sender.send(zmq_message).await {
                    Ok(_) => {
                        let processing_time = start_time.elapsed().as_millis() as f64;
                        
                        // 更新统计
                        let mut stats = stats.write().await;
                        stats.messages_processed += 1;
                        stats.avg_processing_latency_ms =
                            (stats.avg_processing_latency_ms * 0.9) + (processing_time * 0.1);
                        stats.last_message_timestamp = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs();
                    }
                    Err(e) => {
                        tracing::warn!("Failed to send ZeroMQ message: {}", e);
                        let mut stats = stats.write().await;
                        stats.errors_count += 1;
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Failed to parse ZeroMQ message: {}", e);
                let mut stats = stats.write().await;
                stats.errors_count += 1;
            }
        }
    }
    
    /// 解析消息
    /// 
    /// 假设消息格式为 JSON（单帧或多帧）：
    /// 单帧：{"measurement_point_id": "1464", "sequence": 12345, ...}
    /// 多帧：第一帧是主题（可选），后续帧是 JSON 数据
    fn parse_message(message: &ZmqMessage) -> Result<ZeroMQMessage> {
        // 获取消息帧数
        let frame_count = message.len();
        
        // 如果是多帧消息，第一帧可能是主题，数据在后续帧中
        // 如果是单帧消息，直接解析
        let data_frame = if frame_count > 1 {
            // 多帧消息：跳过第一帧（主题），使用第二帧（数据）
            message.get(1).ok_or("Message has no data frame")?
        } else {
            // 单帧消息：直接使用
            message.get(0).ok_or("Message is empty")?
        };
        
        // 将字节转换为字符串
        let data_str = std::str::from_utf8(data_frame)
            .map_err(|e| format!("Failed to convert message to UTF-8: {}", e))?;
        
        // 解析 JSON
        let message: ZeroMQMessage = serde_json::from_str(data_str)
            .map_err(|e| format!("Failed to parse message as JSON: {}", e))?;
        
        // 验证必需字段
        if message.measurement_point_id.is_empty() {
            return Err("measurement_point_id is required".into());
        }
        
        Ok(message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_zeromq_config_default() {
        let config = ZeroMQConfig::default();
        assert_eq!(config.endpoint, "tcp://localhost:5555");
        assert_eq!(config.socket_type, ZeroMQSocketType::Pull);
        assert_eq!(config.receive_timeout_ms, 1000);
        assert_eq!(config.max_buffer_size, 10000);
    }
    
    #[test]
    fn test_parse_message_json() {
        let json = r#"{
            "measurement_point_id": "1464",
            "sequence": 12345,
            "timestamp": 1234567890,
            "payload": {"value": 42.0},
            "metadata": {"source": "sensor"}
        }"#;
        
        let message: ZeroMQMessage = serde_json::from_str(json).unwrap();
        assert_eq!(message.measurement_point_id, "1464");
        assert_eq!(message.sequence, 12345);
        assert_eq!(message.timestamp, 1234567890);
    }
    
    #[tokio::test]
    async fn test_zeromq_source_creation() {
        let config = ZeroMQConfig {
            endpoint: "tcp://localhost:5556".to_string(),
            socket_type: ZeroMQSocketType::Pull,
            receive_timeout_ms: 1000,
            max_buffer_size: 1000,
            subscribe_filter: None,
        };
        
        let source = ZeroMQSource::new(config);
        assert!(source.is_ok());
    }
}
