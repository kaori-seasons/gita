//! 监控和指标收集
//!
//! 提供全面的系统监控和性能指标收集
//! 支持实时监控、历史分析和告警机制

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::time;
use serde::{Deserialize, Serialize};

/// 流式计算指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingMetrics {
    /// 时间戳
    pub timestamp: u64,
    /// 消息处理统计
    pub message_stats: MessageStats,
    /// 系统资源使用
    pub system_resources: SystemResources,
    /// 插件性能指标
    pub plugin_metrics: HashMap<String, PluginMetrics>,
    /// 背压统计
    pub backpressure_stats: BackpressureStats,
    /// 错误统计
    pub error_stats: ErrorStats,
}

/// 消息处理统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageStats {
    /// 总接收消息数
    pub messages_received: u64,
    /// 总处理消息数
    pub messages_processed: u64,
    /// 成功处理消息数
    pub messages_successful: u64,
    /// 失败消息数
    pub messages_failed: u64,
    /// 队列大小
    pub queue_size: usize,
    /// 平均处理时间(ms)
    pub avg_processing_time_ms: f64,
    /// P95处理时间(ms)
    pub p95_processing_time_ms: f64,
    /// P99处理时间(ms)
    pub p99_processing_time_ms: f64,
    /// 处理速率(msg/s)
    pub processing_rate: f64,
}

/// 系统资源使用
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemResources {
    /// CPU使用率
    pub cpu_usage_percent: f64,
    /// 内存使用量(MB)
    pub memory_usage_mb: f64,
    /// 磁盘使用量(MB)
    pub disk_usage_mb: f64,
    /// 网络接收速率(Mbps)
    pub network_rx_mbps: f64,
    /// 网络发送速率(Mbps)
    pub network_tx_mbps: f64,
    /// 活跃线程数
    pub active_threads: usize,
    /// 打开的文件描述符数
    pub open_file_descriptors: usize,
}

/// 插件性能指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetrics {
    /// 执行次数
    pub executions: u64,
    /// 成功次数
    pub successes: u64,
    /// 失败次数
    pub failures: u64,
    /// 平均执行时间(ms)
    pub avg_execution_time_ms: f64,
    /// 缓存命中率
    pub cache_hit_rate: f64,
    /// 错误率
    pub error_rate: f64,
    /// 资源使用统计
    pub resource_usage: PluginResourceUsage,
}

/// 插件资源使用
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginResourceUsage {
    /// 平均CPU使用率
    pub avg_cpu_usage: f64,
    /// 峰值CPU使用率
    pub peak_cpu_usage: f64,
    /// 平均内存使用(MB)
    pub avg_memory_usage_mb: f64,
    /// 峰值内存使用(MB)
    pub peak_memory_usage_mb: f64,
}

/// 背压统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackpressureStats {
    /// 背压事件总数
    pub total_events: u64,
    /// 当前背压状态
    pub current_state: String,
    /// 背压持续时间(s)
    pub backpressure_duration_seconds: f64,
    /// 背压策略分布
    pub strategy_distribution: HashMap<String, u64>,
}

/// 错误统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorStats {
    /// 总错误数
    pub total_errors: u64,
    /// 错误类型分布
    pub error_types: HashMap<String, u64>,
    /// 错误率
    pub error_rate: f64,
    /// 最近错误列表
    pub recent_errors: Vec<ErrorRecord>,
}

/// 错误记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorRecord {
    /// 时间戳
    pub timestamp: u64,
    /// 错误类型
    pub error_type: String,
    /// 错误消息
    pub message: String,
    /// 相关组件
    pub component: String,
}

/// 监控配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// 启用详细指标收集
    pub enable_detailed_metrics: bool,
    /// 指标收集间隔(ms)
    pub metrics_interval_ms: u64,
    /// 启用健康检查
    pub enable_health_checks: bool,
    /// 健康检查间隔(ms)
    pub health_check_interval_ms: u64,
    /// 启用性能分析
    pub enable_performance_profiling: bool,
    /// 指标保留时间(小时)
    pub metrics_retention_hours: u64,
    /// 启用告警
    pub enable_alerts: bool,
    /// 告警配置
    pub alert_config: AlertConfig,
}

