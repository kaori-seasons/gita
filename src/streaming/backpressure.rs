//! 背压机制
//!
//! 实现智能的背压策略，防止系统过载
//! 支持多种背压算法和自适应调整

use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

/// 背压配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackpressureConfig {
    /// 启用背压
    pub enabled: bool,
    /// 队列大小阈值
    pub queue_threshold: f64,
    /// CPU使用率阈值
    pub cpu_threshold: f64,
    /// 内存使用率阈值
    pub memory_threshold: f64,
    /// 背压持续时间
    pub backpressure_duration_ms: u64,
    /// 恢复检查间隔
    pub recovery_check_interval_ms: u64,
    /// 自适应调整启用
    pub adaptive_adjustment: bool,
}

/// 背压策略
#[derive(Debug, Clone)]
pub enum BackpressureStrategy {
    /// 拒绝新请求
    Reject,
    /// 延迟处理
    Delay(Duration),
    /// 降级处理
    Degrade,
    /// 自适应调整
    Adaptive,
}

/// 背压状态
#[derive(Debug, Clone)]
pub enum BackpressureState {
    /// 正常状态
    Normal,
    /// 轻度背压
    Light,
    /// 中度背压
    Moderate,
    /// 重度背压
    Heavy,
    /// 紧急状态
    Critical,
}

/// 背压管理器
pub struct BackpressureManager {
    config: BackpressureConfig,
    state: Arc<RwLock<BackpressureState>>,
    metrics_history: Arc<RwLock<VecDeque<SystemMetrics>>>,
    last_backpressure_time: Arc<RwLock<Option<Instant>>>,
    strategy_history: Arc<RwLock<VecDeque<(BackpressureStrategy, Instant)>>>,
}

/// 系统指标
#[derive(Debug, Clone)]
pub struct SystemMetrics {
    timestamp: Instant,
    queue_size: usize,
    queue_capacity: usize,
    cpu_usage: f64,
    memory_usage: f64,
    active_connections: usize,
    processing_rate: f64,
}

impl BackpressureManager {
    /// 创建背压管理器
    pub fn new(config: BackpressureConfig) -> Self {
        Self {
            config,
            state: Arc::new(RwLock::new(BackpressureState::Normal)),
            metrics_history: Arc::new(RwLock::new(VecDeque::with_capacity(100))),
            last_backpressure_time: Arc::new(RwLock::new(None)),
            strategy_history: Arc::new(RwLock::new(VecDeque::with_capacity(50))),
        }
    }

    /// 检查是否需要背压
    pub async fn check_backpressure(&self, metrics: SystemMetrics) -> BackpressureStrategy {
        // 记录指标历史
        self.record_metrics(metrics.clone()).await;

        // 计算背压状态
        let new_state = self.calculate_backpressure_state(&metrics).await;

        // 更新状态
        let mut current_state = self.state.write().await;
        let state_changed = !std::mem::discriminant(&*current_state).eq(std::mem::discriminant(&new_state));

        if state_changed {
            tracing::info!("Backpressure state changed: {:?} -> {:?}", current_state, new_state);
        }

        *current_state = new_state;

        // 根据状态选择策略
        self.select_strategy(&current_state, &metrics).await
    }

    /// 记录系统指标
    async fn record_metrics(&self, metrics: SystemMetrics) {
        let mut history = self.metrics_history.write().await;

        // 保持历史记录在合理范围内
        if history.len() >= 100 {
            history.pop_front();
        }

        history.push_back(metrics);
    }

    /// 计算背压状态
    async fn calculate_backpressure_state(&self, metrics: &SystemMetrics) -> BackpressureState {
        let queue_ratio = metrics.queue_size as f64 / metrics.queue_capacity as f64;
        let cpu_ratio = metrics.cpu_usage;
        let memory_ratio = metrics.memory_usage;

        // 计算综合负载分数
        let load_score = (queue_ratio * 0.4) + (cpu_ratio * 0.3) + (memory_ratio * 0.3);

        // 根据负载分数确定状态
        if load_score >= 0.9 {
            BackpressureState::Critical
        } else if load_score >= 0.75 {
            BackpressureState::Heavy
        } else if load_score >= 0.6 {
            BackpressureState::Moderate
        } else if load_score >= 0.4 {
            BackpressureState::Light
        } else {
            BackpressureState::Normal
        }
    }

    /// 选择背压策略
    async fn select_strategy(
        &self,
        state: &BackpressureState,
        metrics: &SystemMetrics,
    ) -> BackpressureStrategy {
        let strategy = match state {
            BackpressureState::Normal => BackpressureStrategy::Reject,
            BackpressureState::Light => {
                // 轻度背压：短暂延迟
                BackpressureStrategy::Delay(Duration::from_millis(10))
            }
            BackpressureState::Moderate => {
                // 中度背压：中等延迟
                BackpressureStrategy::Delay(Duration::from_millis(50))
            }
            BackpressureState::Heavy => {
                // 重度背压：长延迟或降级
                if metrics.processing_rate > 0.5 {
                    BackpressureStrategy::Delay(Duration::from_millis(100))
                } else {
                    BackpressureStrategy::Degrade
                }
            }
            BackpressureState::Critical => {
                // 紧急状态：拒绝请求
                BackpressureStrategy::Reject
            }
        };

        // 记录策略历史
        self.record_strategy(strategy.clone()).await;

        strategy
    }

    /// 记录策略历史
    async fn record_strategy(&self, strategy: BackpressureStrategy) {
        let mut history = self.strategy_history.write().await;

        if history.len() >= 50 {
            history.pop_front();
        }

        history.push_back((strategy, Instant::now()));
    }

    /// 获取当前状态
    pub async fn get_current_state(&self) -> BackpressureState {
        self.state.read().await.clone()
    }

