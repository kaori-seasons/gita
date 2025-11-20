//! 位移跟踪管理器
//!
//! 维护每个测量点的最大连续消费位移，确保数据有序处理
//! 支持处理位移空洞，保证数据的连续性和有序性

use std::collections::{BTreeSet, HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use crate::core::error::Result;

/// 位移跟踪管理器
pub struct OffsetTracker {
    /// 测量点ID -> 位移状态
    offsets: Arc<RwLock<HashMap<String, OffsetState>>>,
    /// 配置
    config: OffsetTrackerConfig,
}

/// 位移跟踪配置
#[derive(Debug, Clone)]
pub struct OffsetTrackerConfig {
    /// 最大等待位移数（超过此数量仍未连续，触发告警）
    pub max_waiting_offsets: usize,
    /// 位移超时时间（毫秒）
    pub offset_timeout_ms: u64,
}

impl Default for OffsetTrackerConfig {
    fn default() -> Self {
        Self {
            max_waiting_offsets: 1000,
            offset_timeout_ms: 5000,
        }
    }
}

/// 位移状态
#[derive(Debug, Clone)]
pub struct OffsetState {
    /// 当前最大连续消费位移
    pub committed_offset: u64,
    /// 已接收但未消费的位移（可能有空洞）
    pub received_offsets: BTreeSet<u64>,
    /// 等待窗口触发的数据（按位移排序）
    pub window_buffer: VecDeque<WindowData>,
    /// 最后更新时间
    pub last_updated: std::time::Instant,
}

/// 窗口数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowData {
    /// 位移
    pub sequence: u64,
    /// 时间戳
    pub timestamp: u64,
    /// 数据
    pub data: serde_json::Value,
}

impl OffsetTracker {
    /// 创建新的位移跟踪管理器
    pub fn new(config: OffsetTrackerConfig) -> Self {
        Self {
            offsets: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }
    
    /// 接收新消息
    /// 
    /// 返回：是否有新的连续数据可以处理
    /// 如果返回的数据不为空，表示有新的连续数据可以处理
    pub async fn receive_message(
        &self,
        measurement_point_id: &str,
        sequence: u64,
        timestamp: u64,
        data: serde_json::Value,
    ) -> Result<Vec<WindowData>> {
        let mut offsets = self.offsets.write().await;
        
        // 获取或创建位移状态
        let state = offsets
            .entry(measurement_point_id.to_string())
            .or_insert_with(|| OffsetState {
                committed_offset: 0,
                received_offsets: BTreeSet::new(),
                window_buffer: VecDeque::new(),
                last_updated: std::time::Instant::now(),
            });
        
        // 检查位移是否已处理
        if sequence <= state.committed_offset {
            tracing::debug!(
                "Message with sequence {} already processed (committed: {}) for measurement point {}",
                sequence,
                state.committed_offset,
                measurement_point_id
            );
            return Ok(vec![]);
        }
        
        // 添加到接收集合
        state.received_offsets.insert(sequence);
        
        // 添加到窗口缓冲区
        state.window_buffer.push_back(WindowData {
            sequence,
            timestamp,
            data,
        });
        
        // 更新最后更新时间
        state.last_updated = std::time::Instant::now();
        
        // 检查是否有新的连续数据
        let continuous_data = self.find_continuous_data(state);
        
        // 更新已提交位移
        if let Some(&max_continuous) = continuous_data.last().map(|d| &d.sequence) {
            state.committed_offset = *max_continuous;
            
            // 清理已提交的位移
            state.received_offsets.retain(|&offset| offset > *max_continuous);
        }
        
        // 检查是否有位移空洞告警
        if state.received_offsets.len() > self.config.max_waiting_offsets {
            tracing::warn!(
                "Too many waiting offsets ({}) for measurement point {}, committed: {}",
                state.received_offsets.len(),
                measurement_point_id,
                state.committed_offset
            );
        }
        
        Ok(continuous_data)
    }
    
    /// 查找连续的数据
    /// 
    /// 从窗口缓冲区中查找从 committed_offset + 1 开始的连续数据序列
    fn find_continuous_data(&self, state: &mut OffsetState) -> Vec<WindowData> {
        let mut continuous_data = Vec::new();
        let mut expected_sequence = state.committed_offset + 1;
        
        // 按位移排序窗口缓冲区
        let mut sorted_buffer: Vec<_> = state.window_buffer.iter().cloned().collect();
        sorted_buffer.sort_by_key(|d| d.sequence);
        
        // 查找连续的数据
        for data in sorted_buffer {
            if data.sequence == expected_sequence {
                continuous_data.push(data.clone());
                expected_sequence += 1;
            } else if data.sequence > expected_sequence {
                // 发现空洞，停止查找
                tracing::debug!(
                    "Found gap in sequence: expected {}, got {}",
                    expected_sequence,
                    data.sequence
                );
                break;
            }
            // 如果 data.sequence < expected_sequence，说明是重复数据，跳过
        }
        
        // 从缓冲区中移除已连续的数据
        for data in &continuous_data {
            state.window_buffer.retain(|d| d.sequence != data.sequence);
        }
        
        continuous_data
    }
    
    /// 获取当前最大连续位移
    pub async fn get_committed_offset(&self, measurement_point_id: &str) -> u64 {
        let offsets = self.offsets.read().await;
        offsets
            .get(measurement_point_id)
            .map(|state| state.committed_offset)
            .unwrap_or(0)
    }
    
    /// 获取等待处理的位移数量
    pub async fn get_waiting_count(&self, measurement_point_id: &str) -> usize {
        let offsets = self.offsets.read().await;
        offsets
            .get(measurement_point_id)
            .map(|state| state.window_buffer.len())
            .unwrap_or(0)
    }
    
    /// 获取位移状态信息
    pub async fn get_offset_state(&self, measurement_point_id: &str) -> Option<OffsetStateInfo> {
        let offsets = self.offsets.read().await;
        offsets.get(measurement_point_id).map(|state| OffsetStateInfo {
            committed_offset: state.committed_offset,
            waiting_count: state.window_buffer.len(),
            received_offsets_count: state.received_offsets.len(),
            last_updated: state.last_updated,
        })
    }
    
    /// 清理过期的位移状态（可选，用于内存管理）
    pub async fn cleanup_expired_states(&self) {
        let mut offsets = self.offsets.write().await;
        let timeout = std::time::Duration::from_millis(self.config.offset_timeout_ms);
        
        offsets.retain(|_key, state| {
            state.last_updated.elapsed() < timeout || !state.window_buffer.is_empty()
        });
    }
    
    /// 获取所有测量点ID列表
    pub async fn get_measurement_point_ids(&self) -> Vec<String> {
        let offsets = self.offsets.read().await;
        offsets.keys().cloned().collect()
    }
    
    /// 获取活动测量点数量
    pub async fn get_active_count(&self) -> usize {
        let offsets = self.offsets.read().await;
        offsets.len()
    }
}

/// 位移状态信息（用于监控）
#[derive(Debug, Clone)]
pub struct OffsetStateInfo {
    /// 已提交位移
    pub committed_offset: u64,
    /// 等待处理的数据数量
    pub waiting_count: usize,
    /// 已接收但未连续的位移数量
    pub received_offsets_count: usize,
    /// 最后更新时间
    pub last_updated: std::time::Instant,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_offset_tracker_creation() {
        let config = OffsetTrackerConfig::default();
        let tracker = OffsetTracker::new(config);
        
        assert_eq!(tracker.get_committed_offset("test").await, 0);
        assert_eq!(tracker.get_waiting_count("test").await, 0);
    }
    
