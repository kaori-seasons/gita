//! 性能优化和测试
//!
//! 提供性能基准测试、负载测试和性能监控功能

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// 性能配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// 是否启用性能监控
    pub enable_monitoring: bool,
    /// 性能采样间隔（毫秒）
    pub sampling_interval_ms: u64,
    /// 是否启用内存监控
    pub enable_memory_monitoring: bool,
    /// 是否启用CPU监控
    pub enable_cpu_monitoring: bool,
    /// 性能阈值配置
    pub thresholds: PerformanceThresholds,
    /// 是否启用自动性能优化
    pub enable_auto_optimization: bool,
}

/// 性能阈值
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceThresholds {
    /// 请求响应时间阈值（毫秒）
    pub request_response_time_ms: u64,
    /// CPU使用率阈值（百分比）
    pub cpu_usage_threshold: f64,
    /// 内存使用率阈值（百分比）
    pub memory_usage_threshold: f64,
    /// 并发连接阈值
    pub concurrent_connections_threshold: usize,
    /// 队列长度阈值
    pub queue_length_threshold: usize,
}

/// 性能指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// 时间戳
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// CPU使用率（百分比）
    pub cpu_usage_percent: f64,
    /// 内存使用量（字节）
    pub memory_usage_bytes: u64,
    /// 内存使用率（百分比）
    pub memory_usage_percent: f64,
    /// 活跃连接数
    pub active_connections: usize,
    /// 总请求数
    pub total_requests: u64,
    /// 平均响应时间（毫秒）
    pub avg_response_time_ms: f64,
    /// P95响应时间（毫秒）
    pub p95_response_time_ms: f64,
    /// P99响应时间（毫秒）
    pub p99_response_time_ms: f64,
    /// 队列长度
    pub queue_length: usize,
    /// 吞吐量（每秒请求数）
    pub throughput_rps: f64,
    /// 错误率（百分比）
    pub error_rate_percent: f64,
}

/// 性能基准测试结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    /// 测试名称
    pub test_name: String,
    /// 执行时间
    pub duration: Duration,
    /// 操作数
    pub operations: u64,
    /// 每秒操作数
    pub ops_per_second: f64,
    /// 平均延迟
    pub avg_latency: Duration,
    /// P95延迟
    pub p95_latency: Duration,
    /// P99延迟
    pub p99_latency: Duration,
    /// CPU使用率
    pub cpu_usage: f64,
    /// 内存使用量
    pub memory_usage: u64,
    /// 详细信息
    pub details: HashMap<String, String>,
}

/// 负载测试配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadTestConfig {
    /// 测试持续时间（秒）
    pub duration_seconds: u64,
    /// 并发用户数
    pub concurrent_users: usize,
    /// 每用户请求间隔（毫秒）
    pub request_interval_ms: u64,
    /// 渐进式负载增加
    pub ramp_up_seconds: u64,
    /// 目标吞吐量（每秒请求数）
    pub target_throughput: Option<u64>,
}

/// 性能分析器
pub struct PerformanceAnalyzer {
    config: PerformanceConfig,
    metrics_history: Arc<Mutex<Vec<PerformanceMetrics>>>,
    benchmark_results: Arc<Mutex<Vec<BenchmarkResult>>>,
}