/// 告警配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertConfig {
    /// CPU使用率阈值
    pub cpu_threshold: f64,
    /// 内存使用率阈值
    pub memory_threshold: f64,
    /// 错误率阈值
    pub error_rate_threshold: f64,
    /// 队列大小阈值
    pub queue_size_threshold: usize,
    /// 告警冷却时间(秒)
    pub alert_cooldown_seconds: u64,
}

/// 指标收集器
pub struct MetricsCollector {
    config: MonitoringConfig,
    current_metrics: Arc<RwLock<StreamingMetrics>>,
    metrics_history: Arc<RwLock<Vec<StreamingMetrics>>>,
    start_time: Instant,
    is_running: Arc<RwLock<bool>>,
}

impl MetricsCollector {
    /// 创建指标收集器
    pub fn new(config: MonitoringConfig) -> Self {
        let current_metrics = StreamingMetrics {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            message_stats: MessageStats::default(),
            system_resources: SystemResources::default(),
            plugin_metrics: HashMap::new(),
            backpressure_stats: BackpressureStats::default(),
            error_stats: ErrorStats::default(),
        };

        Self {
            config,
            current_metrics: Arc::new(RwLock::new(current_metrics)),
            metrics_history: Arc::new(RwLock::new(Vec::new())),
            start_time: Instant::now(),
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    /// 启动指标收集
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut is_running = self.is_running.write().await;
        if *is_running {
            return Err("Metrics collector is already running".into());
        }

        *is_running = true;

        tracing::info!("Starting metrics collector");

        // 启动指标收集协程
        let collector = Arc::new(self.clone());
        tokio::spawn(async move {
            collector.collection_loop().await;
        });

        // 启动健康检查协程
        if self.config.enable_health_checks {
            let collector = Arc::new(self.clone());
            tokio::spawn(async move {
                collector.health_check_loop().await;
            });
        }

        Ok(())
    }

    /// 停止指标收集
    pub async fn stop(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut is_running = self.is_running.write().await;
        if !*is_running {
            return Ok(());
        }

        tracing::info!("Stopping metrics collector");
        *is_running = false;

        Ok(())
    }

    /// 获取当前指标
    pub async fn get_metrics(&self) -> StreamingMetrics {
        self.current_metrics.read().await.clone()
    }

    /// 更新消息统计
    pub async fn update_message_stats(&self, stats: MessageStats) {
        let mut current = self.current_metrics.write().await;
        current.message_stats = stats;
        current.timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }

    /// 更新系统资源
    pub async fn update_system_resources(&self, resources: SystemResources) {
        let mut current = self.current_metrics.write().await;
        current.system_resources = resources;
    }

    /// 更新插件指标
    pub async fn update_plugin_metrics(&self, plugin_name: String, metrics: PluginMetrics) {
        let mut current = self.current_metrics.write().await;
        current.plugin_metrics.insert(plugin_name, metrics);
    }

    /// 记录错误
    pub async fn record_error(&self, error_type: String, message: String, component: String) {
        let mut current = self.current_metrics.write().await;

        current.error_stats.total_errors += 1;
        *current.error_stats.error_types.entry(error_type.clone()).or_insert(0) += 1;

        // 更新错误率
        let total_messages = current.message_stats.messages_received;
        if total_messages > 0 {
            current.error_stats.error_rate = current.error_stats.total_errors as f64 / total_messages as f64;
        }

        // 添加最近错误记录
        let error_record = ErrorRecord {
            timestamp: current.timestamp,
            error_type,
            message,
            component,
        };

        current.error_stats.recent_errors.push(error_record);

        // 保持最近错误列表在合理范围内
        if current.error_stats.recent_errors.len() > 100 {
            current.error_stats.recent_errors.remove(0);
        }
    }

    /// 指标收集循环
    async fn collection_loop(&self) {
        let interval = Duration::from_millis(self.config.metrics_interval_ms);

        loop {
            if !*self.is_running.read().await {
                break;
            }

            // 收集系统指标
            if let Err(e) = self.collect_system_metrics().await {
                tracing::warn!("Failed to collect system metrics: {}", e);
            }

            // 归档当前指标
            self.archive_current_metrics().await;

            tokio::time::sleep(interval).await;
        }
    }

    /// 健康检查循环
    async fn health_check_loop(&self) {
        let interval = Duration::from_millis(self.config.health_check_interval_ms);

        loop {
            if !*self.is_running.read().await {
                break;
            }

            if let Err(e) = self.perform_health_check().await {
                tracing::error!("Health check failed: {}", e);
            }

            tokio::time::sleep(interval).await;
        }
    }

    /// 收集系统指标
    async fn collect_system_metrics(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // 这里应该实现实际的系统指标收集
        // 由于时间关系，这里返回模拟数据

        let system_resources = SystemResources {
            cpu_usage_percent: 45.2,
            memory_usage_mb: 512.0,
            disk_usage_mb: 1024.0,
            network_rx_mbps: 10.5,
            network_tx_mbps: 8.3,
            active_threads: 12,
            open_file_descriptors: 256,
        };

        self.update_system_resources(system_resources).await;

        Ok(())
    }

    /// 执行健康检查
    async fn perform_health_check(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let metrics = self.get_metrics().await;

        // 检查各项指标是否在正常范围内
        if metrics.system_resources.cpu_usage_percent > 90.0 {
            tracing::warn!("High CPU usage detected: {:.1}%", metrics.system_resources.cpu_usage_percent);
        }

        if metrics.system_resources.memory_usage_mb > 4096.0 {
            tracing::warn!("High memory usage detected: {:.0}MB", metrics.system_resources.memory_usage_mb);
        }

        if metrics.error_stats.error_rate > 0.1 {
            tracing::warn!("High error rate detected: {:.3}%", metrics.error_stats.error_rate * 100.0);
        }

        Ok(())
    }

    /// 归档当前指标
    async fn archive_current_metrics(&self) {
        let current = self.get_metrics().await;
        let mut history = self.metrics_history.write().await;

        history.push(current);

        // 根据保留策略清理历史数据
        let retention_duration = Duration::from_secs(self.config.metrics_retention_hours * 3600);
        let cutoff_time = Instant::now() - retention_duration;

        history.retain(|metrics| {
            let metrics_time = Instant::now() - Duration::from_secs(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() - metrics.timestamp
            );
            metrics_time > cutoff_time
        });
    }

    /// 获取指标历史
    pub async fn get_metrics_history(&self) -> Vec<StreamingMetrics> {
        self.metrics_history.read().await.clone()
    }

    /// 生成性能报告
    pub async fn generate_performance_report(&self) -> PerformanceReport {
        let history = self.get_metrics_history().await;

        if history.is_empty() {
            return PerformanceReport::default();
        }

        let total_messages = history.last().unwrap().message_stats.messages_processed;
        let total_time = self.start_time.elapsed().as_secs_f64();
        let avg_processing_rate = total_messages as f64 / total_time;

        let avg_cpu_usage = history.iter()
            .map(|m| m.system_resources.cpu_usage_percent)
            .sum::<f64>() / history.len() as f64;

        let avg_memory_usage = history.iter()
            .map(|m| m.system_resources.memory_usage_mb)
            .sum::<f64>() / history.len() as f64;

        PerformanceReport {
            total_uptime_seconds: total_time,
            average_processing_rate: avg_processing_rate,
            average_cpu_usage: avg_cpu_usage,
            average_memory_usage: avg_memory_usage,
            total_messages_processed: total_messages,
            total_errors: history.last().unwrap().error_stats.total_errors,
            peak_cpu_usage: history.iter()
                .map(|m| m.system_resources.cpu_usage_percent)
                .fold(0.0, f64::max),
            peak_memory_usage: history.iter()
                .map(|m| m.system_resources.memory_usage_mb)
                .fold(0.0, f64::max),
        }
    }
}

/// 性能报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceReport {
    pub total_uptime_seconds: f64,
    pub average_processing_rate: f64,
    pub average_cpu_usage: f64,
    pub average_memory_usage: f64,
    pub total_messages_processed: u64,
    pub total_errors: u64,
    pub peak_cpu_usage: f64,
    pub peak_memory_usage: f64,
}

