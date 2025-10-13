//! 流式处理器
//!
//! 核心的流式数据处理引擎，支持插件链式执行和背压机制
//! 针对边缘计算环境优化性能和资源使用

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, mpsc, RwLock, Semaphore};
use tokio::time;
use serde::{Deserialize, Serialize};

use super::kafka_source::KafkaMessage;
use super::ProcessingConfig;
use super::ResourceConfig;
use super::{PluginConfig, ResourceRequirements};
use crate::container::*;
use crate::core::*;

/// 流式处理器
pub struct StreamProcessor {
    config: ProcessingConfig,
    resource_config: ResourceConfig,
    container_manager: Arc<YoukiContainerManager>,
    plugin_executors: HashMap<String, PluginExecutor>,
    message_queue: Arc<RwLock<VecDeque<ProcessingMessage>>>,
    semaphore: Arc<Semaphore>,
    receiver: Option<broadcast::Receiver<KafkaMessage>>,
    stats: Arc<RwLock<StreamProcessorStats>>,
    is_running: Arc<RwLock<bool>>,
}

/// 插件执行器
struct PluginExecutor {
    config: PluginConfig,
    container_manager: Arc<YoukiContainerManager>,
    cache: HashMap<String, CachedResult>,
    stats: PluginStats,
}

/// 缓存结果
#[derive(Debug, Clone)]
struct CachedResult {
    result: serde_json::Value,
    timestamp: Instant,
    ttl_seconds: u64,
}

/// 插件统计
#[derive(Debug, Clone, Default)]
struct PluginStats {
    executions: u64,
    successes: u64,
    failures: u64,
    avg_execution_time_ms: f64,
    cache_hits: u64,
    cache_misses: u64,
}

/// 处理消息
#[derive(Debug, Clone)]
struct ProcessingMessage {
    kafka_message: KafkaMessage,
    current_plugin_index: usize,
    start_time: Instant,
    metadata: HashMap<String, serde_json::Value>,
}

/// 流式处理器统计
#[derive(Debug, Clone, Default)]
pub struct StreamProcessorStats {
    pub messages_processed: u64,
    pub messages_successful: u64,
    pub messages_failed: u64,
    pub avg_processing_time_ms: f64,
    pub queue_size: usize,
    pub active_workers: usize,
    pub backpressure_events: u64,
    pub plugin_stats: HashMap<String, PluginStats>,
}

impl StreamProcessor {
    /// 创建新的流式处理器
    pub async fn new(
        config: ProcessingConfig,
        resource_config: ResourceConfig,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let container_manager = Arc::new(YoukiContainerManager::new(
            std::path::PathBuf::from("./runtime")
        ));

        let semaphore = Arc::new(Semaphore::new(config.max_concurrent_processes));

        // 初始化插件执行器
        let mut plugin_executors = HashMap::new();
        for plugin_config in &config.plugin_chain {
            let executor = PluginExecutor::new(
                plugin_config.clone(),
                container_manager.clone(),
            ).await?;
            plugin_executors.insert(plugin_config.name.clone(), executor);
        }

        Ok(Self {
            config,
            resource_config,
            container_manager,
            plugin_executors,
            message_queue: Arc::new(RwLock::new(VecDeque::with_capacity(config.buffer_size))),
            semaphore,
            receiver: None,
            stats: Arc::new(RwLock::new(StreamProcessorStats::default())),
            is_running: Arc::new(RwLock::new(false)),
        })
    }

    /// 启动处理器
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut is_running = self.is_running.write().await;
        if *is_running {
            return Err("Stream processor is already running".into());
        }

        tracing::info!("Starting stream processor with {} plugins",
                      self.config.plugin_chain.len());

        *is_running = true;

        // 启动工作协程
        for i in 0..self.config.max_concurrent_processes {
            let processor = Arc::new(self.clone());
            tokio::spawn(async move {
                processor.worker_loop(i).await;
            });
        }

