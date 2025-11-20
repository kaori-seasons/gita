//! 滑动窗口聚合器
//!
//! 按窗口大小聚合连续的数据，支持滑动窗口和固定窗口
//! 提供窗口触发机制，支持基于窗口大小和超时时间的混合触发

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{Duration, Instant};
use serde::{Deserialize, Serialize};

use crate::core::offset_tracker::WindowData;

/// 滑动窗口聚合器
pub struct SlidingWindowAggregator {
    /// 窗口配置
    config: WindowConfig,
    /// 按测量点分组的窗口缓冲区
    windows: Arc<RwLock<HashMap<String, WindowBuffer>>>,
    /// 窗口触发回调
    trigger_callback: Arc<dyn Fn(WindowBatch) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> + Send + Sync>,
}

/// 窗口配置
#[derive(Debug, Clone)]
pub struct WindowConfig {
    /// 窗口大小（数据点数量）
    pub window_size: usize,
    /// 窗口滑动步长
    pub window_slide: usize,
    /// 窗口超时时间（毫秒）
    pub window_timeout_ms: u64,
    /// 是否允许不完整窗口
    pub allow_incomplete_window: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            window_size: 100,
            window_slide: 50,
            window_timeout_ms: 5000,
            allow_incomplete_window: true,
        }
    }
}

/// 窗口缓冲区
#[derive(Debug, Clone)]
pub struct WindowBuffer {
    /// 测量点ID
    pub measurement_point_id: String,
    /// 窗口数据（按位移排序）
    pub data: VecDeque<WindowData>,
    /// 当前窗口的起始位移
    pub window_start_offset: u64,
    /// 当前窗口的结束位移
    pub window_end_offset: u64,
    /// 窗口创建时间
    pub window_created_at: Instant,
    /// 最后更新时间
    pub last_updated_at: Instant,
}

/// 窗口批次（触发时输出）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowBatch {
    /// 测量点ID
    pub measurement_point_id: String,
    /// 窗口起始位移
    pub start_offset: u64,
    /// 窗口结束位移
    pub end_offset: u64,
    /// 窗口数据（按位移排序）
    pub data: Vec<WindowData>,
    /// 窗口时间范围
    pub time_range: (u64, u64),
    /// 数据点数量
    pub count: usize,
}

impl SlidingWindowAggregator {
    /// 创建新的滑动窗口聚合器
    pub fn new<F>(config: WindowConfig, trigger_callback: F) -> Self
    where
        F: Fn(WindowBatch) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> + Send + Sync + 'static,
    {
        Self {
            config,
            windows: Arc::new(RwLock::new(HashMap::new())),
            trigger_callback: Arc::new(trigger_callback),
        }
    }
    
    /// 添加数据到窗口
    pub async fn add_data(
        &self,
        measurement_point_id: &str,
        data: Vec<WindowData>,
    ) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if data.is_empty() {
            return Ok(());
        }
        
        let mut windows = self.windows.write().await;
        
        // 获取或创建窗口缓冲区
        let window = windows
            .entry(measurement_point_id.to_string())
            .or_insert_with(|| WindowBuffer {
                measurement_point_id: measurement_point_id.to_string(),
                data: VecDeque::new(),
                window_start_offset: 0,
                window_end_offset: 0,
                window_created_at: Instant::now(),
                last_updated_at: Instant::now(),
            });
        
        // 添加数据到窗口缓冲区
        for item in data {
            window.data.push_back(item);
        }
        
        // 更新最后更新时间
        window.last_updated_at = Instant::now();
        
        // 检查是否需要触发窗口
        self.check_window_trigger(&mut windows, measurement_point_id).await?;
        
