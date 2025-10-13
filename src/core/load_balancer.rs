//! 负载均衡器模块
//!
//! 提供智能的工作线程负载均衡功能，支持多种调度策略

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use crate::core::types::{LoadBalancingStrategy, WorkerInfo, DynamicStrategyAdjuster, PerformanceThresholds};
use crate::core::intelligent_scheduler::IntelligentScheduler;

/// 负载均衡器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancerConfig {
    /// 调度策略
    pub strategy: LoadBalancingStrategy,
    /// 是否启用智能调度
    pub intelligent_scheduling_enabled: bool,
    /// 健康检查间隔 (秒)
    pub health_check_interval: u64,
    /// 工作线程最大连接数
    pub max_connections_per_worker: usize,
    /// 负载均衡器更新间隔 (毫秒)
    pub update_interval_ms: u64,
    /// 自适应调度阈值
    pub adaptive_threshold: f64,
}

impl Default for LoadBalancerConfig {
    fn default() -> Self {
        Self {
            strategy: LoadBalancingStrategy::Adaptive,
            intelligent_scheduling_enabled: false, // 默认禁用智能调度
            health_check_interval: 30,
            max_connections_per_worker: 10,
            update_interval_ms: 1000,
            adaptive_threshold: 0.8,
        }
    }
}

/// 负载均衡器
pub struct LoadBalancer {
    /// 配置
    config: LoadBalancerConfig,
    /// 工作线程信息
    workers: Arc<RwLock<HashMap<usize, WorkerInfo>>>,
    /// 轮询调度索引
    round_robin_index: Arc<Mutex<usize>>,
    /// 统计信息
    stats: Arc<Mutex<LoadBalancerStats>>,
    /// 动态策略调整器
    dynamic_strategy_adjuster: Arc<Mutex<DynamicStrategyAdjuster>>,
    /// 智能调度器
    intelligent_scheduler: Arc<IntelligentScheduler>,
}

#[derive(Debug, Clone, Default)]
pub struct LoadBalancerStats {
    /// 总请求数
    pub total_requests: u64,
    /// 成功分配数
    pub successful_assignments: u64,
    /// 拒绝分配数
    pub rejected_assignments: u64,
    /// 平均响应时间
    pub avg_response_time: f64,
}

/// 负载均衡器状态
#[derive(Debug, Clone, Serialize)]
pub struct LoadBalancerStatus {
    /// 当前调度策略
    pub strategy: LoadBalancingStrategy,
    /// 总工作线程数
    pub total_workers: usize,
    /// 可用工作线程数
    pub available_workers: usize,
    /// 工作线程状态列表
    pub worker_statuses: Vec<WorkerStatus>,
    /// 统计信息
    pub stats: LoadBalancerStats,
}

/// 工作线程状态
#[derive(Debug, Clone, Serialize)]
pub struct WorkerStatus {
    /// 工作线程ID
    pub id: usize,
    /// 当前连接数
    pub current_connections: usize,
    /// 最大连接数
    pub max_connections: usize,
    /// 负载评分
    pub load_score: f64,
    /// 容量评分
    pub capacity_score: f64,
    /// 是否健康
    pub is_healthy: bool,
    /// 总任务数
    pub total_tasks: u64,
    /// 成功率
    pub success_rate: f64,
}

/// 负载均衡器错误
#[derive(Debug, thiserror::Error)]
pub enum LoadBalancerError {
    #[error("No available workers")]
    NoAvailableWorkers,
    #[error("Worker not found: {0}")]
    WorkerNotFound(usize),
    #[error("Load balancer not initialized")]
    NotInitialized,
}

impl LoadBalancer {
    /// 创建新的负载均衡器
    pub fn new(config: LoadBalancerConfig) -> Self {
        use crate::core::intelligent_scheduler::LearningConfig;

        let learning_config = LearningConfig::default();
        let intelligent_scheduler = Arc::new(IntelligentScheduler::new(learning_config));

        Self {
            config,
            workers: Arc::new(RwLock::new(HashMap::new())),
            round_robin_index: Arc::new(Mutex::new(0)),
            stats: Arc::new(Mutex::new(LoadBalancerStats::default())),
            dynamic_strategy_adjuster: Arc::new(Mutex::new(DynamicStrategyAdjuster::default())),
            intelligent_scheduler,
        }
    }

    /// 默认配置创建
    pub fn default() -> Self {
        Self::new(LoadBalancerConfig::default())
    }