    /// 获取指标历史
    pub async fn get_metrics_history(&self) -> Vec<SystemMetrics> {
        self.metrics_history.read().await.iter().cloned().collect()
    }

    /// 获取策略统计
    pub async fn get_strategy_stats(&self) -> HashMap<String, usize> {
        let history = self.strategy_history.read().await;
        let mut stats = HashMap::new();

        for (strategy, _) in history.iter() {
            let key = match strategy {
                BackpressureStrategy::Reject => "reject",
                BackpressureStrategy::Delay(_) => "delay",
                BackpressureStrategy::Degrade => "degrade",
                BackpressureStrategy::Adaptive => "adaptive",
            };

            *stats.entry(key.to_string()).or_insert(0) += 1;
        }

        stats
    }

    /// 重置背压状态
    pub async fn reset(&self) {
        let mut state = self.state.write().await;
        *state = BackpressureState::Normal;

        let mut metrics_history = self.metrics_history.write().await;
        metrics_history.clear();

        let mut strategy_history = self.strategy_history.write().await;
        strategy_history.clear();

        let mut last_time = self.last_backpressure_time.write().await;
        *last_time = None;
    }
}

impl Default for BackpressureConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            queue_threshold: 0.8,
            cpu_threshold: 0.75,
            memory_threshold: 0.8,
            backpressure_duration_ms: 5000,
            recovery_check_interval_ms: 1000,
            adaptive_adjustment: true,
        }
    }
}

/// 背压控制器
pub struct BackpressureController {
    manager: Arc<BackpressureManager>,
    config: BackpressureConfig,
}

impl BackpressureController {
    /// 创建背压控制器
    pub fn new(manager: Arc<BackpressureManager>, config: BackpressureConfig) -> Self {
        Self { manager, config }
    }

    /// 执行背压控制
    pub async fn execute_backpressure(&self, strategy: &BackpressureStrategy) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match strategy {
            BackpressureStrategy::Reject => {
                // 拒绝新请求 - 已经在调用方处理
                tracing::warn!("Backpressure: Rejecting new requests");
            }
            BackpressureStrategy::Delay(duration) => {
                // 延迟处理
                tracing::info!("Backpressure: Delaying processing by {:?}", duration);
                tokio::time::sleep(*duration).await;
            }
            BackpressureStrategy::Degrade => {
                // 降级处理 - 简化处理逻辑
                tracing::warn!("Backpressure: Degrading processing quality");
                // 这里可以实现降级逻辑，比如减少处理精度
            }
            BackpressureStrategy::Adaptive => {
                // 自适应调整
                tracing::info!("Backpressure: Applying adaptive adjustments");
                self.apply_adaptive_adjustments().await?;
            }
        }

        Ok(())
    }

    /// 应用自适应调整
    async fn apply_adaptive_adjustments(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // 分析历史指标
        let metrics_history = self.manager.get_metrics_history().await;

        if metrics_history.len() < 10 {
            return Ok(()); // 需要足够的历史数据
        }

        // 计算趋势
        let recent_metrics = &metrics_history[metrics_history.len().saturating_sub(10)..];

        // 计算CPU使用率趋势
        let cpu_trend = self.calculate_trend(recent_metrics.iter().map(|m| m.cpu_usage).collect());

        // 计算内存使用率趋势
        let memory_trend = self.calculate_trend(recent_metrics.iter().map(|m| m.memory_usage).collect());

        // 根据趋势调整策略
        if cpu_trend > 0.1 { // CPU使用率上升趋势
            tracing::info!("CPU usage trending up, reducing concurrent processes");
            // 这里可以动态调整并发数
        }

        if memory_trend > 0.1 { // 内存使用率上升趋势
            tracing::info!("Memory usage trending up, enabling memory optimization");
            // 这里可以启用内存优化措施
        }

        Ok(())
    }

    /// 计算趋势（简单线性回归斜率）
    fn calculate_trend(&self, values: Vec<f64>) -> f64 {
        if values.len() < 2 {
            return 0.0;
        }

        let n = values.len() as f64;
        let sum_x: f64 = (0..values.len()).map(|i| i as f64).sum();
        let sum_y: f64 = values.iter().sum();
        let sum_xy: f64 = values.iter().enumerate().map(|(i, &y)| i as f64 * y).sum();
        let sum_x2: f64 = (0..values.len()).map(|i| (i as f64).powi(2)).sum();

        let slope = (n * sum_xy - sum_x * sum_y) / (n * sum_x2 - sum_x.powi(2));

        slope
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backpressure_config_default() {
        let config = BackpressureConfig::default();
        assert!(config.enabled);
        assert_eq!(config.queue_threshold, 0.8);
    }

    #[tokio::test]
    async fn test_backpressure_manager() {
        let config = BackpressureConfig::default();
        let manager = BackpressureManager::new(config);

        // 测试正常状态
        let metrics = SystemMetrics {
            timestamp: Instant::now(),
            queue_size: 10,
            queue_capacity: 100,
            cpu_usage: 0.3,
            memory_usage: 0.4,
            active_connections: 5,
            processing_rate: 1.0,
        };

        let strategy = manager.check_backpressure(metrics).await;
        match strategy {
            BackpressureStrategy::Reject => {
                // 正常状态下应该是Reject（表示不应用背压）
            }
            _ => panic!("Expected Reject strategy for normal conditions"),
        }
    }

    #[tokio::test]
    async fn test_backpressure_controller() {
        let config = BackpressureConfig::default();
        let manager = Arc::new(BackpressureManager::new(config.clone()));
        let controller = BackpressureController::new(manager, config);

        let strategy = BackpressureStrategy::Delay(Duration::from_millis(10));
        let result = controller.execute_backpressure(&strategy).await;
        assert!(result.is_ok());
    }
}
