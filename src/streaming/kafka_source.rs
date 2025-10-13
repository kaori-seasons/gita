//! Kafka数据源集成
//!
//! 提供高性能的Kafka消费者，支持实时数据流处理
//! 针对边缘计算场景优化内存使用和延迟

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, RwLock, mpsc};
use tokio::time;
use serde::{Deserialize, Serialize};
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::ClientConfig;
use rdkafka::Message;
use rdkafka::config::RDKafkaLogLevel;

use super::KafkaConfig;

/// Kafka消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaMessage {
    /// 消息键
    pub key: Option<String>,
    /// 消息值
    pub payload: serde_json::Value,
    /// 主题
    pub topic: String,
    /// 分区
    pub partition: i32,
    /// 偏移量
    pub offset: i64,
    /// 时间戳
    pub timestamp: u64,
    /// 头信息
    pub headers: HashMap<String, String>,
}

/// Kafka数据源
pub struct KafkaSource {
    config: KafkaConfig,
    consumer: Arc<RwLock<Option<StreamConsumer>>>,
    sender: broadcast::Sender<KafkaMessage>,
    receiver: broadcast::Receiver<KafkaMessage>,
    is_running: Arc<RwLock<bool>>,
    stats: Arc<RwLock<KafkaSourceStats>>,
}

/// Kafka数据源统计信息
#[derive(Debug, Clone, Default)]
pub struct KafkaSourceStats {
    /// 总接收消息数
    pub messages_received: u64,
    /// 总处理消息数
    pub messages_processed: u64,
    /// 总错误数
    pub errors_count: u64,
    /// 当前缓冲区大小
    pub buffer_size: usize,
    /// 平均处理延迟
    pub avg_processing_latency_ms: f64,
    /// 最后处理时间
    pub last_message_timestamp: u64,
}

impl KafkaSource {
    /// 创建新的Kafka数据源
    pub async fn new(config: KafkaConfig) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let (sender, receiver) = broadcast::channel(10000); // 10k缓冲区