    /// 注册工作线程
    pub async fn register_worker(&self, worker_id: usize) {
        let worker_info = WorkerInfo::new(worker_id, self.config.max_connections_per_worker);
        let mut workers = self.workers.write().await;
        workers.insert(worker_id, worker_info);
        tracing::info!("Registered worker {} with load balancer", worker_id);
    }

    /// 注销工作线程
    pub async fn unregister_worker(&self, worker_id: usize) {
        let mut workers = self.workers.write().await;
        workers.remove(&worker_id);
        tracing::info!("Unregistered worker {} from load balancer", worker_id);
    }

    /// 更新工作线程状态
    pub async fn update_worker_status(
        &self,
        worker_id: usize,
        cpu_usage: f64,
        memory_usage: f64,
        response_time: f64,
        is_healthy: bool,
    ) {
        let mut workers = self.workers.write().await;
        if let Some(worker) = workers.get_mut(&worker_id) {
            worker.update_metrics(cpu_usage, memory_usage, response_time);
            worker.is_healthy = is_healthy;
        }
    }

    /// 记录任务分配结果
    pub async fn record_task_assignment(&self, worker_id: usize, success: bool) {
        let mut workers = self.workers.write().await;
        if let Some(worker) = workers.get_mut(&worker_id) {
            worker.record_task_result(success);
        }

        let mut stats = self.stats.lock().await;
        stats.total_requests += 1;
        if success {
            stats.successful_assignments += 1;
        }
    }

    /// 选择最优工作线程
    pub async fn select_worker(&self) -> Result<usize, LoadBalancerError> {
        let mut stats = self.stats.lock().await;
        stats.total_requests += 1;

        let workers = self.workers.read().await;

        // 过滤健康的且有容量的线程
        let available_workers: Vec<&WorkerInfo> = workers.values()
            .filter(|w| w.is_healthy && w.current_connections < w.max_connections)
            .collect();

        if available_workers.is_empty() {
            stats.rejected_assignments += 1;
            return Err(LoadBalancerError::NoAvailableWorkers);
        }

        // 检查是否启用智能调度
        let selected_worker_id = if self.config.intelligent_scheduling_enabled {
            // 尝试使用智能调度
            if let Some(worker_id) = self.try_intelligent_selection(&available_workers).await {
                worker_id
            } else {
                // 智能调度失败，回退到传统调度策略
                tracing::debug!("Intelligent scheduling failed, falling back to traditional strategy");
                self.select_traditional_strategy(&available_workers).await
            }
        } else {
            // 使用传统调度策略
            self.select_traditional_strategy(&available_workers).await
        };

        // 更新选中工作线程的连接数
        drop(workers); // 释放读锁
        let mut workers_write = self.workers.write().await;
        if let Some(worker) = workers_write.get_mut(&selected_worker_id) {
            worker.increment_connections();
        }

        Ok(selected_worker_id)
    }

    /// 尝试使用智能调度选择工作线程
    async fn try_intelligent_selection(&self, workers: &[&WorkerInfo]) -> Option<usize> {
        // 计算当前系统状态
        let system_load = self.calculate_system_load().await;
        let avg_response_time = self.calculate_average_response_time().await;
        let queue_length = 0; // 这里需要从调度器获取实际队列长度

        // 使用智能调度器进行预测
        self.intelligent_scheduler.predict_optimal_worker(
            workers,
            system_load,
            queue_length,
        ).await
    }

    /// 选择传统调度策略
    async fn select_traditional_strategy(&self, workers: &[&WorkerInfo]) -> usize {
        match self.config.strategy {
            LoadBalancingStrategy::RoundRobin => self.select_round_robin(workers).await,
            LoadBalancingStrategy::LeastConnections => self.select_least_connections(workers),
            LoadBalancingStrategy::Weighted => self.select_weighted(workers),
            LoadBalancingStrategy::Random => self.select_random(workers),
            LoadBalancingStrategy::Adaptive => self.select_adaptive(workers),
            LoadBalancingStrategy::LoadAware => self.select_load_aware(workers),
            LoadBalancingStrategy::ResponseTimeAware => self.select_response_time_aware(workers),
            LoadBalancingStrategy::ResourceAware => self.select_resource_aware(workers),
        }
    }

    /// 释放工作线程连接
    pub async fn release_worker(&self, worker_id: usize) {
        let mut workers = self.workers.write().await;
        if let Some(worker) = workers.get_mut(&worker_id) {
            worker.decrement_connections();
        }
    }