impl PerformanceAnalyzer {
    /// 创建新的性能分析器
    pub fn new(config: PerformanceConfig) -> Self {
        Self {
            config,
            metrics_history: Arc::new(Mutex::new(Vec::new())),
            benchmark_results: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// 记录性能指标
    pub async fn record_metrics(&self, metrics: PerformanceMetrics) {
        let mut history = self.metrics_history.lock().await;
        history.push(metrics.clone());

        // 保持历史记录在合理范围内（最近24小时的数据）
        if history.len() > 1000 {
            history.remove(0);
        }

        // 检查性能阈值
        self.check_thresholds(&metrics).await;
    }

    /// 检查性能阈值
    async fn check_thresholds(&self, metrics: &PerformanceMetrics) {
        let mut alerts = Vec::new();

        if metrics.avg_response_time_ms > self.config.thresholds.request_response_time_ms as f64 {
            alerts.push(format!("High response time: {:.2}ms", metrics.avg_response_time_ms));
        }

        if metrics.cpu_usage_percent > self.config.thresholds.cpu_usage_threshold {
            alerts.push(format!("High CPU usage: {:.1}%", metrics.cpu_usage_percent));
        }

        if metrics.memory_usage_percent > self.config.thresholds.memory_usage_threshold {
            alerts.push(format!("High memory usage: {:.1}%", metrics.memory_usage_percent));
        }

        if metrics.active_connections > self.config.thresholds.concurrent_connections_threshold {
            alerts.push(format!("High concurrent connections: {}", metrics.active_connections));
        }

        if metrics.queue_length > self.config.thresholds.queue_length_threshold {
            alerts.push(format!("Long queue: {}", metrics.queue_length));
        }

        if metrics.error_rate_percent > 5.0 {
            alerts.push(format!("High error rate: {:.2}%", metrics.error_rate_percent));
        }

        if !alerts.is_empty() {
            tracing::warn!("Performance alerts: {}", alerts.join(", "));
        }
    }

    /// 获取性能统计
    pub async fn get_performance_stats(&self) -> PerformanceStats {
        let history = self.metrics_history.lock().await;

        if history.is_empty() {
            return PerformanceStats::default();
        }

        let latest = history.last().unwrap();

        // 计算趋势
        let mut response_times: Vec<f64> = history.iter()
            .rev()
            .take(100)
            .map(|m| m.avg_response_time_ms)
            .collect();

        response_times.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let p50_response_time = response_times[response_times.len() / 2];
        let p95_response_time = response_times[(response_times.len() as f64 * 0.95) as usize];
        let p99_response_time = response_times[(response_times.len() as f64 * 0.99) as usize];

        PerformanceStats {
            latest_metrics: latest.clone(),
            p50_response_time_ms: p50_response_time,
            p95_response_time_ms: p95_response_time,
            p99_response_time_ms: p99_response_time,
            trend_cpu_usage: self.calculate_trend(&history.iter().map(|m| m.cpu_usage_percent).collect::<Vec<_>>()),
            trend_memory_usage: self.calculate_trend(&history.iter().map(|m| m.memory_usage_percent).collect::<Vec<_>>()),
        }
    }

    /// 计算趋势
    fn calculate_trend(&self, values: &[f64]) -> f64 {
        if values.len() < 2 {
            return 0.0;
        }

        let len = values.len() as f64;
        let first_half: f64 = values.iter().take(values.len() / 2).sum::<f64>() / (len / 2.0);
        let second_half: f64 = values.iter().rev().take(values.len() / 2).sum::<f64>() / (len / 2.0);

        if first_half == 0.0 {
            return 0.0;
        }

        ((second_half - first_half) / first_half) * 100.0
    }

    /// 运行基准测试
    pub async fn run_benchmark(&self, test_name: &str) -> BenchmarkResult {
        tracing::info!("Starting benchmark: {}", test_name);

        let start_time = Instant::now();

        // 这里应该运行具体的基准测试
        // 暂时返回模拟结果
        let duration = start_time.elapsed();

        let result = BenchmarkResult {
            test_name: test_name.to_string(),
            duration,
            operations: 1000,
            ops_per_second: 1000.0 / duration.as_secs_f64(),
            avg_latency: Duration::from_micros(500),
            p95_latency: Duration::from_micros(1000),
            p99_latency: Duration::from_micros(2000),
            cpu_usage: 45.0,
            memory_usage: 100 * 1024 * 1024, // 100MB
            details: HashMap::new(),
        };

        let mut results = self.benchmark_results.lock().await;
        results.push(result.clone());

        tracing::info!("Benchmark completed: {} - {:.0} ops/sec", test_name, result.ops_per_second);
        result
    }

    /// 获取基准测试历史
    pub async fn get_benchmark_history(&self) -> Vec<BenchmarkResult> {
        self.benchmark_results.lock().await.clone()
    }

    /// 生成性能报告
    pub async fn generate_performance_report(&self) -> String {
        let stats = self.get_performance_stats().await;
        let benchmarks = self.get_benchmark_history().await;

        let mut report = String::new();
        report.push_str("# Performance Report\n\n");

        report.push_str("## Current Metrics\n");
        report.push_str(&format!("- CPU Usage: {:.1}%\n", stats.latest_metrics.cpu_usage_percent));
        report.push_str(&format!("- Memory Usage: {:.1}% ({:.1}MB)\n",
            stats.latest_metrics.memory_usage_percent,
            stats.latest_metrics.memory_usage_bytes as f64 / (1024.0 * 1024.0)));
        report.push_str(&format!("- Active Connections: {}\n", stats.latest_metrics.active_connections));
        report.push_str(&format!("- Throughput: {:.0} RPS\n", stats.latest_metrics.throughput_rps));
        report.push_str(&format!("- P95 Response Time: {:.1}ms\n", stats.p95_response_time_ms));
        report.push_str(&format!("- Error Rate: {:.2}%\n", stats.latest_metrics.error_rate_percent));

        report.push_str("\n## Trends (vs 24h ago)\n");
        report.push_str(&format!("- CPU Usage: {:.1}%\n", stats.trend_cpu_usage));
        report.push_str(&format!("- Memory Usage: {:.1}%\n", stats.trend_memory_usage));

        if !benchmarks.is_empty() {
            report.push_str("\n## Recent Benchmarks\n");
            for benchmark in benchmarks.iter().rev().take(5) {
                report.push_str(&format!("- {}: {:.0} ops/sec\n",
                    benchmark.test_name, benchmark.ops_per_second));
            }
        }

        report
    }
}

/// 性能统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceStats {
    /// 最新指标
    pub latest_metrics: PerformanceMetrics,
    /// P50响应时间（毫秒）
    pub p50_response_time_ms: f64,
    /// P95响应时间（毫秒）
    pub p95_response_time_ms: f64,
    /// P99响应时间（毫秒）
    pub p99_response_time_ms: f64,
    /// CPU使用率趋势（百分比变化）
    pub trend_cpu_usage: f64,
    /// 内存使用率趋势（百分比变化）
    pub trend_memory_usage: f64,
}

impl Default for PerformanceStats {
    fn default() -> Self {
        Self {
            latest_metrics: PerformanceMetrics {
                timestamp: chrono::Utc::now(),
                cpu_usage_percent: 0.0,
                memory_usage_bytes: 0,
                memory_usage_percent: 0.0,
                active_connections: 0,
                total_requests: 0,
                avg_response_time_ms: 0.0,
                p95_response_time_ms: 0.0,
                p99_response_time_ms: 0.0,
                queue_length: 0,
                throughput_rps: 0.0,
                error_rate_percent: 0.0,
            },
            p50_response_time_ms: 0.0,
            p95_response_time_ms: 0.0,
            p99_response_time_ms: 0.0,
            trend_cpu_usage: 0.0,
            trend_memory_usage: 0.0,
        }
    }
}

/// 负载测试器
pub struct LoadTester {
    config: LoadTestConfig,
    http_client: reqwest::Client,
}

impl LoadTester {
    /// 创建新的负载测试器
    pub fn new(config: LoadTestConfig) -> Self {
        Self {
            config,
            http_client: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
        }
    }

