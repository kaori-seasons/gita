//! 智能调度器模块
//!
//! 基于机器学习和历史数据的智能任务调度

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, Mutex};
use serde::{Deserialize, Serialize};
use crate::core::types::{WorkerInfo, LoadBalancingStrategy};

/// 智能调度器
pub struct IntelligentScheduler {
    /// 历史调度数据
    historical_data: Arc<RwLock<SchedulingHistory>>,
    /// 性能预测模型
    performance_predictor: Arc<Mutex<PerformancePredictor>>,
    /// 学习配置
    learning_config: LearningConfig,
}

/// 学习配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningConfig {
    /// 学习率
    pub learning_rate: f64,
    /// 历史数据窗口大小
    pub history_window_size: usize,
    /// 最小训练样本数
    pub min_training_samples: usize,
    /// 预测时间窗口 (秒)
    pub prediction_window_seconds: u64,
    /// 模型更新间隔 (秒)
    pub model_update_interval_seconds: u64,
}

impl Default for LearningConfig {
    fn default() -> Self {
        Self {
            learning_rate: 0.01,
            history_window_size: 1000,
            min_training_samples: 100,
            prediction_window_seconds: 300, // 5分钟
            model_update_interval_seconds: 3600, // 1小时
        }
    }
}

/// 调度历史数据
#[derive(Debug, Clone)]
pub struct SchedulingHistory {
    /// 调度决策历史
    decisions: VecDeque<SchedulingDecision>,
    /// 性能指标历史
    performance_metrics: VecDeque<PerformanceMetrics>,
    /// 工作线程性能历史
    worker_performance_history: HashMap<usize, VecDeque<WorkerPerformanceRecord>>,
}

/// 调度决策记录
#[derive(Debug, Clone)]
pub struct SchedulingDecision {
    /// 决策时间
    pub timestamp: Instant,
    /// 选择的调度策略
    pub strategy: LoadBalancingStrategy,
    /// 选择的工作线程ID
    pub selected_worker: usize,
    /// 决策前的系统状态
    pub system_state: SystemStateSnapshot,
    /// 决策结果
    pub result: SchedulingResult,
}

/// 系统状态快照
#[derive(Debug, Clone)]
pub struct SystemStateSnapshot {
    /// 系统负载水平
    pub system_load: f64,
    /// 平均响应时间
    pub avg_response_time: f64,
    /// 工作线程状态
    pub worker_states: HashMap<usize, WorkerState>,
    /// 队列长度
    pub queue_length: usize,
}

/// 工作线程状态
#[derive(Debug, Clone)]
pub struct WorkerState {
    /// CPU使用率
    pub cpu_usage: f64,
    /// 内存使用率
    pub memory_usage: f64,
    /// 当前连接数
    pub current_connections: usize,
    /// 负载评分
    pub load_score: f64,
    /// 成功率
    pub success_rate: f64,
}

/// 调度结果
#[derive(Debug, Clone)]
pub enum SchedulingResult {
    /// 成功
    Success {
        /// 实际响应时间
        actual_response_time: f64,
        /// 任务是否成功完成
        task_success: bool,
    },
    /// 失败
    Failure {
        /// 失败原因
        reason: String,
    },
}

/// 性能指标
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    /// 时间戳
    pub timestamp: Instant,
    /// 系统吞吐量 (tasks/second)
    pub throughput: f64,
    /// 平均响应时间
    pub avg_response_time: f64,
    /// 成功率
    pub success_rate: f64,
    /// 系统负载
    pub system_load: f64,
}

/// 工作线程性能记录
#[derive(Debug, Clone)]
pub struct WorkerPerformanceRecord {
    /// 时间戳
    pub timestamp: Instant,
    /// CPU使用率
    pub cpu_usage: f64,
    /// 内存使用率
    pub memory_usage: f64,
    /// 处理的任务数
    pub tasks_processed: u64,
    /// 成功率
    pub success_rate: f64,
    /// 平均响应时间
    pub avg_response_time: f64,
}

/// 性能预测模型
#[derive(Debug, Clone)]
pub struct PerformancePredictor {
    /// 线性回归权重
    weights: HashMap<String, f64>,
    /// 偏置
    bias: f64,
    /// 最后更新时间
    last_updated: Instant,
    /// 训练样本数
    training_samples: usize,
}

