//! 有序窗口处理器
//!
//! 整合 ZeroMQ 数据源、位移跟踪和滑动窗口聚合
//! 提供完整的有序数据处理流程

use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use crate::core::error::Result;
use crate::core::offset_tracker::{OffsetTracker, OffsetTrackerConfig, WindowData};
use crate::core::window_aggregator::{SlidingWindowAggregator, WindowConfig, WindowBatch};
use crate::core::zeromq_source::{ZeroMQSource, ZeroMQMessage, ZeroMQConfig};
use crate::core::scheduler::{TaskScheduler, ScheduledTask, TaskPriority};
use crate::core::types::ComputeRequest;

/// 有序窗口处理器
pub struct OrderedWindowProcessor {
    /// ZeroMQ 数据源
    zmq_source: Arc<ZeroMQSource>,
    /// 位移跟踪器
    offset_tracker: Arc<OffsetTracker>,
    /// 滑动窗口聚合器
    window_aggregator: Arc<SlidingWindowAggregator>,
    /// 任务调度器
    scheduler: Arc<TaskScheduler>,
    /// 统计信息
    stats: Arc<RwLock<ProcessorStats>>,
    /// 是否运行中
    is_running: Arc<RwLock<bool>>,
}

/// 处理器统计信息
#[derive(Debug, Clone, Default)]
pub struct ProcessorStats {
    /// 总接收消息数
    pub messages_received: u64,
    /// 总处理窗口数
    pub windows_processed: u64,
    /// 总错误数
    pub errors_count: u64,
    /// 当前等待处理的测量点数
    pub active_measurement_points: usize,
    /// 总连续数据点数
    pub total_continuous_data_points: u64,
}

/// 有序窗口处理器配置
#[derive(Debug, Clone)]
pub struct OrderedWindowProcessorConfig {
    /// ZeroMQ 配置
    pub zmq_config: ZeroMQConfig,
    /// 位移跟踪配置
    pub offset_config: OffsetTrackerConfig,
    /// 窗口配置
    pub window_config: WindowConfig,
}

impl Default for OrderedWindowProcessorConfig {
    fn default() -> Self {
        Self {
            zmq_config: ZeroMQConfig::default(),
            offset_config: OffsetTrackerConfig::default(),
            window_config: WindowConfig::default(),
        }
    }
}

impl OrderedWindowProcessor {
    /// 创建新的有序窗口处理器
    pub fn new(
        config: OrderedWindowProcessorConfig,
        scheduler: Arc<TaskScheduler>,
    ) -> Result<Self> {
        // 创建 ZeroMQ 数据源
        let zmq_source = Arc::new(ZeroMQSource::new(config.zmq_config.clone())?);
        
        // 创建位移跟踪器
        let offset_tracker = Arc::new(OffsetTracker::new(config.offset_config.clone()));
        
        // 创建窗口触发回调
        let scheduler_clone = Arc::clone(&scheduler);
        let trigger_callback = move |batch: WindowBatch| -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
            // 将窗口批次提交到任务调度器
            Self::submit_window_batch(Arc::clone(&scheduler_clone), batch)?;
            Ok(())
        };
        
        // 创建滑动窗口聚合器
        let window_aggregator = Arc::new(SlidingWindowAggregator::new(
            config.window_config.clone(),
            trigger_callback,
        ));
        