    /// 运行负载测试
    pub async fn run_load_test(&self, target_url: &str) -> Result<LoadTestResult, Box<dyn std::error::Error + Send + Sync>> {
        tracing::info!("Starting load test with {} concurrent users for {}s",
            self.config.concurrent_users, self.config.duration_seconds);

        let start_time = Instant::now();
        let mut handles = Vec::new();

        // 启动并发用户
        for user_id in 0..self.config.concurrent_users {
            let client = self.http_client.clone();
            let url = target_url.to_string();
            let config = self.config.clone();

            let handle = tokio::spawn(async move {
                Self::run_user_simulation(client, url, user_id, config).await
            });

            handles.push(handle);

            // 渐进式启动用户
            if config.ramp_up_seconds > 0 {
                let delay = Duration::from_millis(
                    (config.ramp_up_seconds * 1000 / config.concurrent_users as u64)
                );
                tokio::time::sleep(delay).await;
            }
        }

        // 等待所有用户完成
        let mut total_requests = 0u64;
        let mut total_errors = 0u64;
        let mut response_times = Vec::new();

        for handle in handles {
            match handle.await {
                Ok(Ok((requests, errors, times))) => {
                    total_requests += requests;
                    total_errors += errors;
                    response_times.extend(times);
                }
                Ok(Err(e)) => {
                    tracing::error!("User simulation failed: {}", e);
                }
                Err(e) => {
                    tracing::error!("Task join failed: {}", e);
                }
            }
        }

        let duration = start_time.elapsed();
        let throughput = total_requests as f64 / duration.as_secs_f64();
        let error_rate = if total_requests > 0 {
            (total_errors as f64 / total_requests as f64) * 100.0
        } else {
            0.0
        };

        // 计算响应时间统计
        response_times.sort();
        let avg_response_time = if !response_times.is_empty() {
            response_times.iter().sum::<u128>() as f64 / response_times.len() as f64 / 1_000_000.0
        } else {
            0.0
        };

        let p95_response_time = if response_times.len() >= 20 {
            response_times[(response_times.len() as f64 * 0.95) as usize] as f64 / 1_000_000.0
        } else {
            0.0
        };

        let p99_response_time = if response_times.len() >= 100 {
            response_times[(response_times.len() as f64 * 0.99) as usize] as f64 / 1_000_000.0
        } else {
            0.0
        };

        let result = LoadTestResult {
            duration,
            total_requests,
            total_errors,
            throughput_rps: throughput,
            error_rate_percent: error_rate,
            avg_response_time_ms: avg_response_time,
            p95_response_time_ms: p95_response_time,
            p99_response_time_ms: p99_response_time,
            concurrent_users: self.config.concurrent_users,
        };

        tracing::info!("Load test completed: {:.0} RPS, {:.2}% error rate",
            result.throughput_rps, result.error_rate_percent);

        Ok(result)
    }