impl IntelligentScheduler {
    /// 创建新的智能调度器
    pub fn new(learning_config: LearningConfig) -> Self {
        Self {
            historical_data: Arc::new(RwLock::new(SchedulingHistory {
                decisions: VecDeque::with_capacity(learning_config.history_window_size),
                performance_metrics: VecDeque::with_capacity(learning_config.history_window_size),
                worker_performance_history: HashMap::new(),
            })),
            performance_predictor: Arc::new(Mutex::new(PerformancePredictor {
                weights: HashMap::new(),
                bias: 0.0,
                last_updated: Instant::now(),
                training_samples: 0,
            })),
            learning_config,
        }
    }

    /// 记录调度决策
    pub async fn record_decision(&self, decision: SchedulingDecision) {
        let mut history = self.historical_data.write().await;

        // 添加决策记录
        history.decisions.push_back(decision.clone());

        // 维护历史数据窗口大小
        while history.decisions.len() > self.learning_config.history_window_size {
            history.decisions.pop_front();
        }

        // 更新工作线程性能历史
        let worker_history = history.worker_performance_history
            .entry(decision.selected_worker)
            .or_insert_with(|| VecDeque::with_capacity(100));

        let worker_state = &decision.system_state.worker_states[&decision.selected_worker];
        worker_history.push_back(WorkerPerformanceRecord {
            timestamp: decision.timestamp,
            cpu_usage: worker_state.cpu_usage,
            memory_usage: worker_state.memory_usage,
            tasks_processed: 0, // 这里需要实际的任务处理计数
            success_rate: worker_state.success_rate,
            avg_response_time: decision.system_state.avg_response_time,
        });

        // 维护工作线程历史大小
        while worker_history.len() > 100 {
            worker_history.pop_front();
        }
    }

    /// 记录性能指标
    pub async fn record_performance_metrics(&self, metrics: PerformanceMetrics) {
        let mut history = self.historical_data.write().await;
        history.performance_metrics.push_back(metrics);

        // 维护性能指标历史大小
        while history.performance_metrics.len() > self.learning_config.history_window_size {
            history.performance_metrics.pop_front();
        }
    }

    /// 预测最优工作线程
    pub async fn predict_optimal_worker(
        &self,
        workers: &[&WorkerInfo],
        system_load: f64,
        queue_length: usize,
    ) -> Option<usize> {
        if workers.is_empty() {
            return None;
        }

        // 如果没有足够的训练数据，使用简单的启发式方法
        if !self.has_sufficient_training_data().await {
            return self.simple_heuristic_selection(workers);
        }

        // 使用机器学习模型进行预测
        let mut best_worker = None;
        let mut best_score = f64::NEG_INFINITY;

        for worker in workers {
            let prediction_score = self.predict_worker_performance(
                worker,
                system_load,
                queue_length,
            ).await;

            if prediction_score > best_score {
                best_score = prediction_score;
                best_worker = Some(worker.id);
            }
        }

        best_worker
    }

    /// 预测工作线程性能
    async fn predict_worker_performance(
        &self,
        worker: &WorkerInfo,
        system_load: f64,
        queue_length: usize,
    ) -> f64 {
        let predictor = self.performance_predictor.lock().await;

        // 简化的线性预测模型
        // 实际实现中应该使用更复杂的机器学习算法
        let features = self.extract_features(worker, system_load, queue_length);
        let mut prediction = predictor.bias;

        for (feature_name, feature_value) in features {
            if let Some(weight) = predictor.weights.get(&feature_name) {
                prediction += weight * feature_value;
            }
        }

        prediction
    }

    /// 提取特征向量
    fn extract_features(
        &self,
        worker: &WorkerInfo,
        system_load: f64,
        queue_length: usize,
    ) -> HashMap<String, f64> {
        let mut features = HashMap::new();

        features.insert("cpu_usage".to_string(), worker.cpu_usage);
        features.insert("memory_usage".to_string(), worker.memory_usage);
        features.insert("current_connections".to_string(), worker.current_connections as f64);
        features.insert("load_score".to_string(), worker.load_score());
        features.insert("capacity_score".to_string(), worker.capacity_score());
        features.insert("success_rate".to_string(), worker.success_rate());
        features.insert("avg_response_time".to_string(), worker.avg_response_time);
        features.insert("system_load".to_string(), system_load);
        features.insert("queue_length".to_string(), queue_length as f64);
        features.insert("weight".to_string(), worker.weight as f64);

        features
    }