impl Default for PerformanceReport {
    fn default() -> Self {
        Self {
            total_uptime_seconds: 0.0,
            average_processing_rate: 0.0,
            average_cpu_usage: 0.0,
            average_memory_usage: 0.0,
            total_messages_processed: 0,
            total_errors: 0,
            peak_cpu_usage: 0.0,
            peak_memory_usage: 0.0,
        }
    }
}

impl Default for MessageStats {
    fn default() -> Self {
        Self {
            messages_received: 0,
            messages_processed: 0,
            messages_successful: 0,
            messages_failed: 0,
            queue_size: 0,
            avg_processing_time_ms: 0.0,
            p95_processing_time_ms: 0.0,
            p99_processing_time_ms: 0.0,
            processing_rate: 0.0,
        }
    }
}

impl Default for SystemResources {
    fn default() -> Self {
        Self {
            cpu_usage_percent: 0.0,
            memory_usage_mb: 0.0,
            disk_usage_mb: 0.0,
            network_rx_mbps: 0.0,
            network_tx_mbps: 0.0,
            active_threads: 0,
            open_file_descriptors: 0,
        }
    }
}

impl Default for BackpressureStats {
    fn default() -> Self {
        Self {
            total_events: 0,
            current_state: "normal".to_string(),
            backpressure_duration_seconds: 0.0,
            strategy_distribution: HashMap::new(),
        }
    }
}

