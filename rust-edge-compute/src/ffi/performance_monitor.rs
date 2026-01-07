//! 性能监控器模块
//!
//! 提供跨语言调用的性能监控功能

use std::future::Future;
use std::time::Instant;

/// 性能监控器
pub struct PerformanceMonitor;

impl Default for PerformanceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl PerformanceMonitor {
    /// 创建新的性能监控器
    pub fn new() -> Self {
        Self
    }

    /// 执行带监控的异步操作
    pub async fn execute_with_monitoring<F, Fut, R>(
        &self,
        _operation_name: &str,
        operation: F,
    ) -> Result<R, Box<dyn std::error::Error + Send + Sync>>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<R, Box<dyn std::error::Error + Send + Sync>>>,
    {
        let start_time = Instant::now();
        let result = operation().await;
        let duration = start_time.elapsed();

        println!("Operation '{}' took {:?}", _operation_name, duration);
        result
    }
}