    /// 轮询调度
    async fn select_round_robin(&self, workers: &[&WorkerInfo]) -> usize {
        let mut index = self.round_robin_index.lock().await;
        let selected = workers[*index % workers.len()].id;
        *index += 1;
        selected
    }

    /// 最少连接调度
    fn select_least_connections(&self, workers: &[&WorkerInfo]) -> usize {
        workers.iter()
            .min_by_key(|w| w.current_connections)
            .map(|w| w.id)
            .unwrap_or(workers[0].id)
    }

    /// 权重调度
    fn select_weighted(&self, workers: &[&WorkerInfo]) -> usize {
        let total_weight: usize = workers.iter().map(|w| w.weight).sum();
        if total_weight == 0 {
            return workers[0].id;
        }

        let mut random_weight = fastrand::usize(0..total_weight);
        for worker in workers {
            if random_weight < worker.weight {
                return worker.id;
            }
            random_weight -= worker.weight;
        }

        workers[0].id
    }

    /// 随机调度
    fn select_random(&self, workers: &[&WorkerInfo]) -> usize {
        let index = fastrand::usize(0..workers.len());
        workers[index].id
    }

    /// 自适应调度
    fn select_adaptive(&self, workers: &[&WorkerInfo]) -> usize {
        // 基于多维度指标选择最优工作线程
        workers.iter()
            .min_by(|a, b| {
                let score_a = self.calculate_adaptive_score(a);
                let score_b = self.calculate_adaptive_score(b);
                score_a.partial_cmp(&score_b).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|w| w.id)
            .unwrap_or(workers[0].id)
    }

    /// 基于负载预测的选择
    fn select_load_aware(&self, workers: &[&WorkerInfo]) -> usize {
        // 考虑当前负载和预测负载
        workers.iter()
            .min_by(|a, b| {
                let predicted_load_a = self.predict_worker_load(a);
                let predicted_load_b = self.predict_worker_load(b);
                predicted_load_a.partial_cmp(&predicted_load_b).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|w| w.id)
            .unwrap_or(workers[0].id)
    }

    /// 基于响应时间的选择
    fn select_response_time_aware(&self, workers: &[&WorkerInfo]) -> usize {
        // 选择响应时间最短的工作线程
        workers.iter()
            .min_by(|a, b| {
                a.avg_response_time.partial_cmp(&b.avg_response_time).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|w| w.id)
            .unwrap_or(workers[0].id)
    }

    /// 基于资源利用率的选择
    fn select_resource_aware(&self, workers: &[&WorkerInfo]) -> usize {
        // 选择资源利用率最低的工作线程
        workers.iter()
            .min_by(|a, b| {
                let utilization_a = (a.cpu_usage + a.memory_usage) / 2.0;
                let utilization_b = (b.cpu_usage + b.memory_usage) / 2.0;
                utilization_a.partial_cmp(&utilization_b).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|w| w.id)
            .unwrap_or(workers[0].id)
    }

    /// 计算自适应评分
    fn calculate_adaptive_score(&self, worker: &WorkerInfo) -> f64 {
        let load_score = worker.load_score();
        let capacity_score = worker.capacity_score();
        let success_rate = worker.success_rate();
        let response_time_penalty = (worker.avg_response_time / 1000.0).min(1.0); // 归一化到0-1

        // 综合评分：容量权重40%，负载惩罚权重30%，成功率权重20%，响应时间惩罚权重10%
        (capacity_score * 0.4) +
        ((1.0 - load_score) * 0.3) +
        (success_rate * 0.2) +
        ((1.0 - response_time_penalty) * 0.1)
    }

    /// 预测工作线程负载
    fn predict_worker_load(&self, worker: &WorkerInfo) -> f64 {
        // 基于当前负载和趋势预测未来负载
        let current_load = worker.load_score();
        let trend_factor = self.calculate_load_trend(worker);

        // 预测负载 = 当前负载 + 趋势因子
        (current_load + trend_factor).min(1.0)
    }

    /// 计算负载趋势
    fn calculate_load_trend(&self, worker: &WorkerInfo) -> f64 {
        // 简化的趋势计算：基于最近的任务执行情况
        // 在实际实现中，这应该基于历史数据进行更复杂的趋势分析

        let success_rate = worker.success_rate();
        let avg_response_time = worker.avg_response_time;

        // 如果成功率高且响应时间低，趋势为下降（负载会减少）
        // 如果成功率低或响应时间高，趋势为上升（负载会增加）
        let trend = if success_rate > 0.95 && avg_response_time < 100.0 {
            -0.1 // 负载趋势下降
        } else if success_rate < 0.85 || avg_response_time > 200.0 {
            0.1 // 负载趋势上升
        } else {
            0.0 // 负载稳定
        };

        trend
    }

    /// 计算系统整体负载
    async fn calculate_system_load(&self) -> f64 {
        let workers = self.workers.read().await;

        if workers.is_empty() {
            return 0.0;
        }

        let total_load: f64 = workers.values().map(|w| w.load_score()).sum();
        total_load / workers.len() as f64
    }

    /// 计算平均响应时间
    async fn calculate_average_response_time(&self) -> f64 {
        let workers = self.workers.read().await;

        if workers.is_empty() {
            return 0.0;
        }

        let total_response_time: f64 = workers.values().map(|w| w.avg_response_time).sum();
        total_response_time / workers.len() as f64
    }

    /// 动态调整调度策略
    pub async fn adjust_strategy_dynamically(&self) -> Option<LoadBalancingStrategy> {
        let mut adjuster = self.dynamic_strategy_adjuster.lock().await;

        // 检查是否达到最小调整间隔
        if adjuster.last_adjustment.elapsed() < Duration::from_secs(adjuster.performance_thresholds.min_adjustment_interval) {
            return None;
        }

        // 计算当前系统负载
        let system_load = self.calculate_system_load().await;
        adjuster.system_load_level = system_load;

        // 计算平均响应时间
        let avg_response_time = self.calculate_average_response_time().await;

        // 根据负载和响应时间决定新的策略
        let new_strategy = self.determine_optimal_strategy(system_load, avg_response_time);

        // 如果策略发生变化，记录历史
        if new_strategy != self.config.strategy {
            adjuster.strategy_history.push((new_strategy.clone(), Instant::now()));
            adjuster.last_adjustment = Instant::now();

            tracing::info!(
                "Dynamically adjusted load balancing strategy from {:?} to {:?} (load: {:.2}, response_time: {:.2}ms)",
                self.config.strategy, new_strategy, system_load, avg_response_time
            );

            Some(new_strategy)
        } else {
            None
        }
    }

    /// 根据系统状态确定最优策略
    fn determine_optimal_strategy(&self, system_load: f64, avg_response_time: f64) -> LoadBalancingStrategy {
        let adjuster = self.dynamic_strategy_adjuster.lock().unwrap();

        // 高负载情况
        if system_load > adjuster.performance_thresholds.high_load_threshold {
            if avg_response_time > adjuster.performance_thresholds.high_response_time_threshold {
                // 高负载且高响应时间：使用资源感知调度
                LoadBalancingStrategy::ResourceAware
            } else {
                // 高负载但响应时间正常：使用最少连接调度
                LoadBalancingStrategy::LeastConnections
            }
        }
        // 中等负载情况
        else if system_load > adjuster.performance_thresholds.low_load_threshold {
            if avg_response_time > adjuster.performance_thresholds.high_response_time_threshold {
                // 中等负载但响应时间高：使用响应时间感知调度
                LoadBalancingStrategy::ResponseTimeAware
            } else {
                // 中等负载且响应时间正常：使用自适应调度
                LoadBalancingStrategy::Adaptive
            }
        }
        // 低负载情况
        else {
            if avg_response_time < adjuster.performance_thresholds.low_response_time_threshold {
                // 低负载且响应时间低：使用负载感知调度（预测性）
                LoadBalancingStrategy::LoadAware
            } else {
                // 低负载但响应时间正常：使用轮询调度（简单高效）
                LoadBalancingStrategy::RoundRobin
            }
        }
    }

    /// 获取动态调整统计信息
    pub async fn get_dynamic_adjustment_stats(&self) -> DynamicAdjustmentStats {
        let adjuster = self.dynamic_strategy_adjuster.lock().await;

        DynamicAdjustmentStats {
            current_system_load: adjuster.system_load_level,
            strategy_changes: adjuster.strategy_history.len(),
            last_adjustment: adjuster.last_adjustment,
            performance_thresholds: adjuster.performance_thresholds.clone(),
        }
    }

    /// 重置动态调整器
    pub async fn reset_dynamic_adjuster(&self) {
        let mut adjuster = self.dynamic_strategy_adjuster.lock().await;
        *adjuster = DynamicStrategyAdjuster::default();
        tracing::info!("Reset dynamic strategy adjuster");
    }

    /// 获取负载均衡器状态
    pub async fn get_status(&self) -> LoadBalancerStatus {
        let workers = self.workers.read().await;
        let stats = self.stats.lock().await;

        let worker_statuses: Vec<WorkerStatus> = workers.values()
            .map(|w| WorkerStatus {
                id: w.id,
                current_connections: w.current_connections,
                max_connections: w.max_connections,
                load_score: w.load_score(),
                capacity_score: w.capacity_score(),
                is_healthy: w.is_healthy,
                total_tasks: w.total_tasks,
                success_rate: w.success_rate(),
            })
            .collect();

        LoadBalancerStatus {
            strategy: self.config.strategy.clone(),
            total_workers: workers.len(),
            available_workers: workers.values().filter(|w| w.is_healthy && w.current_connections < w.max_connections).count(),
            worker_statuses,
            stats: stats.clone(),
        }
    }

    /// 健康检查
    pub async fn health_check(&self) -> bool {
        let workers = self.workers.read().await;

        // 检查是否有健康的worker
        let healthy_workers = workers.values()
            .filter(|w| w.is_healthy && !w.is_expired())
            .count();

        healthy_workers > 0
    }

    /// 清理过期的工作线程信息
    pub async fn cleanup_expired_workers(&self) {
        let mut workers = self.workers.write().await;
        let expired_ids: Vec<usize> = workers.values()
            .filter(|w| w.is_expired())
            .map(|w| w.id)
            .collect();

        for id in expired_ids {
            workers.remove(&id);
            tracing::warn!("Removed expired worker {}", id);
        }
    }
}

/// 动态调整统计信息
#[derive(Debug, Clone, Serialize)]
pub struct DynamicAdjustmentStats {
    /// 当前系统负载水平
    pub current_system_load: f64,
    /// 策略变更次数
    pub strategy_changes: usize,
    /// 最后调整时间
    pub last_adjustment: Instant,
    /// 性能阈值
    pub performance_thresholds: PerformanceThresholds,
}

impl Default for LoadBalancer {
    fn default() -> Self {
        Self::new(LoadBalancerConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_round_robin_selection() {
        let lb = LoadBalancer::new(LoadBalancerConfig {
            strategy: LoadBalancingStrategy::RoundRobin,
            ..Default::default()
        });

        // 注册工作线程
        lb.register_worker(0).await;
        lb.register_worker(1).await;
        lb.register_worker(2).await;

        // 测试轮询
        let worker1 = lb.select_worker().await.unwrap();
        let worker2 = lb.select_worker().await.unwrap();
        let worker3 = lb.select_worker().await.unwrap();
        let worker4 = lb.select_worker().await.unwrap();

        assert_eq!(worker1, 0);
        assert_eq!(worker2, 1);
        assert_eq!(worker3, 2);
        assert_eq!(worker4, 0); // 循环回到第一个
    }

    #[tokio::test]
    async fn test_least_connections_selection() {
        let lb = LoadBalancer::new(LoadBalancerConfig {
            strategy: LoadBalancingStrategy::LeastConnections,
            ..Default::default()
        });

        // 注册工作线程
        lb.register_worker(0).await;
        lb.register_worker(1).await;

        // 手动设置连接数
        {
            let mut workers = lb.workers.write().await;
            workers.get_mut(&0).unwrap().current_connections = 5;
            workers.get_mut(&1).unwrap().current_connections = 2;
        }

        // 应该选择连接数最少的工作线程
        let selected = lb.select_worker().await.unwrap();
        assert_eq!(selected, 1);
    }

    #[tokio::test]
    async fn test_adaptive_selection() {
        let lb = LoadBalancer::new(LoadBalancerConfig {
            strategy: LoadBalancingStrategy::Adaptive,
            ..Default::default()
        });

        // 注册工作线程
        lb.register_worker(0).await;
        lb.register_worker(1).await;

        // 更新工作线程状态
        lb.update_worker_status(0, 0.9, 0.8, 100.0, true).await; // 高负载
        lb.update_worker_status(1, 0.2, 0.3, 50.0, true).await;  // 低负载

        // 应该选择负载较低的工作线程
        let selected = lb.select_worker().await.unwrap();
        assert_eq!(selected, 1);
    }
}