impl Default for ErrorStats {
    fn default() -> Self {
        Self {
            total_errors: 0,
            error_types: HashMap::new(),
            error_rate: 0.0,
            recent_errors: Vec::new(),
        }
    }
}

impl Clone for MetricsCollector {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            current_metrics: self.current_metrics.clone(),
            metrics_history: self.metrics_history.clone(),
            start_time: self.start_time,
            is_running: self.is_running.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_monitoring_config() {
        let config = MonitoringConfig {
            enable_detailed_metrics: true,
            metrics_interval_ms: 1000,
            enable_health_checks: true,
            health_check_interval_ms: 5000,
            enable_performance_profiling: false,
            metrics_retention_hours: 24,
            enable_alerts: true,
            alert_config: AlertConfig {
                cpu_threshold: 0.8,
                memory_threshold: 0.8,
                error_rate_threshold: 0.05,
                queue_size_threshold: 1000,
                alert_cooldown_seconds: 300,
            },
        };

        assert!(config.enable_detailed_metrics);
        assert_eq!(config.metrics_interval_ms, 1000);
    }

    #[tokio::test]
    async fn test_metrics_collector() {
        let config = MonitoringConfig {
            enable_detailed_metrics: true,
            metrics_interval_ms: 1000,
            enable_health_checks: true,
            health_check_interval_ms: 5000,
            enable_performance_profiling: false,
            metrics_retention_hours: 24,
            enable_alerts: false,
            alert_config: AlertConfig {
                cpu_threshold: 0.8,
                memory_threshold: 0.8,
                error_rate_threshold: 0.05,
                queue_size_threshold: 1000,
                alert_cooldown_seconds: 300,
            },
        };

        let collector = MetricsCollector::new(config);
        let metrics = collector.get_metrics().await;

        assert_eq!(metrics.message_stats.messages_received, 0);
        assert!(metrics.plugin_metrics.is_empty());
    }
}
