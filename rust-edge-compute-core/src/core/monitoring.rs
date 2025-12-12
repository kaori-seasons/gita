//! 监控模块
//!
//! 提供系统性能监控、指标收集和健康检查功能

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use std::time::{Instant, Duration};

/// 监控指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metrics {
    /// CPU使用率（百分比）
    pub cpu_usage: f64,
    /// 内存使用量（字节）
    pub memory_usage: u64,
    /// 活跃任务数
    pub active_tasks: usize,
    /// 队列长度
    pub queue_length: usize,
    /// 吞吐量（每秒任务数）
    pub throughput: f64,
    /// 平均响应时间（毫秒）
    pub avg_response_time: f64,
    /// 错误率（百分比）
    pub error_rate: f64,
}

impl Default for Metrics {
    fn default() -> Self {
        Self {
            cpu_usage: 0.0,
            memory_usage: 0,
            active_tasks: 0,
            queue_length: 0,
            throughput: 0.0,
            avg_response_time: 0.0,
            error_rate: 0.0,
        }
    }
}

/// 监控统计
#[derive(Debug, Clone)]
pub struct MonitoringStats {
    /// 指标历史记录
    metrics_history: Arc<RwLock<Vec<(Instant, Metrics)>>>,
    /// 最大历史记录数
    max_history: usize,
}

impl MonitoringStats {
    /// 创建新的监控统计
    pub fn new(max_history: usize) -> Self {
        Self {
            metrics_history: Arc::new(RwLock::new(Vec::new())),
            max_history,
        }
    }

    /// 记录指标
    pub async fn record_metrics(&self, metrics: Metrics) {
        let mut history = self.metrics_history.write().await;
        history.push((Instant::now(), metrics));
        
        // 保持历史记录在合理范围内
        if history.len() > self.max_history {
            history.drain(0..(history.len() - self.max_history));
        }
    }

    /// 获取最新指标
    pub async fn get_latest_metrics(&self) -> Option<Metrics> {
        let history = self.metrics_history.read().await;
        history.last().map(|(_, metrics)| metrics.clone())
    }

    /// 获取指标历史
    pub async fn get_metrics_history(&self, duration: Duration) -> Vec<Metrics> {
        let history = self.metrics_history.read().await;
        let now = Instant::now();
        history.iter()
            .filter(|(timestamp, _)| now.duration_since(*timestamp) <= duration)
            .map(|(_, metrics)| metrics.clone())
            .collect()
    }
}

/// 健康检查结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    /// 是否健康
    pub healthy: bool,
    /// 消息
    pub message: String,
    /// 详细信息
    pub details: HashMap<String, String>,
}

/// 监控管理器
pub struct MonitoringManager {
    stats: Arc<MonitoringStats>,
}

impl MonitoringManager {
    /// 创建新的监控管理器
    pub fn new() -> Self {
        Self {
            stats: Arc::new(MonitoringStats::new(1000)), // 保留最近1000条记录
        }
    }

    /// 获取监控统计
    pub fn get_stats(&self) -> Arc<MonitoringStats> {
        Arc::clone(&self.stats)
    }

    /// 执行健康检查
    pub async fn health_check(&self) -> HealthCheckResult {
        // 这里应该实现实际的健康检查逻辑
        // 暂时返回模拟结果
        HealthCheckResult {
            healthy: true,
            message: "System is healthy".to_string(),
            details: {
                let mut map = HashMap::new();
                map.insert("status".to_string(), "operational".to_string());
                map.insert("version".to_string(), env!("CARGO_PKG_VERSION").to_string());
                map
            },
        }
    }
}

impl Default for MonitoringManager {
    fn default() -> Self {
        Self::new()
    }
}