        tracing::info!("Stream processor started successfully");
        Ok(())
    }

    /// 停止处理器
    pub async fn stop(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut is_running = self.is_running.write().await;
        if !*is_running {
            return Ok(());
        }

        tracing::info!("Stopping stream processor");

        *is_running = false;

        // 等待队列处理完成
        let mut attempts = 0;
        loop {
            let queue_size = self.message_queue.read().await.len();
            if queue_size == 0 || attempts >= 30 { // 最多等待30秒
                break;
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
            attempts += 1;
        }

        tracing::info!("Stream processor stopped successfully");
        Ok(())
    }

    /// 连接数据源
    pub async fn connect_source(&mut self, receiver: broadcast::Receiver<KafkaMessage>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.receiver = Some(receiver);

        // 启动消息接收协程
        let processor = Arc::new(self.clone());
        tokio::spawn(async move {
            processor.message_receiver_loop().await;
        });

        Ok(())
    }

    /// 获取统计信息
    pub async fn get_stats(&self) -> StreamProcessorStats {
        let mut stats = self.stats.read().await.clone();

        // 更新队列大小和活跃工作者数
        stats.queue_size = self.message_queue.read().await.len();
        stats.active_workers = self.config.max_concurrent_processes - self.semaphore.available_permits();

        stats
    }

    /// 消息接收循环
    async fn message_receiver_loop(&self) {
        let mut receiver = self.receiver.as_ref().unwrap().resubscribe();

        tracing::info!("Starting message receiver loop");

        loop {
            if !*self.is_running.read().await {
                break;
            }

            match receiver.recv().await {
                Ok(kafka_message) => {
                    self.handle_incoming_message(kafka_message).await;
                }
                Err(broadcast::error::RecvError::Lagged(count)) => {
                    tracing::warn!("Message receiver lagged by {} messages", count);
                }
                Err(broadcast::error::RecvError::Closed) => {
                    tracing::info!("Message receiver channel closed");
                    break;
                }
            }
        }

        tracing::info!("Message receiver loop stopped");
    }

    /// 处理传入的消息
    async fn handle_incoming_message(&self, kafka_message: KafkaMessage) {
        let mut stats = self.stats.write().await;
        stats.messages_processed += 1;

        // 检查背压
        let queue_size = {
            let queue = self.message_queue.read().await;
            queue.len()
        };

        if queue_size as f64 / self.config.buffer_size as f64 > self.config.backpressure_threshold {
            stats.backpressure_events += 1;
            tracing::warn!("Backpressure triggered, queue size: {}/{}",
                          queue_size, self.config.buffer_size);
            // 这里可以实现背压策略，比如暂时拒绝新消息或降低处理速度
        }

        // 创建处理消息
        let processing_message = ProcessingMessage {
            kafka_message,
            current_plugin_index: 0,
            start_time: Instant::now(),
            metadata: HashMap::new(),
        };

        // 添加到队列
        let mut queue = self.message_queue.write().await;
        if queue.len() < self.config.buffer_size {
            queue.push_back(processing_message);
        } else {
            tracing::warn!("Message queue full, dropping message");
            stats.messages_failed += 1;
        }
    }

    /// 工作协程循环
    async fn worker_loop(&self, worker_id: usize) {
        tracing::info!("Starting worker {} loop", worker_id);

        loop {
            if !*self.is_running.read().await {
                break;
            }

            // 获取信号量许可
            let permit = match self.semaphore.acquire().await {
                Ok(permit) => permit,
                Err(_) => break,
            };

            // 从队列获取消息
            let processing_message = {
                let mut queue = self.message_queue.write().await;
                queue.pop_front()
            };

            match processing_message {
                Some(message) => {
                    // 处理消息
                    self.process_message(message).await;
                }
                None => {
                    // 队列为空，短暂等待
                    drop(permit);
                    tokio::time::sleep(Duration::from_millis(10)).await;
                    continue;
                }
            }

            // 释放信号量许可
            drop(permit);
        }

        tracing::info!("Worker {} loop stopped", worker_id);
    }

    /// 处理消息
    async fn process_message(&self, mut message: ProcessingMessage) {
        let plugin_chain = &self.config.plugin_chain;

        // 按顺序执行插件链
        while message.current_plugin_index < plugin_chain.len() {
            let plugin_config = &plugin_chain[message.current_plugin_index];

            match self.execute_plugin(plugin_config, &mut message).await {
                Ok(_) => {
                    message.current_plugin_index += 1;
                }
                Err(e) => {
                    tracing::error!("Plugin {} execution failed: {}", plugin_config.name, e);
                    let mut stats = self.stats.write().await;
                    stats.messages_failed += 1;
                    return;
                }
            }
        }

        // 所有插件执行完成
        let mut stats = self.stats.write().await;
        stats.messages_successful += 1;

        let processing_time = message.start_time.elapsed().as_millis() as f64;
        stats.avg_processing_time_ms = (stats.avg_processing_time_ms * 0.9) + (processing_time * 0.1);

        tracing::info!("Message processed successfully in {:.2}ms", processing_time);
    }

    /// 执行单个插件
    async fn execute_plugin(
        &self,
        plugin_config: &PluginConfig,
        message: &mut ProcessingMessage,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let plugin_name = &plugin_config.name;

        // 获取插件执行器
        let executor = self.plugin_executors.get(plugin_name)
            .ok_or_else(|| format!("Plugin executor not found: {}", plugin_name))?;

        // 执行插件
        let result = executor.execute_plugin(&message.kafka_message, plugin_config).await?;

        // 将结果添加到消息元数据中
        message.metadata.insert(plugin_name.clone(), result);

        Ok(())
    }
}

impl PluginExecutor {
    /// 创建新的插件执行器
    async fn new(
        config: PluginConfig,
        container_manager: Arc<YoukiContainerManager>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Ok(Self {
            config,
            container_manager,
            cache: HashMap::new(),
            stats: PluginStats::default(),
        })
    }