    /// 简单的启发式选择（当没有足够训练数据时使用）
    fn simple_heuristic_selection(&self, workers: &[&WorkerInfo]) -> Option<usize> {
        workers.iter()
            .min_by(|a, b| {
                let score_a = self.calculate_heuristic_score(a);
                let score_b = self.calculate_heuristic_score(b);
                score_a.partial_cmp(&score_b).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|w| w.id)
    }

    /// 计算启发式评分
    fn calculate_heuristic_score(&self, worker: &WorkerInfo) -> f64 {
        // 基于负载、成功率和响应时间的综合评分
        let load_penalty = worker.load_score();
        let success_bonus = worker.success_rate();
        let response_penalty = (worker.avg_response_time / 1000.0).min(1.0); // 归一化到0-1

        // 公式：成功率权重40% + 负载惩罚权重40% + 响应时间惩罚权重20%
        success_bonus * 0.4 - load_penalty * 0.4 - response_penalty * 0.2
    }

    /// 检查是否有足够的训练数据
    async fn has_sufficient_training_data(&self) -> bool {
        let history = self.historical_data.read().await;
        let predictor = self.performance_predictor.lock().await;

        history.decisions.len() >= self.learning_config.min_training_samples
            && predictor.training_samples >= self.learning_config.min_training_samples
    }

    /// 更新预测模型
    pub async fn update_model(&self) {
        let history = self.historical_data.read().await;
        if history.decisions.len() < self.learning_config.min_training_samples {
            return; // 训练数据不足
        }

        let mut predictor = self.performance_predictor.lock().await;

        // 简化的在线学习算法（梯度下降）
        for decision in &history.decisions {
            if let SchedulingResult::Success { actual_response_time, task_success } = &decision.result {
                let target = if *task_success { 1.0 / (1.0 + actual_response_time) } else { 0.0 };
                let features = self.extract_features_from_decision(decision);

                let mut prediction = predictor.bias;
                for (feature_name, feature_value) in &features {
                    if let Some(weight) = predictor.weights.get(feature_name) {
                        prediction += weight * feature_value;
                    }
                }

                let error = target - prediction;

                // 更新偏置
                predictor.bias += self.learning_config.learning_rate * error;

                // 更新权重
                for (feature_name, feature_value) in &features {
                    let weight = predictor.weights.entry(feature_name.clone())
                        .or_insert(0.0);
                    *weight += self.learning_config.learning_rate * error * feature_value;
                }
            }
        }

        predictor.training_samples = history.decisions.len();
        predictor.last_updated = Instant::now();

        tracing::info!("Updated performance prediction model with {} samples", predictor.training_samples);
    }

    /// 从调度决策中提取特征
    fn extract_features_from_decision(&self, decision: &SchedulingDecision) -> HashMap<String, f64> {
        let mut features = HashMap::new();

        features.insert("system_load".to_string(), decision.system_state.system_load);
        features.insert("avg_response_time".to_string(), decision.system_state.avg_response_time);
        features.insert("queue_length".to_string(), decision.system_state.queue_length as f64);

        if let Some(worker_state) = decision.system_state.worker_states.get(&decision.selected_worker) {
            features.insert("worker_cpu_usage".to_string(), worker_state.cpu_usage);
            features.insert("worker_memory_usage".to_string(), worker_state.memory_usage);
            features.insert("worker_current_connections".to_string(), worker_state.current_connections as f64);
            features.insert("worker_load_score".to_string(), worker_state.load_score);
            features.insert("worker_success_rate".to_string(), worker_state.success_rate);
        }

        features
    }

    /// 获取智能调度统计信息
    pub async fn get_intelligent_stats(&self) -> IntelligentStats {
        let history = self.historical_data.read().await;
        let predictor = self.performance_predictor.lock().await;

        let total_decisions = history.decisions.len();
        let successful_decisions = history.decisions.iter()
            .filter(|d| matches!(d.result, SchedulingResult::Success { .. }))
            .count();

        let success_rate = if total_decisions > 0 {
            successful_decisions as f64 / total_decisions as f64
        } else {
            0.0
        };

        let avg_response_time = if !history.decisions.is_empty() {
            history.decisions.iter()
                .filter_map(|d| {
                    if let SchedulingResult::Success { actual_response_time, .. } = &d.result {
                        Some(*actual_response_time)
                    } else {
                        None
                    }
                })
                .sum::<f64>() / history.decisions.len() as f64
        } else {
            0.0
        };

        IntelligentStats {
            total_decisions,
            successful_decisions,
            success_rate,
            avg_response_time,
            model_training_samples: predictor.training_samples,
            model_last_updated: predictor.last_updated,
            learning_config: self.learning_config.clone(),
        }
    }

    /// 分析调度模式
    pub async fn analyze_scheduling_patterns(&self) -> SchedulingPatternAnalysis {
        let history = self.historical_data.read().await;

        let mut strategy_usage = HashMap::new();
        let mut hourly_patterns = HashMap::new();
        let mut load_correlations = Vec::new();

        for decision in &history.decisions {
            // 统计策略使用情况
            *strategy_usage.entry(decision.strategy.clone()).or_insert(0) += 1;

            // 分析时间模式（这里简化处理，实际应该考虑小时）
            let hour = decision.timestamp.elapsed().as_secs() / 3600;
            *hourly_patterns.entry(hour).or_insert(0) += 1;

            // 收集负载相关性数据
            load_correlations.push((
                decision.system_state.system_load,
                decision.system_state.avg_response_time,
            ));
        }

        SchedulingPatternAnalysis {
            strategy_usage,
            hourly_patterns,
            load_correlations,
        }
    }
}

/// 智能调度统计信息
#[derive(Debug, Clone, Serialize)]
pub struct IntelligentStats {
    /// 总调度决策数
    pub total_decisions: usize,
    /// 成功调度决策数
    pub successful_decisions: usize,
    /// 调度成功率
    pub success_rate: f64,
    /// 平均响应时间
    pub avg_response_time: f64,
    /// 模型训练样本数
    pub model_training_samples: usize,
    /// 模型最后更新时间
    pub model_last_updated: Instant,
    /// 学习配置
    pub learning_config: LearningConfig,
}

/// 调度模式分析
#[derive(Debug, Clone)]
pub struct SchedulingPatternAnalysis {
    /// 策略使用统计
    pub strategy_usage: HashMap<LoadBalancingStrategy, usize>,
    /// 小时模式统计
    pub hourly_patterns: HashMap<u64, usize>,
    /// 负载相关性数据 (load, response_time)
    pub load_correlations: Vec<(f64, f64)>,
}

impl Default for SchedulingHistory {
    fn default() -> Self {
        Self {
            decisions: VecDeque::new(),
            performance_metrics: VecDeque::new(),
            worker_performance_history: HashMap::new(),
        }
    }
}

impl Default for PerformancePredictor {
    fn default() -> Self {
        Self {
            weights: HashMap::new(),
            bias: 0.0,
            last_updated: Instant::now(),
            training_samples: 0,
        }
    }
}

impl Default for IntelligentScheduler {
    fn default() -> Self {
        Self::new(LearningConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_intelligent_scheduler_creation() {
        let scheduler = IntelligentScheduler::default();
        assert!(scheduler.learning_config.history_window_size > 0);
    }

    #[tokio::test]
    async fn test_simple_heuristic_selection() {
        let scheduler = IntelligentScheduler::default();

        // 创建模拟的工作线程信息
        let worker1 = WorkerInfo::new(0, 10);
        let worker2 = WorkerInfo::new(1, 10);

        // 设置不同的负载水平
        let mut worker1_mut = worker1.clone();
        worker1_mut.cpu_usage = 0.8; // 高负载
        worker1_mut.success_rate = 0.9;

        let mut worker2_mut = worker2.clone();
        worker2_mut.cpu_usage = 0.3; // 低负载
        worker2_mut.success_rate = 0.95;

        let workers = vec![&worker1_mut, &worker2_mut];

        // 应该选择负载较低的工作线程
        let selected = scheduler.simple_heuristic_selection(&workers);
        assert_eq!(selected, Some(1));
    }

    #[tokio::test]
    async fn test_scheduling_decision_recording() {
        let scheduler = IntelligentScheduler::default();

        let decision = SchedulingDecision {
            timestamp: Instant::now(),
            strategy: LoadBalancingStrategy::Adaptive,
            selected_worker: 0,
            system_state: SystemStateSnapshot {
                system_load: 0.5,
                avg_response_time: 100.0,
                worker_states: HashMap::new(),
                queue_length: 5,
            },
            result: SchedulingResult::Success {
                actual_response_time: 95.0,
                task_success: true,
            },
        };

        scheduler.record_decision(decision).await;

        let stats = scheduler.get_intelligent_stats().await;
        assert_eq!(stats.total_decisions, 1);
        assert_eq!(stats.successful_decisions, 1);
        assert!(stats.success_rate > 0.0);
    }
}
