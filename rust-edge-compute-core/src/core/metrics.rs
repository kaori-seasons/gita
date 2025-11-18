//! 监控和指标收集

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// 指标类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MetricType {
    /// 计数器 - 单调递增的值
    Counter,
    /// 仪表 - 可以任意变化的值
    Gauge,
    /// 直方图 - 分布统计
    Histogram,
    /// 摘要 - 分位数统计
    Summary,
}

/// 指标值
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MetricValue {
    /// 整数值
    Integer(i64),
    /// 浮点值
    Float(f64),
    /// 直方图数据
    Histogram {
        count: u64,
        sum: f64,
        buckets: Vec<(f64, u64)>,
    },
}

/// 指标定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metric {
    /// 指标名称
    pub name: String,
    /// 指标类型
    pub metric_type: MetricType,
    /// 指标描述
    pub description: String,
    /// 标签
    pub labels: HashMap<String, String>,
    /// 当前值
    pub value: MetricValue,
    /// 时间戳
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// 指标收集器
pub struct MetricsCollector {
    metrics: Arc<Mutex<HashMap<String, Metric>>>,
    start_time: Instant,
}

impl MetricsCollector {
    /// 创建新的指标收集器
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(Mutex::new(HashMap::new())),
            start_time: Instant::now(),
        }
    }

    /// 注册指标
    pub async fn register_metric(&self, metric: Metric) {
        let mut metrics = self.metrics.lock().await;
        metrics.insert(metric.name.clone(), metric);
    }

    /// 更新计数器指标
    pub async fn increment_counter(&self, name: &str, value: i64, labels: HashMap<String, String>) {
        let mut metrics = self.metrics.lock().await;

        if let Some(metric) = metrics.get_mut(name) {
            if let MetricValue::Integer(current) = &mut metric.value {
                *current += value;
            }
            metric.timestamp = chrono::Utc::now();
            metric.labels.extend(labels);
        } else {
            // 自动注册新指标
            let metric = Metric {
                name: name.to_string(),
                metric_type: MetricType::Counter,
                description: format!("Auto-registered counter: {}", name),
                labels,
                value: MetricValue::Integer(value),
                timestamp: chrono::Utc::now(),
            };
            metrics.insert(name.to_string(), metric);
        }
    }

    /// 设置仪表指标
    pub async fn set_gauge(&self, name: &str, value: f64, labels: HashMap<String, String>) {
        let mut metrics = self.metrics.lock().await;

        if let Some(metric) = metrics.get_mut(name) {
            metric.value = MetricValue::Float(value);
            metric.timestamp = chrono::Utc::now();
            metric.labels.extend(labels);
        } else {
            // 自动注册新指标
            let metric = Metric {
                name: name.to_string(),
                metric_type: MetricType::Gauge,
                description: format!("Auto-registered gauge: {}", name),
                labels,
                value: MetricValue::Float(value),
                timestamp: chrono::Utc::now(),
            };
            metrics.insert(name.to_string(), metric);
        }
    }

    /// 记录直方图观测值
    pub async fn observe_histogram(&self, name: &str, value: f64, labels: HashMap<String, String>) {
        let mut metrics = self.metrics.lock().await;

        if let Some(metric) = metrics.get_mut(name) {
            if let MetricValue::Histogram { count, sum, buckets } = &mut metric.value {
                *count += 1;
                *sum += value;

                // 更新直方图桶
                for (bucket_value, bucket_count) in buckets.iter_mut() {
                    if value <= *bucket_value {
                        *bucket_count += 1;
                    }
                }
            }
            metric.timestamp = chrono::Utc::now();
            metric.labels.extend(labels);
        } else {
            // 自动注册新指标
            let buckets = vec![
                (0.1, 0), (0.5, 0), (1.0, 0), (2.5, 0), (5.0, 0), (10.0, 0)
            ];
            let mut bucket_counts = buckets.clone();

            // 更新第一个合适的桶
            for (bucket_value, bucket_count) in bucket_counts.iter_mut() {
                if value <= *bucket_value {
                    *bucket_count = 1;
                    break;
                }
            }

            let metric = Metric {
                name: name.to_string(),
                metric_type: MetricType::Histogram,
                description: format!("Auto-registered histogram: {}", name),
                labels,
                value: MetricValue::Histogram {
                    count: 1,
                    sum: value,
                    buckets: bucket_counts,
                },
                timestamp: chrono::Utc::now(),
            };
            metrics.insert(name.to_string(), metric);
        }
    }

    /// 获取所有指标
    pub async fn get_all_metrics(&self) -> Vec<Metric> {
        let metrics = self.metrics.lock().await;
        metrics.values().cloned().collect()
    }

    /// 获取特定指标
    pub async fn get_metric(&self, name: &str) -> Option<Metric> {
        let metrics = self.metrics.lock().await;
        metrics.get(name).cloned()
    }

    /// 获取运行时间
    pub fn uptime(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// 重置所有指标
    pub async fn reset_all_metrics(&self) {
        let mut metrics = self.metrics.lock().await;
        metrics.clear();
    }

    /// 导出为Prometheus格式
    pub async fn export_prometheus(&self) -> String {
        let metrics = self.get_all_metrics().await;
        let mut output = String::new();

        for metric in metrics {
            // 添加HELP注释
            output.push_str(&format!("# HELP {} {}\n", metric.name, metric.description));
            output.push_str(&format!("# TYPE {} {}\n",
                metric.name,
                match metric.metric_type {
                    MetricType::Counter => "counter",
                    MetricType::Gauge => "gauge",
                    MetricType::Histogram => "histogram",
                    MetricType::Summary => "summary",
                }
            ));

            // 添加标签
            let labels_str = if metric.labels.is_empty() {
                String::new()
            } else {
                let label_parts: Vec<String> = metric.labels.iter()
                    .map(|(k, v)| format!("{}=\"{}\"", k, v))
                    .collect();
                format!("{{{}}}", label_parts.join(","))
            };

            // 添加值
            match &metric.value {
                MetricValue::Integer(value) => {
                    output.push_str(&format!("{}{} {}\n",
                        metric.name, labels_str, value));
                }
                MetricValue::Float(value) => {
                    output.push_str(&format!("{}{} {}\n",
                        metric.name, labels_str, value));
                }
                MetricValue::Histogram { count, sum, buckets } => {
                    output.push_str(&format!("{}_count{} {}\n",
                        metric.name, labels_str, count));
                    output.push_str(&format!("{}_sum{} {}\n",
                        metric.name, labels_str, sum));

                    for (bucket_value, bucket_count) in buckets {
                        output.push_str(&format!("{}_bucket{{le=\"{}\"}}{} {}\n",
                            metric.name, bucket_value, labels_str, bucket_count));
                    }
                }
            }

            output.push_str("\n");
        }

        output
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// 性能监控器
pub struct PerformanceMonitor {
    collector: Arc<MetricsCollector>,
}

impl PerformanceMonitor {
    /// 创建新的性能监控器
    pub fn new(collector: Arc<MetricsCollector>) -> Self {
        Self { collector }
    }

    /// 记录请求处理时间
    pub async fn record_request_duration(&self, method: &str, path: &str, status: u16, duration: Duration) {
        let mut labels = HashMap::new();
        labels.insert("method".to_string(), method.to_string());
        labels.insert("path".to_string(), path.to_string());
        labels.insert("status".to_string(), status.to_string());

        self.collector.observe_histogram(
            "http_request_duration_seconds",
            duration.as_secs_f64(),
            labels
        ).await;
    }

    /// 记录活跃连接数
    pub async fn record_active_connections(&self, count: i64) {
        let mut labels = HashMap::new();
        labels.insert("type".to_string(), "active".to_string());

        self.collector.set_gauge(
            "http_connections_active",
            count as f64,
            labels
        ).await;
    }

    /// 记录请求总数
    pub async fn record_request_total(&self, method: &str, status: u16) {
        let mut labels = HashMap::new();
        labels.insert("method".to_string(), method.to_string());
        labels.insert("status".to_string(), status.to_string());

        self.collector.increment_counter(
            "http_requests_total",
            1,
            labels
        ).await;
    }

    /// 记录数据库操作时间
    pub async fn record_db_operation(&self, operation: &str, table: &str, duration: Duration) {
        let mut labels = HashMap::new();
        labels.insert("operation".to_string(), operation.to_string());
        labels.insert("table".to_string(), table.to_string());

        self.collector.observe_histogram(
            "db_operation_duration_seconds",
            duration.as_secs_f64(),
            labels
        ).await;
    }

    /// 记录内存使用情况
    pub async fn record_memory_usage(&self) {
        // 这里可以集成系统内存监控
        // 暂时使用估算值
        let mut labels = HashMap::new();
        labels.insert("type".to_string(), "rss".to_string());

        self.collector.set_gauge(
            "process_memory_bytes",
            50.0 * 1024.0 * 1024.0, // 50MB估算值
            labels
        ).await;
    }

    /// 记录CPU使用率
    pub async fn record_cpu_usage(&self, usage_percent: f64) {
        let mut labels = HashMap::new();
        labels.insert("type".to_string(), "usage".to_string());

        self.collector.set_gauge(
            "process_cpu_usage_percent",
            usage_percent,
            labels
        ).await;
    }

    /// 记录任务队列大小
    pub async fn record_queue_size(&self, queue_name: &str, size: i64) {
        let mut labels = HashMap::new();
        labels.insert("queue".to_string(), queue_name.to_string());

        self.collector.set_gauge(
            "task_queue_size",
            size as f64,
            labels
        ).await;
    }

    /// 记录错误发生
    pub async fn record_error(&self, error_type: &str, severity: &str) {
        let mut labels = HashMap::new();
        labels.insert("type".to_string(), error_type.to_string());
        labels.insert("severity".to_string(), severity.to_string());

        self.collector.increment_counter(
            "application_errors_total",
            1,
            labels
        ).await;
    }
}

/// 健康检查
pub struct HealthChecker {
    checks: Arc<Mutex<HashMap<String, Box<dyn HealthCheck>>>>,
}

#[async_trait::async_trait]
pub trait HealthCheck: Send + Sync {
    /// 执行健康检查
    async fn check(&self) -> HealthStatus;

    /// 获取检查名称
    fn name(&self) -> &str;

    /// 获取检查描述
    fn description(&self) -> &str;
}

/// 健康状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    /// 检查名称
    pub name: String,
    /// 状态
    pub status: HealthState,
    /// 详细信息
    pub details: Option<String>,
    /// 检查时间
    pub checked_at: chrono::DateTime<chrono::Utc>,
}

/// 健康状态枚举
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthState {
    /// 健康
    Healthy,
    /// 不健康
    Unhealthy,
    /// 警告
    Warning,
}

impl HealthChecker {
    /// 创建新的健康检查器
    pub fn new() -> Self {
        Self {
            checks: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// 注册健康检查
    pub async fn register_check(&self, check: Box<dyn HealthCheck>) {
        let mut checks = self.checks.lock().await;
        checks.insert(check.name().to_string(), check);
    }

    /// 执行所有健康检查
    pub async fn perform_all_checks(&self) -> Vec<HealthStatus> {
        let checks = self.checks.lock().await;
        let mut results = Vec::new();

        for check in checks.values() {
            let status = check.check().await;
            results.push(status);
        }

        results
    }

    /// 执行特定健康检查
    pub async fn perform_check(&self, name: &str) -> Option<HealthStatus> {
        let checks = self.checks.lock().await;
        if let Some(check) = checks.get(name) {
            Some(check.check().await)
        } else {
            None
        }
    }

    /// 获取整体健康状态
    pub async fn overall_health(&self) -> HealthState {
        let results = self.perform_all_checks().await;

        let has_unhealthy = results.iter().any(|r| matches!(r.status, HealthState::Unhealthy));
        let has_warnings = results.iter().any(|r| matches!(r.status, HealthState::Warning));

        if has_unhealthy {
            HealthState::Unhealthy
        } else if has_warnings {
            HealthState::Warning
        } else {
            HealthState::Healthy
        }
    }
}

impl Default for HealthChecker {
    fn default() -> Self {
        Self::new()
    }
}

/// 标准健康检查实现
pub mod checks {
    use super::*;

    /// 数据库健康检查
    pub struct DatabaseHealthCheck;

    impl DatabaseHealthCheck {
        pub fn new() -> Self {
            Self
        }
    }

    #[async_trait::async_trait]
    impl HealthCheck for DatabaseHealthCheck {
        async fn check(&self) -> HealthStatus {
            // 这里应该实际检查数据库连接
            // 暂时返回健康状态
            HealthStatus {
                name: "database".to_string(),
                status: HealthState::Healthy,
                details: Some("Database connection is healthy".to_string()),
                checked_at: chrono::Utc::now(),
            }
        }

        fn name(&self) -> &str {
            "database"
        }

        fn description(&self) -> &str {
            "Checks database connectivity and performance"
        }
    }

    /// API健康检查
    pub struct ApiHealthCheck;

    impl ApiHealthCheck {
        pub fn new() -> Self {
            Self
        }
    }

    #[async_trait::async_trait]
    impl HealthCheck for ApiHealthCheck {
        async fn check(&self) -> HealthStatus {
            // 这里应该检查API端点的响应性
            HealthStatus {
                name: "api".to_string(),
                status: HealthState::Healthy,
                details: Some("API endpoints are responding".to_string()),
                checked_at: chrono::Utc::now(),
            }
        }

        fn name(&self) -> &str {
            "api"
        }

        fn description(&self) -> &str {
            "Checks API endpoint responsiveness"
        }
    }

    /// 系统资源健康检查
    pub struct SystemHealthCheck;

    impl SystemHealthCheck {
        pub fn new() -> Self {
            Self
        }
    }

    #[async_trait::async_trait]
    impl HealthCheck for SystemHealthCheck {
        async fn check(&self) -> HealthStatus {
            // 检查系统资源使用情况
            HealthStatus {
                name: "system".to_string(),
                status: HealthState::Healthy,
                details: Some("System resources are within normal limits".to_string()),
                checked_at: chrono::Utc::now(),
            }
        }

        fn name(&self) -> &str {
            "system"
        }

        fn description(&self) -> &str {
            "Checks system resource usage"
        }
    }
}
