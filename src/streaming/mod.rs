//! 实时流式计算模块
//!
//! 提供低延迟、高吞吐量的流式数据处理能力，专为边缘计算场景优化

pub mod kafka_source;
pub mod stream_processor;
pub mod plugin_chain;
pub mod data_flow;
pub mod backpressure;
pub mod metrics;
pub mod edge_optimization;
pub mod high_availability;
pub mod state_recovery;
pub mod garbage_collector;

// 重新导出主要类型
pub use kafka_source::{KafkaSource, KafkaMessage, KafkaSourceStats};
pub use stream_processor::{StreamProcessor, StreamProcessorStats};
pub use plugin_chain::{PluginChainExecutor, PluginChainConfig, ExecutionStrategy};
pub use data_flow::{DataFlowManager, DataFlowConfig, DataFlowContext};
pub use backpressure::{BackpressureManager, BackpressureConfig, BackpressureStrategy};
pub use metrics::{MetricsCollector, StreamingMetrics, MonitoringConfig};
pub use edge_optimization::{EdgeOptimizationManager, EdgeOptimizationConfig};
pub use high_availability::{HighAvailabilityManager, HighAvailabilityConfig, NodeInfo, NodeStatus};
pub use state_recovery::{StateRecoveryManager, SnapshotConfig, RecoveryConfig, SystemSnapshot};
pub use garbage_collector::{GarbageCollector, GCConfig, GCStrategy, GCMetrics};

use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

/// 流式计算配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingConfig {
    /// Kafka配置
    pub kafka: KafkaConfig,
    /// 流处理配置
    pub processing: ProcessingConfig,
    /// 资源配置
    pub resources: ResourceConfig,
    /// 监控配置
    pub monitoring: MonitoringConfig,
}

/// Kafka数据源配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaConfig {
    /// Bootstrap服务器列表
    pub bootstrap_servers: Vec<String>,
    /// 消费者组ID
    pub group_id: String,
    /// 主题列表
    pub topics: Vec<String>,
    /// 自动提交偏移量
    pub enable_auto_commit: bool,
    /// 会话超时时间
    pub session_timeout_ms: u64,
    /// 心跳间隔
    pub heartbeat_interval_ms: u64,
    /// 最大轮询记录数
    pub max_poll_records: usize,
    /// 自动提交间隔
    pub auto_commit_interval_ms: u64,
}

/// 流处理配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingConfig {
    /// 批处理大小
    pub batch_size: usize,
    /// 处理超时时间
    pub processing_timeout_ms: u64,
    /// 最大并发处理数
    pub max_concurrent_processes: usize,
    /// 缓冲区大小
    pub buffer_size: usize,
    /// 背压阈值
    pub backpressure_threshold: f64,
    /// 插件链配置
    pub plugin_chain: Vec<PluginConfig>,
}

/// 插件配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    /// 插件名称
    pub name: String,
    /// 插件版本
    pub version: String,
    /// 执行顺序
    pub order: usize,
    /// 资源需求
    pub resource_requirements: ResourceRequirements,
    /// 超时时间
    pub timeout_ms: u64,
    /// 是否启用缓存
    pub enable_caching: bool,
    /// 缓存TTL
    pub cache_ttl_seconds: u64,
}

/// 资源配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceConfig {
    /// CPU核心限制
    pub cpu_cores_limit: f64,
    /// 内存限制(MB)
    pub memory_limit_mb: u64,
    /// 磁盘缓存大小(MB)
    pub disk_cache_size_mb: u64,
    /// 网络带宽限制(Mbps)
    pub network_bandwidth_mbps: Option<u64>,
    /// 启用内存映射文件
    pub enable_memory_mapping: bool,
    /// 启用压缩
    pub enable_compression: bool,
}

/// 监控配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// 启用详细指标收集
    pub enable_detailed_metrics: bool,
    /// 指标收集间隔
    pub metrics_interval_ms: u64,
    /// 启用健康检查
    pub enable_health_checks: bool,
    /// 健康检查间隔
    pub health_check_interval_ms: u64,
    /// 启用性能分析
    pub enable_performance_profiling: bool,
}

/// 资源需求配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    /// CPU核心数
    pub cpu_cores: f64,
    /// 内存使用(MB)
    pub memory_mb: u64,
    /// 磁盘空间(MB)
    pub disk_mb: u64,
}