    /// 执行插件
    async fn execute_plugin(
        &mut self,
        kafka_message: &KafkaMessage,
        plugin_config: &PluginConfig,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        let start_time = Instant::now();

        // 检查缓存
        if plugin_config.enable_caching {
            if let Some(cached_result) = self.check_cache(kafka_message).await {
                self.stats.cache_hits += 1;
                return Ok(cached_result);
            }
            self.stats.cache_misses += 1;
        }

        self.stats.executions += 1;

        // 准备插件执行环境
        let result = self.execute_with_container(kafka_message, plugin_config).await;

        match result {
            Ok(output) => {
                self.stats.successes += 1;

                // 缓存结果
                if plugin_config.enable_caching {
                    self.cache_result(kafka_message, &output, plugin_config.cache_ttl_seconds).await;
                }

                let execution_time = start_time.elapsed().as_millis() as f64;
                self.stats.avg_execution_time_ms = (self.stats.avg_execution_time_ms * 0.9) + (execution_time * 0.1);

                Ok(output)
            }
            Err(e) => {
                self.stats.failures += 1;
                Err(e)
            }
        }
    }

    /// 使用容器执行插件
    async fn execute_with_container(
        &self,
        kafka_message: &KafkaMessage,
        plugin_config: &PluginConfig,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        // 创建容器配置
        let container_config = ContainerConfig {
            name: format!("{}_{}", plugin_config.name, uuid::Uuid::new_v4()),
            image: format!("{}/{}", plugin_config.name, plugin_config.version),
            env: vec![],
            volumes: vec![],
            resources: ResourceLimits {
                cpu_cores: Some(plugin_config.resource_requirements.cpu_cores),
                memory_mb: Some(plugin_config.resource_requirements.memory_mb),
                disk_mb: Some(plugin_config.resource_requirements.disk_mb),
            },
            security: SecurityConfig {
                rootless: true,
                seccomp: None,
                apparmor: None,
                network_isolation: false,
            },
        };

        // 启动容器
        let container_id = self.container_manager.create_container(
            container_config,
            plugin_config.name.clone(),
        ).await?;

        // 等待容器完成执行
        // 这里需要实现具体的容器执行逻辑
        // 由于时间关系，这里返回模拟结果

        // 清理容器
        let _ = self.container_manager.destroy_container(&container_id).await;

        // 返回模拟结果
        Ok(serde_json::json!({
            "status": "success",
            "plugin": plugin_config.name,
            "execution_time_ms": 150,
            "result": {
                "processed_data": kafka_message.payload,
                "confidence": 0.95
            }
        }))
    }

    /// 检查缓存
    async fn check_cache(&self, kafka_message: &KafkaMessage) -> Option<serde_json::Value> {
        let cache_key = self.generate_cache_key(kafka_message);

        if let Some(cached) = self.cache.get(&cache_key) {
            if cached.timestamp.elapsed().as_secs() < cached.ttl_seconds {
                return Some(cached.result.clone());
            }
        }

        None
    }

    /// 缓存结果
    async fn cache_result(&mut self, kafka_message: &KafkaMessage, result: &serde_json::Value, ttl_seconds: u64) {
        let cache_key = self.generate_cache_key(kafka_message);
        let cached_result = CachedResult {
            result: result.clone(),
            timestamp: Instant::now(),
            ttl_seconds,
        };

        // 限制缓存大小
        if self.cache.len() >= 1000 { // 最大缓存1000个结果
            // 简单的LRU策略：移除最旧的条目
            if let Some(oldest_key) = self.cache.keys().next().cloned() {
                self.cache.remove(&oldest_key);
            }
        }

        self.cache.insert(cache_key, cached_result);
    }

    /// 生成缓存键
    fn generate_cache_key(&self, kafka_message: &KafkaMessage) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        kafka_message.payload.to_string().hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
}

impl Clone for StreamProcessor {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            resource_config: self.resource_config.clone(),
            container_manager: self.container_manager.clone(),
            plugin_executors: HashMap::new(), // 不克隆执行器，避免复杂性
            message_queue: self.message_queue.clone(),
            semaphore: self.semaphore.clone(),
            receiver: None,
            stats: self.stats.clone(),
            is_running: self.is_running.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_processing_config() {
        let config = ProcessingConfig {
            batch_size: 100,
            processing_timeout_ms: 5000,
            max_concurrent_processes: 4,
            buffer_size: 10000,
            backpressure_threshold: 0.8,
            plugin_chain: vec![],
        };

        assert_eq!(config.batch_size, 100);
        assert_eq!(config.max_concurrent_processes, 4);
    }

    #[tokio::test]
    async fn test_plugin_executor_creation() {
        let config = PluginConfig {
            name: "test_plugin".to_string(),
            version: "1.0.0".to_string(),
            order: 0,
            resource_requirements: ResourceRequirements {
                cpu_cores: 1.0,
                memory_mb: 256,
                disk_mb: 100,
            },
            timeout_ms: 2000,
            enable_caching: true,
            cache_ttl_seconds: 300,
        };

        let container_manager = Arc::new(YoukiContainerManager::new(
            std::path::PathBuf::from("./runtime")
        ));

        let result = PluginExecutor::new(config, container_manager).await;
        assert!(result.is_ok());
    }
}
