//! 集成测试
//!
//! 测试 ZeroMQ 数据源、位移跟踪和窗口聚合的完整流程

#[cfg(test)]
mod integration_tests {
    use super::super::*;
    use crate::core::scheduler::SchedulerConfig;
    use crate::core::offset_tracker::{OffsetTracker, OffsetTrackerConfig, WindowData};
    use crate::core::window_aggregator::{SlidingWindowAggregator, WindowConfig, WindowBatch};
    use crate::core::ordered_window_processor::{OrderedWindowProcessor, OrderedWindowProcessorConfig};
    use crate::core::zeromq_source::{ZeroMQConfig, ZeroMQSocketType};
    use std::sync::Arc;
    use std::time::Duration;
    
    /// 测试完整的数据处理流程
    #[tokio::test]
    async fn test_complete_processing_flow() {
        // 创建配置
        let zmq_config = ZeroMQConfig {
            endpoint: "tcp://localhost:5557".to_string(),
            socket_type: ZeroMQSocketType::Pull,
            receive_timeout_ms: 1000,
            max_buffer_size: 1000,
            subscribe_filter: None,
        };
        
        let offset_config = OffsetTrackerConfig {
            max_waiting_offsets: 100,
            offset_timeout_ms: 5000,
        };
        
        let window_config = WindowConfig {
            window_size: 5,
            window_slide: 3,
            window_timeout_ms: 2000,
            allow_incomplete_window: true,
        };
        
        let processor_config = OrderedWindowProcessorConfig {
            zmq_config,
            offset_config,
            window_config,
        };
        
        // 创建任务调度器
        let scheduler_config = SchedulerConfig::default();
        let scheduler = Arc::new(TaskScheduler::new(scheduler_config));
        
        // 创建有序窗口处理器
        let processor = OrderedWindowProcessor::new(processor_config, scheduler)
            .expect("Failed to create ordered window processor");
        
        // 测试创建成功
        let stats = processor.get_stats().await;
        assert_eq!(stats.messages_received, 0);
        assert_eq!(stats.windows_processed, 0);
    }
    
    /// 测试位移跟踪和窗口聚合的集成
    #[tokio::test]
    async fn test_offset_tracker_and_window_aggregator_integration() {
        // 创建位移跟踪器
        let offset_tracker = Arc::new(OffsetTracker::new(OffsetTrackerConfig::default()));
        
        // 创建窗口聚合器（使用简单的回调）
        let mut triggered_batches = Arc::new(tokio::sync::Mutex::new(Vec::new()));
        let triggered_batches_clone = Arc::clone(&triggered_batches);
        
        let window_config = WindowConfig {
            window_size: 3,
            window_slide: 1,
            window_timeout_ms: 10000,
            allow_incomplete_window: false,
        };
        
        let aggregator = Arc::new(SlidingWindowAggregator::new(
            window_config,
            move |batch| {
                let triggered_batches = Arc::clone(&triggered_batches_clone);
                tokio::spawn(async move {
                    triggered_batches.lock().await.push(batch);
                });
                Ok(())
            },
        ));
        
        // 模拟接收消息
        let measurement_point_id = "1464";
        
        // 接收连续的消息
        for i in 1..=6 {
            let continuous_data = offset_tracker
                .receive_message(
                    measurement_point_id,
                    i,
                    i * 1000,
                    serde_json::json!({"value": i}),
                )
                .await
                .unwrap();
            
            if !continuous_data.is_empty() {
                aggregator.add_data(measurement_point_id, continuous_data).await.unwrap();
            }
        }
        
        // 等待窗口触发
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // 检查是否触发了窗口
        let batches = triggered_batches.lock().await;
        // 应该至少触发一个窗口（3个数据点）
        assert!(batches.len() >= 1);
    }
    
    /// 测试乱序消息处理
    #[tokio::test]
    async fn test_out_of_order_message_processing() {
        let offset_tracker = Arc::new(OffsetTracker::new(OffsetTrackerConfig::default()));
        let measurement_point_id = "1464";
        
        // 先接收序列号 3
        let result1 = offset_tracker
            .receive_message(measurement_point_id, 3, 3000, serde_json::json!({"value": 3}))
            .await
            .unwrap();
        assert_eq!(result1.len(), 0); // 没有连续数据
        
        // 再接收序列号 1
        let result2 = offset_tracker
            .receive_message(measurement_point_id, 1, 1000, serde_json::json!({"value": 1}))
            .await
            .unwrap();
        assert_eq!(result2.len(), 1); // 现在有连续数据了
        
        // 接收序列号 2
        let result3 = offset_tracker
            .receive_message(measurement_point_id, 2, 2000, serde_json::json!({"value": 2}))
            .await
            .unwrap();
        assert_eq!(result3.len(), 2); // 现在有连续数据 2 和 3
        
        assert_eq!(offset_tracker.get_committed_offset(measurement_point_id).await, 3);
    }
    
    /// 测试窗口超时触发
    #[tokio::test]
    async fn test_window_timeout_trigger() {
        let mut triggered_batches = Arc::new(tokio::sync::Mutex::new(Vec::new()));
        let triggered_batches_clone = Arc::clone(&triggered_batches);
        
        let window_config = WindowConfig {
            window_size: 10,
            window_slide: 5,
            window_timeout_ms: 500, // 短超时时间用于测试
            allow_incomplete_window: true,
        };
        
        let aggregator = Arc::new(SlidingWindowAggregator::new(
            window_config.clone(),
            move |batch| {
                let triggered_batches = Arc::clone(&triggered_batches_clone);
                tokio::spawn(async move {
                    triggered_batches.lock().await.push(batch);
                });
                Ok(())
            },
        ));
        
        // 启动超时检查
        aggregator.start_timeout_checker();
        
        // 添加少量数据（不足以触发窗口）
        let data = vec![
            WindowData { sequence: 1, timestamp: 1000, data: serde_json::json!({"value": 1}) },
            WindowData { sequence: 2, timestamp: 2000, data: serde_json::json!({"value": 2}) },
        ];
        
        aggregator.add_data("1464", data).await.unwrap();
        
        // 等待超时
        tokio::time::sleep(Duration::from_millis(600)).await;
        
        // 检查是否触发了超时窗口
        let batches = triggered_batches.lock().await;
        // 应该触发一个不完整窗口
        assert!(batches.len() >= 1);
    }
}