    /// 运行单个用户模拟
    async fn run_user_simulation(
        client: reqwest::Client,
        url: String,
        user_id: usize,
        config: LoadTestConfig,
    ) -> Result<(u64, u64, Vec<u128>), Box<dyn std::error::Error + Send + Sync>> {
        let mut requests = 0u64;
        let mut errors = 0u64;
        let mut response_times = Vec::new();

        let end_time = Instant::now() + Duration::from_secs(config.duration_seconds);

        while Instant::now() < end_time {
            let request_start = Instant::now();

            match client.get(&url).send().await {
                Ok(_) => {
                    requests += 1;
                    let response_time = request_start.elapsed().as_nanos();
                    response_times.push(response_time);
                }
                Err(_) => {
                    errors += 1;
                }
            }

            // 控制请求频率
            if config.request_interval_ms > 0 {
                tokio::time::sleep(Duration::from_millis(config.request_interval_ms)).await;
            }
        }

        Ok((requests, errors, response_times))
    }
}

/// 负载测试结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadTestResult {
    /// 测试持续时间
    pub duration: Duration,
    /// 总请求数
    pub total_requests: u64,
    /// 总错误数
    pub total_errors: u64,
    /// 吞吐量（每秒请求数）
    pub throughput_rps: f64,
    /// 错误率（百分比）
    pub error_rate_percent: f64,
    /// 平均响应时间（毫秒）
    pub avg_response_time_ms: f64,
    /// P95响应时间（毫秒）
    pub p95_response_time_ms: f64,
    /// P99响应时间（毫秒）
    pub p99_response_time_ms: f64,
    /// 并发用户数
    pub concurrent_users: usize,
}

/// 性能优化建议
pub struct PerformanceOptimizer;