        Ok(Self {
            zmq_source,
            offset_tracker,
            window_aggregator,
            scheduler,
            stats: Arc::new(RwLock::new(ProcessorStats::default())),
            is_running: Arc::new(RwLock::new(false)),
        })
    }
    
    /// 启动处理器
    pub async fn start(&self) -> Result<()> {
        let mut is_running = self.is_running.write().await;
        if *is_running {
            return Err("Ordered window processor is already running".into());
        }
        
        tracing::info!("Starting ordered window processor");
        
        // 启动 ZeroMQ 数据源
        self.zmq_source.start().await?;
        
        // 启动窗口超时检查
        self.window_aggregator.start_timeout_checker();
        
        *is_running = true;
        
        // 启动消息处理循环
        let receiver = self.zmq_source.subscribe();
        let offset_tracker = Arc::clone(&self.offset_tracker);
        let window_aggregator = Arc::clone(&self.window_aggregator);
        let stats = Arc::clone(&self.stats);
        let is_running_clone = self.is_running.clone();
        
        tokio::spawn(async move {
            Self::process_loop(
                receiver,
                offset_tracker,
                window_aggregator,
                stats,
                is_running_clone,
            ).await;
        });
        
        tracing::info!("Ordered window processor started successfully");
        Ok(())
    }
    
    /// 停止处理器
    pub async fn stop(&self) -> Result<()> {
        let mut is_running = self.is_running.write().await;
        if !*is_running {
            return Ok(());
        }
        
        tracing::info!("Stopping ordered window processor");
        
        *is_running = false;
        
        // 停止 ZeroMQ 数据源
        self.zmq_source.stop().await?;
        
        tracing::info!("Ordered window processor stopped successfully");
        Ok(())
    }
    
    /// 消息处理循环
    async fn process_loop(
        mut receiver: mpsc::Receiver<ZeroMQMessage>,
        offset_tracker: Arc<OffsetTracker>,
        window_aggregator: Arc<SlidingWindowAggregator>,
        stats: Arc<RwLock<ProcessorStats>>,
        is_running: Arc<RwLock<bool>>,
    ) {
        tracing::info!("Starting message processing loop");
        
        loop {
            // 检查是否应该停止
            if !*is_running.read().await {
                tracing::info!("Message processing loop stopping");
                break;
            }
            
            // 接收消息
            match receiver.recv().await {
                Some(message) => {
                    // 更新统计
                    {
                        let mut stats = stats.write().await;
                        stats.messages_received += 1;
                    }
                    
                    // 接收消息到位移跟踪器
                    match offset_tracker
                        .receive_message(
                            &message.measurement_point_id,
                            message.sequence,
                            message.timestamp,
                            message.payload,
                        )
                        .await
                    {
                        Ok(continuous_data) => {
                            if !continuous_data.is_empty() {
                                // 更新统计
                                {
                                    let mut stats = stats.write().await;
                                    stats.total_continuous_data_points += continuous_data.len() as u64;
                                }
                                
                                // 添加到窗口聚合器
                                match window_aggregator
                                    .add_data(&message.measurement_point_id, continuous_data)
                                    .await
                                {
                                    Ok(_) => {
                                        // 窗口可能已触发，更新统计
                                        let mut stats = stats.write().await;
                                        stats.windows_processed += 1;
                                    }
                                    Err(e) => {
                                        tracing::error!(
                                            "Failed to add data to window aggregator: {}",
                                            e
                                        );
                                        let mut stats = stats.write().await;
                                        stats.errors_count += 1;
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!("Failed to receive message to offset tracker: {}", e);
                            let mut stats = stats.write().await;
                            stats.errors_count += 1;
                        }
                    }
                }
                None => {
                    tracing::info!("ZeroMQ receiver closed");
                    break;
                }
            }
        }
        
        tracing::info!("Message processing loop stopped");
    }
    
    /// 提交窗口批次到任务调度器
    fn submit_window_batch(
        scheduler: Arc<TaskScheduler>,
        batch: WindowBatch,
    ) -> Result<()> {
        // 创建计算请求
        let request = ComputeRequest {
            id: format!(
                "{}-{}-{}",
                batch.measurement_point_id, batch.start_offset, batch.end_offset
            ),
            algorithm: "window_aggregation".to_string(),
            parameters: serde_json::json!({
                "measurement_point_id": batch.measurement_point_id,
                "start_offset": batch.start_offset,
                "end_offset": batch.end_offset,
                "time_range": batch.time_range,
                "count": batch.count,
                "data": batch.data,
            }),
            timeout_seconds: Some(300),
        };
        
        // 创建调度任务
        let task = ScheduledTask::new(request)
            .with_priority(TaskPriority::Normal);
        
        // 提交任务（异步）
        let scheduler_clone = Arc::clone(&scheduler);
        tokio::spawn(async move {
            if let Err(e) = scheduler_clone.submit_task(task).await {
                tracing::error!("Failed to submit window batch task: {}", e);
            }
        });
        
        Ok(())
    }
    
    /// 获取统计信息
    pub async fn get_stats(&self) -> ProcessorStats {
        let stats = self.stats.read().await;
        
        // 获取活动测量点数
        let active_count = self.offset_tracker.get_active_count().await;
        
        ProcessorStats {
            messages_received: stats.messages_received,
            windows_processed: stats.windows_processed,
            errors_count: stats.errors_count,
            active_measurement_points: active_count,
            total_continuous_data_points: stats.total_continuous_data_points,
        }
    }
    
    /// 获取测量点的位移状态
    pub async fn get_offset_state(&self, measurement_point_id: &str) -> Option<crate::core::offset_tracker::OffsetStateInfo> {
        self.offset_tracker.get_offset_state(measurement_point_id).await
    }
    
    /// 获取测量点的窗口状态
    pub async fn get_window_status(&self, measurement_point_id: &str) -> Option<crate::core::window_aggregator::WindowStatus> {
        self.window_aggregator.get_window_status(measurement_point_id).await
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::scheduler::SchedulerConfig;
    
    #[tokio::test]
    async fn test_ordered_window_processor_creation() {
        let config = OrderedWindowProcessorConfig::default();
        let scheduler_config = SchedulerConfig::default();
        let scheduler = Arc::new(TaskScheduler::new(scheduler_config));
        
        let processor = OrderedWindowProcessor::new(config, scheduler);
        assert!(processor.is_ok());
    }
    
    #[tokio::test]
    async fn test_processor_stats() {
        let config = OrderedWindowProcessorConfig::default();
        let scheduler_config = SchedulerConfig::default();
        let scheduler = Arc::new(TaskScheduler::new(scheduler_config));
        
        let processor = OrderedWindowProcessor::new(config, scheduler).unwrap();
        let stats = processor.get_stats().await;
        
        assert_eq!(stats.messages_received, 0);
        assert_eq!(stats.windows_processed, 0);
        assert_eq!(stats.errors_count, 0);
    }
}