        Ok(Self {
            config,
            consumer: Arc::new(RwLock::new(None)),
            sender,
            receiver,
            is_running: Arc::new(RwLock::new(false)),
            stats: Arc::new(RwLock::new(KafkaSourceStats::default())),
        })
    }

    /// 启动Kafka消费者
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut is_running = self.is_running.write().await;
        if *is_running {
            return Err("Kafka source is already running".into());
        }

        tracing::info!("Starting Kafka source with topics: {:?}", self.config.topics);

        // 创建Kafka消费者配置
        let mut client_config = ClientConfig::new();
        for server in &self.config.bootstrap_servers {
            client_config.set("bootstrap.servers", server);
        }
        client_config.set("group.id", &self.config.group_id);
        client_config.set("enable.auto.commit", self.config.enable_auto_commit.to_string());
        client_config.set("session.timeout.ms", self.config.session_timeout_ms.to_string());
        client_config.set("heartbeat.interval.ms", self.config.heartbeat_interval_ms.to_string());
        client_config.set("max.poll.interval.ms", "300000"); // 5分钟
        client_config.set("auto.offset.reset", "latest");
        client_config.set_log_level(RDKafkaLogLevel::Info);

        // 创建消费者
        let consumer: StreamConsumer = client_config.create()?;
        consumer.subscribe(&self.config.topics.iter().map(|s| s.as_str()).collect::<Vec<_>>())?;

        // 存储消费者
        let mut consumer_lock = self.consumer.write().await;
        *consumer_lock = Some(consumer);

        *is_running = true;

        // 启动消费循环
        let consumer_arc = self.consumer.clone();
        let sender = self.sender.clone();
        let stats = self.stats.clone();
        let is_running_clone = self.is_running.clone();

        tokio::spawn(async move {
            Self::consume_loop(consumer_arc, sender, stats, is_running_clone).await;
        });

        tracing::info!("Kafka source started successfully");
        Ok(())
    }

    /// 停止Kafka消费者
    pub async fn stop(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut is_running = self.is_running.write().await;
        if !*is_running {
            return Ok(());
        }

        tracing::info!("Stopping Kafka source");

        *is_running = false;

        // 等待一小段时间让消费循环停止
        tokio::time::sleep(Duration::from_millis(100)).await;

        // 停止消费者
        if let Some(consumer) = self.consumer.write().await.as_mut() {
            consumer.unsubscribe();
        }

        tracing::info!("Kafka source stopped successfully");
        Ok(())
    }

    /// 订阅消息流
    pub async fn subscribe(&self) -> Result<broadcast::Receiver<KafkaMessage>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(self.sender.subscribe())
    }

    /// 获取统计信息
    pub async fn get_stats(&self) -> KafkaSourceStats {
        self.stats.read().await.clone()
    }

    /// 消费循环
    async fn consume_loop(
        consumer: Arc<RwLock<Option<StreamConsumer>>>,
        sender: broadcast::Sender<KafkaMessage>,
        stats: Arc<RwLock<KafkaSourceStats>>,
        is_running: Arc<RwLock<bool>>,
    ) {
        tracing::info!("Starting Kafka consumption loop");

        loop {
            // 检查是否应该停止
            if !*is_running.read().await {
                break;
            }

            // 获取消费者
            let consumer_guard = consumer.read().await;
            if consumer_guard.is_none() {
                tokio::time::sleep(Duration::from_millis(100)).await;
                continue;
            }

            let consumer = consumer_guard.as_ref().unwrap();

            // 轮询消息
            match consumer.recv().await {
                Ok(message) => {
                    let start_time = std::time::Instant::now();

                    // 更新统计信息
                    {
                        let mut stats = stats.write().await;
                        stats.messages_received += 1;
                        stats.buffer_size = sender.len();
                    }

                    // 处理消息
                    match Self::process_message(&message).await {
                        Ok(kafka_message) => {
                            // 发送消息到广播通道
                            match sender.send(kafka_message) {
                                Ok(_) => {
                                    let mut stats = stats.write().await;
                                    stats.messages_processed += 1;

                                    // 计算处理延迟
                                    let processing_time = start_time.elapsed().as_millis() as f64;
                                    stats.avg_processing_latency_ms =
                                        (stats.avg_processing_latency_ms * 0.9) + (processing_time * 0.1);
                                    stats.last_message_timestamp = std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap_or_default()
                                        .as_secs();
                                }
                                Err(e) => {
                                    tracing::warn!("Failed to send message to broadcast channel: {}", e);
                                    let mut stats = stats.write().await;
                                    stats.errors_count += 1;
                                }
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Failed to process Kafka message: {}", e);
                            let mut stats = stats.write().await;
                            stats.errors_count += 1;
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Kafka consumer error: {}", e);
                    let mut stats = stats.write().await;
                    stats.errors_count += 1;

                    // 如果是不可恢复的错误，等待一段时间再重试
                    if e.to_string().contains("Broker transport failure") {
                        tokio::time::sleep(Duration::from_secs(5)).await;
                    }
                }
            }
        }

        tracing::info!("Kafka consumption loop stopped");
    }

    /// 处理Kafka消息
    async fn process_message(message: &rdkafka::message::Message) -> Result<KafkaMessage, Box<dyn std::error::Error + Send + Sync>> {
        // 提取消息内容
        let payload_bytes = message.payload().ok_or("Message payload is empty")?;
        let payload: serde_json::Value = serde_json::from_slice(payload_bytes)?;

        // 提取消息头
        let mut headers = HashMap::new();
        if let Some(headers_ref) = message.headers() {
            for header in headers_ref.iter() {
                if let (Some(key), Some(value)) = (header.0, header.1) {
                    headers.insert(key.to_string(), String::from_utf8_lossy(value).to_string());
                }
            }
        }

        let kafka_message = KafkaMessage {
            key: message.key().map(|k| String::from_utf8_lossy(k).to_string()),
            payload,
            topic: message.topic().to_string(),
            partition: message.partition(),
            offset: message.offset(),
            timestamp: message.timestamp().to_millis().unwrap_or(0) as u64,
            headers,
        };

        Ok(kafka_message)
    }
}

impl Drop for KafkaSource {
    fn drop(&mut self) {
        // 确保消费者被正确清理
        if let Ok(mut consumer) = self.consumer.try_write() {
            if let Some(c) = consumer.as_mut() {
                let _ = c.unsubscribe();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kafka_config_default() {
        let config = KafkaConfig {
            bootstrap_servers: vec!["localhost:9092".to_string()],
            group_id: "test-group".to_string(),
            topics: vec!["test-topic".to_string()],
            enable_auto_commit: true,
            session_timeout_ms: 30000,
            heartbeat_interval_ms: 3000,
            max_poll_records: 1000,
            auto_commit_interval_ms: 5000,
        };

        assert_eq!(config.bootstrap_servers.len(), 1);
        assert_eq!(config.group_id, "test-group");
        assert!(config.enable_auto_commit);
    }

    #[tokio::test]
    async fn test_kafka_source_creation() {
        let config = KafkaConfig {
            bootstrap_servers: vec!["localhost:9092".to_string()],
            group_id: "test-group".to_string(),
            topics: vec!["test-topic".to_string()],
            enable_auto_commit: true,
            session_timeout_ms: 30000,
            heartbeat_interval_ms: 3000,
            max_poll_records: 1000,
            auto_commit_interval_ms: 5000,
        };

        // 注意：这个测试在没有Kafka服务器的情况下会失败
        // 在实际环境中需要设置正确的Kafka配置
        let result = KafkaSource::new(config).await;
        assert!(result.is_ok());
    }
}