        Ok(())
    }
    
    /// 检查窗口触发条件
    async fn check_window_trigger(
        &self,
        windows: &mut HashMap<String, WindowBuffer>,
        measurement_point_id: &str,
    ) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let window = windows.get_mut(measurement_point_id).unwrap();
        
        // 检查窗口大小是否达到阈值
        if window.data.len() >= self.config.window_size {
            // 触发窗口
            self.trigger_window(windows, measurement_point_id).await?;
        }
        
        // 检查窗口超时
        let elapsed = window.last_updated_at.elapsed();
        if elapsed.as_millis() as u64 >= self.config.window_timeout_ms {
            // 窗口超时，触发不完整窗口（如果允许）
            if self.config.allow_incomplete_window && !window.data.is_empty() {
                self.trigger_window(windows, measurement_point_id).await?;
            }
        }
        
        Ok(())
    }
    
    /// 触发窗口
    async fn trigger_window(
        &self,
        windows: &mut HashMap<String, WindowBuffer>,
        measurement_point_id: &str,
    ) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let window = windows.get_mut(measurement_point_id).unwrap();
        
        // 提取窗口数据
        let window_size = self.config.window_size.min(window.data.len());
        let mut window_data: Vec<WindowData> = Vec::with_capacity(window_size);
        
        for _ in 0..window_size {
            if let Some(data) = window.data.pop_front() {
                window_data.push(data);
            }
        }
        
        if window_data.is_empty() {
            return Ok(());
        }
        
        // 计算窗口范围
        let start_offset = window_data.first().map(|d| d.sequence).unwrap_or(0);
        let end_offset = window_data.last().map(|d| d.sequence).unwrap_or(0);
        let start_time = window_data.first().map(|d| d.timestamp).unwrap_or(0);
        let end_time = window_data.last().map(|d| d.timestamp).unwrap_or(0);
        
        // 更新窗口起始位移
        window.window_start_offset = end_offset + 1;
        
        // 创建窗口批次
        let batch = WindowBatch {
            measurement_point_id: measurement_point_id.to_string(),
            start_offset,
            end_offset,
            data: window_data,
            time_range: (start_time, end_time),
            count: window_size,
        };
        
        // 调用触发回调
        (self.trigger_callback)(batch)?;
        
        Ok(())
    }
    
    /// 启动窗口超时检查任务
    pub fn start_timeout_checker(&self) {
        let windows = Arc::clone(&self.windows);
        let config = self.config.clone();
        let trigger_callback = Arc::clone(&self.trigger_callback);
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(1000));
            
            loop {
                interval.tick().await;
                
                // 检查所有窗口的超时
                let mut windows_guard = windows.write().await;
                let measurement_point_ids: Vec<String> = windows_guard.keys().cloned().collect();
                
                for measurement_point_id in measurement_point_ids {
                    let window = windows_guard.get(&measurement_point_id).unwrap();
                    let elapsed = window.last_updated_at.elapsed();
                    
                    if elapsed.as_millis() as u64 >= config.window_timeout_ms {
                        // 窗口超时，触发不完整窗口（如果允许）
                        if config.allow_incomplete_window && !window.data.is_empty() {
                            // 需要重新获取可变引用
                            drop(windows_guard);
                            let mut windows_guard = windows.write().await;
                            
                            // 触发窗口
                            if let Some(window) = windows_guard.get_mut(&measurement_point_id) {
                                let window_size = window.data.len();
                                if window_size > 0 {
                                    let mut window_data: Vec<WindowData> = Vec::with_capacity(window_size);
                                    
                                    for _ in 0..window_size {
                                        if let Some(data) = window.data.pop_front() {
                                            window_data.push(data);
                                        }
                                    }
                                    
                                    if !window_data.is_empty() {
                                        let start_offset = window_data.first().map(|d| d.sequence).unwrap_or(0);
                                        let end_offset = window_data.last().map(|d| d.sequence).unwrap_or(0);
                                        let start_time = window_data.first().map(|d| d.timestamp).unwrap_or(0);
                                        let end_time = window_data.last().map(|d| d.timestamp).unwrap_or(0);
                                        
                                        let batch = WindowBatch {
                                            measurement_point_id: measurement_point_id.clone(),
                                            start_offset,
                                            end_offset,
                                            data: window_data,
                                            time_range: (start_time, end_time),
                                            count: window_size,
                                        };
                                        
                                        if let Err(e) = (trigger_callback)(batch) {
                                            tracing::error!("Failed to trigger timeout window: {}", e);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });
    }
    
    /// 获取窗口状态信息
    pub async fn get_window_status(&self, measurement_point_id: &str) -> Option<WindowStatus> {
        let windows = self.windows.read().await;
        windows.get(measurement_point_id).map(|window| WindowStatus {
            measurement_point_id: measurement_point_id.to_string(),
            data_count: window.data.len(),
            window_start_offset: window.window_start_offset,
            window_end_offset: window.window_end_offset,
            window_created_at: window.window_created_at,
            last_updated_at: window.last_updated_at,
        })
    }
}

/// 窗口状态信息（用于监控）
#[derive(Debug, Clone)]
pub struct WindowStatus {
    /// 测量点ID
    pub measurement_point_id: String,
    /// 当前数据点数量
    pub data_count: usize,
    /// 窗口起始位移
    pub window_start_offset: u64,
    /// 窗口结束位移
    pub window_end_offset: u64,
    /// 窗口创建时间
    pub window_created_at: Instant,
    /// 最后更新时间
    pub last_updated_at: Instant,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::offset_tracker::WindowData;
    
    #[tokio::test]
    async fn test_window_aggregator_creation() {
        let config = WindowConfig::default();
        let aggregator = SlidingWindowAggregator::new(config, |_batch| Ok(()));
        
        // 测试创建成功
        assert!(aggregator.get_window_status("test").await.is_none());
    }
    
    #[tokio::test]
    async fn test_window_trigger_by_size() {
        let mut triggered_batches = Vec::new();
        let config = WindowConfig {
            window_size: 3,
            window_slide: 1,
            window_timeout_ms: 10000,
            allow_incomplete_window: false,
        };
        
        let aggregator = SlidingWindowAggregator::new(config, |batch| {
            triggered_batches.push(batch);
            Ok(())
        });
        
        // 添加数据，达到窗口大小
        let data = vec![
            WindowData { sequence: 1, timestamp: 1000, data: serde_json::json!({"value": 1}) },
            WindowData { sequence: 2, timestamp: 2000, data: serde_json::json!({"value": 2}) },
            WindowData { sequence: 3, timestamp: 3000, data: serde_json::json!({"value": 3}) },
        ];
        
        aggregator.add_data("1464", data).await.unwrap();
        
        // 应该触发一个窗口
        assert_eq!(triggered_batches.len(), 1);
        assert_eq!(triggered_batches[0].count, 3);
        assert_eq!(triggered_batches[0].start_offset, 1);
        assert_eq!(triggered_batches[0].end_offset, 3);
    }
    
    #[tokio::test]
    async fn test_window_sliding() {
        let mut triggered_batches = Vec::new();
        let config = WindowConfig {
            window_size: 3,
            window_slide: 2,
            window_timeout_ms: 10000,
            allow_incomplete_window: false,
        };
        
        let aggregator = SlidingWindowAggregator::new(config, |batch| {
            triggered_batches.push(batch);
            Ok(())
        });
        
        // 第一组数据
        let data1 = vec![
            WindowData { sequence: 1, timestamp: 1000, data: serde_json::json!({"value": 1}) },
            WindowData { sequence: 2, timestamp: 2000, data: serde_json::json!({"value": 2}) },
            WindowData { sequence: 3, timestamp: 3000, data: serde_json::json!({"value": 3}) },
        ];
        aggregator.add_data("1464", data1).await.unwrap();
        
        // 第二组数据（滑动窗口）
        let data2 = vec![
            WindowData { sequence: 4, timestamp: 4000, data: serde_json::json!({"value": 4}) },
            WindowData { sequence: 5, timestamp: 5000, data: serde_json::json!({"value": 5}) },
        ];
        aggregator.add_data("1464", data2).await.unwrap();
        
        // 应该触发两个窗口
        assert_eq!(triggered_batches.len(), 2);
    }
}