/// 流式计算管理器
pub struct StreamingManager {
    config: StreamingConfig,
    kafka_source: Arc<kafka_source::KafkaSource>,
    stream_processor: Arc<stream_processor::StreamProcessor>,
    metrics_collector: Arc<metrics::MetricsCollector>,
    is_running: Arc<RwLock<bool>>,
}

impl StreamingManager {
    /// 创建新的流式计算管理器
    pub async fn new(config: StreamingConfig) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let kafka_source = Arc::new(kafka_source::KafkaSource::new(config.kafka.clone()).await?);
        let stream_processor = Arc::new(stream_processor::StreamProcessor::new(
            config.processing.clone(),
            config.resources.clone(),
        ).await?);
        let metrics_collector = Arc::new(metrics::MetricsCollector::new(config.monitoring.clone()));

        Ok(Self {
            config,
            kafka_source,
            stream_processor,
            metrics_collector,
            is_running: Arc::new(RwLock::new(false)),
        })
    }

    /// 启动流式计算
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut is_running = self.is_running.write().await;
        if *is_running {
            return Err("Streaming manager is already running".into());
        }
        *is_running = true;

        tracing::info!("Starting streaming computation manager");

        // 启动各个组件
        self.kafka_source.start().await?;
        self.stream_processor.start().await?;
        self.metrics_collector.start().await?;

        // 建立数据流
        self.setup_data_flow().await?;

        tracing::info!("Streaming computation manager started successfully");
        Ok(())
    }

    /// 停止流式计算
    pub async fn stop(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut is_running = self.is_running.write().await;
        if !*is_running {
            return Ok(());
        }
        *is_running = false;

        tracing::info!("Stopping streaming computation manager");

        // 停止各个组件
        self.metrics_collector.stop().await?;
        self.stream_processor.stop().await?;
        self.kafka_source.stop().await?;

        tracing::info!("Streaming computation manager stopped successfully");
        Ok(())
    }

    /// 获取运行状态
    pub async fn is_running(&self) -> bool {
        *self.is_running.read().await
    }

    /// 获取指标
    pub async fn get_metrics(&self) -> metrics::StreamingMetrics {
        self.metrics_collector.get_metrics().await
    }

    /// 设置数据流
    async fn setup_data_flow(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // 从Kafka到流处理器的连接
        let kafka_receiver = self.kafka_source.subscribe().await?;
        self.stream_processor.connect_source(kafka_receiver).await?;

        Ok(())
    }
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            kafka: KafkaConfig {
                bootstrap_servers: vec!["localhost:9092".to_string()],
                group_id: "edge-compute-streaming".to_string(),
                topics: vec!["sensor-data".to_string()],
                enable_auto_commit: true,
                session_timeout_ms: 30000,
                heartbeat_interval_ms: 3000,
                max_poll_records: 1000,
                auto_commit_interval_ms: 5000,
            },
            processing: ProcessingConfig {
                batch_size: 100,
                processing_timeout_ms: 5000,
                max_concurrent_processes: 4,
                buffer_size: 10000,
                backpressure_threshold: 0.8,
                plugin_chain: vec![
                    PluginConfig {
                        name: "vibrate31".to_string(),
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
                    },
                    PluginConfig {
                        name: "anomaly_detector".to_string(),
                        version: "1.0.0".to_string(),
                        order: 1,
                        resource_requirements: ResourceRequirements {
                            cpu_cores: 0.5,
                            memory_mb: 128,
                            disk_mb: 50,
                        },
                        timeout_ms: 1000,
                        enable_caching: false,
                        cache_ttl_seconds: 0,
                    },
                ],
            },
            resources: ResourceConfig {
                cpu_cores_limit: 3.0,
                memory_limit_mb: 4096,
                disk_cache_size_mb: 1024,
                network_bandwidth_mbps: Some(100),
                enable_memory_mapping: true,
                enable_compression: true,
            },
            monitoring: MonitoringConfig {
                enable_detailed_metrics: true,
                metrics_interval_ms: 1000,
                enable_health_checks: true,
                health_check_interval_ms: 5000,
                enable_performance_profiling: false,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_streaming_config_default() {
        let config = StreamingConfig::default();

        assert_eq!(config.kafka.bootstrap_servers.len(), 1);
        assert_eq!(config.processing.plugin_chain.len(), 2);
        assert!(config.resources.enable_memory_mapping);
    }

    #[test]
    fn test_plugin_config() {
        let plugin = PluginConfig {
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

        assert_eq!(plugin.name, "test_plugin");
        assert!(plugin.enable_caching);
    }
}