    #[tokio::test]
    async fn test_receive_continuous_messages() {
        let tracker = OffsetTracker::new(OffsetTrackerConfig::default());
        
        // 接收连续的消息
        let result1 = tracker
            .receive_message("1464", 1, 1000, serde_json::json!({"value": 1}))
            .await
            .unwrap();
        assert_eq!(result1.len(), 1);
        assert_eq!(result1[0].sequence, 1);
        
        let result2 = tracker
            .receive_message("1464", 2, 2000, serde_json::json!({"value": 2}))
            .await
            .unwrap();
        assert_eq!(result2.len(), 1);
        assert_eq!(result2[0].sequence, 2);
        
        assert_eq!(tracker.get_committed_offset("1464").await, 2);
    }
    
    #[tokio::test]
    async fn test_receive_out_of_order_messages() {
        let tracker = OffsetTracker::new(OffsetTrackerConfig::default());
        
        // 先接收序列号 3
        let result1 = tracker
            .receive_message("1464", 3, 3000, serde_json::json!({"value": 3}))
            .await
            .unwrap();
        assert_eq!(result1.len(), 0); // 没有连续数据
        
        // 再接收序列号 1
        let result2 = tracker
            .receive_message("1464", 1, 1000, serde_json::json!({"value": 1}))
            .await
            .unwrap();
        assert_eq!(result2.len(), 1); // 现在有连续数据了
        assert_eq!(result2[0].sequence, 1);
        
        // 接收序列号 2
        let result3 = tracker
            .receive_message("1464", 2, 2000, serde_json::json!({"value": 2}))
            .await
            .unwrap();
        assert_eq!(result3.len(), 2); // 现在有连续数据 2 和 3
        assert_eq!(result3[0].sequence, 2);
        assert_eq!(result3[1].sequence, 3);
        
        assert_eq!(tracker.get_committed_offset("1464").await, 3);
    }
    
    #[tokio::test]
    async fn test_duplicate_messages() {
        let tracker = OffsetTracker::new(OffsetTrackerConfig::default());
        
        // 接收消息 1
        let _ = tracker
            .receive_message("1464", 1, 1000, serde_json::json!({"value": 1}))
            .await
            .unwrap();
        
        // 再次接收消息 1（重复）
        let result = tracker
            .receive_message("1464", 1, 1000, serde_json::json!({"value": 1}))
            .await
            .unwrap();
        assert_eq!(result.len(), 0); // 已处理，不返回
        
        assert_eq!(tracker.get_committed_offset("1464").await, 1);
    }
    
    #[tokio::test]
    async fn test_multiple_measurement_points() {
        let tracker = OffsetTracker::new(OffsetTrackerConfig::default());
        
        // 不同测量点的位移是独立的
        let _ = tracker
            .receive_message("1464", 1, 1000, serde_json::json!({"value": 1}))
            .await
            .unwrap();
        
        let _ = tracker
            .receive_message("1465", 1, 1000, serde_json::json!({"value": 1}))
            .await
            .unwrap();
        
        assert_eq!(tracker.get_committed_offset("1464").await, 1);
        assert_eq!(tracker.get_committed_offset("1465").await, 1);
    }
}

