//! 核心模块测试
//!
//! 测试核心功能是否正常工作

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::{LoadBalancingStrategy, WorkerInfo};
    use crate::core::load_balancer::LoadBalancerConfig;
    use crate::core::scheduler::{SchedulerConfig, TaskScheduler};

    #[test]
    fn test_worker_info_creation() {
        let worker = WorkerInfo::new(0, 10);
        assert_eq!(worker.id, 0);
        assert_eq!(worker.max_connections, 10);
        assert_eq!(worker.current_connections, 0);
        assert!(worker.is_healthy);
    }

    #[test]
    fn test_worker_load_score() {
        let worker = WorkerInfo::new(0, 10);

        // 空闲工作线程的负载分数应该很低
        let load_score = worker.load_score();
        assert!(load_score >= 0.0 && load_score <= 1.0);

        // 高负载工作线程
        let mut high_load_worker = worker.clone();
        high_load_worker.cpu_usage = 0.9;
        high_load_worker.memory_usage = 0.8;
        high_load_worker.current_connections = 9;

        let high_load_score = high_load_worker.load_score();
        assert!(high_load_score > load_score);
    }

    #[test]
    fn test_load_balancer_config() {
        let config = LoadBalancerConfig::default();
        assert_eq!(config.strategy, LoadBalancingStrategy::Adaptive);
        assert!(!config.intelligent_scheduling_enabled);
        assert!(config.max_connections_per_worker > 0);
    }

    #[test]
    fn test_scheduler_config() {
        let config = SchedulerConfig::default();
        assert!(!config.intelligent_scheduling_enabled);
        assert!(config.max_concurrent_tasks > 0);
        assert!(config.queue_size > 0);
    }

    #[tokio::test]
    async fn test_scheduler_creation() {
        let config = SchedulerConfig::default();
        let scheduler = TaskScheduler::new(config);

        // 测试智能调度状态
        let status = scheduler.get_intelligent_scheduling_status();
        assert!(!status.enabled);
        assert_eq!(status.strategy, LoadBalancingStrategy::Adaptive);
    }
}