impl PerformanceOptimizer {
    /// 分析性能瓶颈并生成优化建议
    pub async fn analyze_and_optimize(
        analyzer: &PerformanceAnalyzer,
        metrics: &PerformanceMetrics,
    ) -> Vec<OptimizationRecommendation> {
        let mut recommendations = Vec::new();

        // 检查响应时间
        if metrics.avg_response_time_ms > 100.0 {
            recommendations.push(OptimizationRecommendation {
                category: "Response Time".to_string(),
                severity: "High".to_string(),
                description: "High average response time detected".to_string(),
                suggestion: "Consider optimizing database queries, adding caching, or scaling resources".to_string(),
                estimated_impact: "20-50% improvement".to_string(),
            });
        }

        // 检查CPU使用率
        if metrics.cpu_usage_percent > 80.0 {
            recommendations.push(OptimizationRecommendation {
                category: "CPU Usage".to_string(),
                severity: "High".to_string(),
                description: "High CPU usage detected".to_string(),
                suggestion: "Consider optimizing algorithms, reducing computation, or adding more CPU cores".to_string(),
                estimated_impact: "30-60% improvement".to_string(),
            });
        }

        // 检查内存使用率
        if metrics.memory_usage_percent > 85.0 {
            recommendations.push(OptimizationRecommendation {
                category: "Memory Usage".to_string(),
                severity: "Medium".to_string(),
                description: "High memory usage detected".to_string(),
                suggestion: "Consider optimizing data structures, implementing memory pooling, or adding more RAM".to_string(),
                estimated_impact: "15-40% improvement".to_string(),
            });
        }

        // 检查队列长度
        if metrics.queue_length > 100 {
            recommendations.push(OptimizationRecommendation {
                category: "Queue Management".to_string(),
                severity: "Medium".to_string(),
                description: "Long request queue detected".to_string(),
                suggestion: "Consider increasing worker threads, optimizing request processing, or load balancing".to_string(),
                estimated_impact: "25-45% improvement".to_string(),
            });
        }

        recommendations
    }
}

/// 优化建议
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationRecommendation {
    /// 分类
    pub category: String,
    /// 严重程度
    pub severity: String,
    /// 问题描述
    pub description: String,
    /// 建议措施
    pub suggestion: String,
    /// 预期影响
    pub estimated_impact: String,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            enable_monitoring: true,
            sampling_interval_ms: 5000, // 5秒
            enable_memory_monitoring: true,
            enable_cpu_monitoring: true,
            thresholds: PerformanceThresholds {
                request_response_time_ms: 1000,
                cpu_usage_threshold: 80.0,
                memory_usage_threshold: 85.0,
                concurrent_connections_threshold: 1000,
                queue_length_threshold: 100,
            },
            enable_auto_optimization: false,
        }
    }
}

impl Default for LoadTestConfig {
    fn default() -> Self {
        Self {
            duration_seconds: 60,
            concurrent_users: 10,
            request_interval_ms: 100,
            ramp_up_seconds: 10,
            target_throughput: None,
        }
    }
}

/// 性能监控服务
pub struct PerformanceMonitoringService {
    analyzer: Arc<PerformanceAnalyzer>,
    _monitoring_task: Option<tokio::task::JoinHandle<()>>,
}

impl PerformanceMonitoringService {
    /// 创建新的性能监控服务
    pub fn new(analyzer: Arc<PerformanceAnalyzer>) -> Self {
        Self {
            analyzer,
            _monitoring_task: None,
        }
    }

    /// 启动性能监控
    pub async fn start_monitoring(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let analyzer = Arc::clone(&self.analyzer);
        let config = analyzer.config.clone();

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(config.sampling_interval_ms));

            loop {
                interval.tick().await;

                // 收集系统指标
                let metrics = Self::collect_system_metrics().await;

                // 记录指标
                analyzer.record_metrics(metrics).await;
            }
        });

        self._monitoring_task = Some(handle);
        tracing::info!("Performance monitoring started");
        Ok(())
    }

    /// 收集系统指标
    async fn collect_system_metrics() -> PerformanceMetrics {
        // 这里应该收集实际的系统指标
        // 暂时返回模拟数据
        PerformanceMetrics {
            timestamp: chrono::Utc::now(),
            cpu_usage_percent: 25.0 + (rand::random::<f64>() * 20.0), // 25-45%
            memory_usage_bytes: 50 * 1024 * 1024, // 50MB
            memory_usage_percent: 30.0 + (rand::random::<f64>() * 20.0), // 30-50%
            active_connections: 15 + (rand::random::<u32>() % 20) as usize, // 15-35
            total_requests: 1000 + (rand::random::<u64>() % 500), // 1000-1500
            avg_response_time_ms: 50.0 + (rand::random::<f64>() * 100.0), // 50-150ms
            p95_response_time_ms: 100.0 + (rand::random::<f64>() * 200.0), // 100-300ms
            p99_response_time_ms: 200.0 + (rand::random::<f64>() * 400.0), // 200-600ms
            queue_length: (rand::random::<usize>() % 50), // 0-50
            throughput_rps: 20.0 + (rand::random::<f64>() * 30.0), // 20-50 RPS
            error_rate_percent: (rand::random::<f64>() * 5.0), // 0-5%
        }
    }
}
