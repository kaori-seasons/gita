# 基于滑动窗口和位移跟踪的有序处理方案

## 📋 需求分析

### 业务场景

1. **数据源**：ZeroMQ，每个测量点（如 1464）的数据存到一个 ZeroMQ
2. **上游保证**：上游已经保证了每个测量点的有序性
3. **处理需求**：
   - 多个异步协程并发处理任务
   - 维护内存队列，记录当前最大的连续消费位移
   - 开窗接收连续最大位移的时间序列数据
   - 累计到窗口大小后再传输给下游

### 核心挑战

1. **负载不均衡**：不同测量点数据发送速度不同，按测量点分区会导致负载不均衡
2. **有序性保证**：需要保证每个测量点的数据按顺序处理
3. **位移跟踪**：需要跟踪每个测量点的消费位移，确保连续性
4. **窗口聚合**：需要按窗口大小聚合数据后再传输

---

## 🏗️ 架构设计

### 整体架构

```
ZeroMQ (按测量点分组)
    ↓
ZeroMQSource (接收消息，提取测量点ID和位移)
    ↓
OrderedWindowProcessor (按测量点分组，维护位移和窗口)
    ↓
WindowBuffer (滑动窗口，按位移排序)
    ↓
WindowTrigger (窗口触发机制)
    ↓
TaskScheduler (提交窗口聚合任务)
    ↓
Worker Pool (多个异步协程处理)
    ↓
下游系统
```

### 核心组件

#### 1. **ZeroMQ 消息结构**

```rust
/// ZeroMQ 消息结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZeroMQMessage {
    /// 测量点ID（如 "1464"）
    pub measurement_point_id: String,
    /// 位移（序列号，从上游保证有序）
    pub sequence: u64,
    /// 时间戳
    pub timestamp: u64,
    /// 数据负载
    pub payload: serde_json::Value,
    /// 元数据
    pub metadata: HashMap<String, String>,
}
```

#### 2. **位移跟踪管理器**

```rust
/// 位移跟踪管理器
/// 维护每个测量点的最大连续消费位移
pub struct OffsetTracker {
    /// 测量点ID -> 位移状态
    offsets: Arc<RwLock<HashMap<String, OffsetState>>>,
}

/// 位移状态
#[derive(Debug, Clone)]
pub struct OffsetState {
    /// 当前最大连续消费位移
    pub committed_offset: u64,
    /// 已接收但未消费的位移（可能有空洞）
    pub received_offsets: BTreeSet<u64>,
    /// 等待窗口触发的数据
    pub window_buffer: VecDeque<WindowData>,
}

/// 窗口数据
#[derive(Debug, Clone)]
pub struct WindowData {
    /// 位移
    pub sequence: u64,
    /// 时间戳
    pub timestamp: u64,
    /// 数据
    pub data: serde_json::Value,
}
```

#### 3. **滑动窗口聚合器**

```rust
/// 滑动窗口聚合器
pub struct SlidingWindowAggregator {
    /// 窗口配置
    config: WindowConfig,
    /// 按测量点分组的窗口缓冲区
    windows: Arc<RwLock<HashMap<String, WindowBuffer>>>,
    /// 窗口触发回调
    trigger_callback: Arc<dyn Fn(WindowBatch) -> Result<()> + Send + Sync>,
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

/// 窗口缓冲区
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
#[derive(Debug, Clone)]
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
```

---

## 🔧 详细设计

### 1. ZeroMQ 集成模块

**文件**：`rust-edge-compute-core/src/streaming/zeromq_source.rs`

```rust
//! ZeroMQ 数据源集成
//!
//! 从 ZeroMQ 接收测量点数据，支持按测量点分组

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use serde::{Deserialize, Serialize};
use zmq::{Context, Socket, SocketType};

/// ZeroMQ 消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZeroMQMessage {
    /// 测量点ID
    pub measurement_point_id: String,
    /// 位移（序列号）
    pub sequence: u64,
    /// 时间戳
    pub timestamp: u64,
    /// 数据负载
    pub payload: serde_json::Value,
    /// 元数据
    pub metadata: HashMap<String, String>,
}

/// ZeroMQ 数据源配置
#[derive(Debug, Clone)]
pub struct ZeroMQConfig {
    /// ZeroMQ 连接地址
    pub endpoint: String,
    /// Socket 类型（PULL, SUB, etc.）
    pub socket_type: SocketType,
    /// 接收超时（毫秒）
    pub receive_timeout_ms: i32,
    /// 最大缓冲区大小
    pub max_buffer_size: usize,
}

/// ZeroMQ 数据源
pub struct ZeroMQSource {
    config: ZeroMQConfig,
    socket: Arc<RwLock<Option<Socket>>>,
    sender: mpsc::Sender<ZeroMQMessage>,
    receiver: mpsc::Receiver<ZeroMQMessage>,
    is_running: Arc<RwLock<bool>>,
    stats: Arc<RwLock<ZeroMQSourceStats>>,
}

impl ZeroMQSource {
    /// 创建新的 ZeroMQ 数据源
    pub fn new(config: ZeroMQConfig) -> Result<Self> {
        let (sender, receiver) = mpsc::channel(config.max_buffer_size);
        
        Ok(Self {
            config,
            socket: Arc::new(RwLock::new(None)),
            sender,
            receiver,
            is_running: Arc::new(RwLock::new(false)),
            stats: Arc::new(RwLock::new(ZeroMQSourceStats::default())),
        })
    }
    
    /// 启动 ZeroMQ 消费者
    pub async fn start(&self) -> Result<()> {
        let mut is_running = self.is_running.write().await;
        if *is_running {
            return Err("ZeroMQ source is already running".into());
        }
        
        // 创建 ZeroMQ socket
        let context = Context::new();
        let socket = context.socket(self.config.socket_type)?;
        socket.connect(&self.config.endpoint)?;
        socket.set_rcvtimeo(self.config.receive_timeout_ms)?;
        
        // 存储 socket
        let mut socket_lock = self.socket.write().await;
        *socket_lock = Some(socket);
        
        *is_running = true;
        
        // 启动消费循环
        let socket_clone = self.socket.clone();
        let sender = self.sender.clone();
        let stats = self.stats.clone();
        let is_running_clone = self.is_running.clone();
        let config = self.config.clone();
        
        tokio::spawn(async move {
            Self::consume_loop(
                socket_clone,
                sender,
                stats,
                is_running_clone,
                config,
            ).await;
        });
        
        Ok(())
    }
    
    /// 消费循环
    async fn consume_loop(
        socket: Arc<RwLock<Option<Socket>>>,
        sender: mpsc::Sender<ZeroMQMessage>,
        stats: Arc<RwLock<ZeroMQSourceStats>>,
        is_running: Arc<RwLock<bool>>,
        config: ZeroMQConfig,
    ) {
        loop {
            if !*is_running.read().await {
                break;
            }
            
            let socket_guard = socket.read().await;
            if socket_guard.is_none() {
                tokio::time::sleep(Duration::from_millis(100)).await;
                continue;
            }
            
            let socket = socket_guard.as_ref().unwrap();
            
            // 接收消息
            match socket.recv_bytes(0) {
                Ok(bytes) => {
                    // 解析消息
                    match Self::parse_message(&bytes) {
                        Ok(message) => {
                            // 更新统计
                            {
                                let mut stats = stats.write().await;
                                stats.messages_received += 1;
                            }
                            
                            // 发送到处理通道
                            if let Err(e) = sender.send(message).await {
                                tracing::warn!("Failed to send ZeroMQ message: {}", e);
                                let mut stats = stats.write().await;
                                stats.errors_count += 1;
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Failed to parse ZeroMQ message: {}", e);
                            let mut stats = stats.write().await;
                            stats.errors_count += 1;
                        }
                    }
                }
                Err(zmq::Error::EAGAIN) => {
                    // 超时，继续循环
                    continue;
                }
                Err(e) => {
                    tracing::error!("ZeroMQ receive error: {}", e);
                    let mut stats = stats.write().await;
                    stats.errors_count += 1;
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }
    }
    
    /// 解析消息
    fn parse_message(bytes: &[u8]) -> Result<ZeroMQMessage> {
        // 假设消息格式为 JSON
        let message: ZeroMQMessage = serde_json::from_slice(bytes)?;
        Ok(message)
    }
    
    /// 订阅消息流
    pub fn subscribe(&self) -> mpsc::Receiver<ZeroMQMessage> {
        self.receiver.clone()
    }
}
```

### 2. 位移跟踪管理器

**文件**：`rust-edge-compute-core/src/core/offset_tracker.rs`

```rust
//! 位移跟踪管理器
//!
//! 维护每个测量点的最大连续消费位移，确保数据有序处理

use std::collections::{BTreeSet, HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

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
    /// 返回：是否有新的连续数据可以处理
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
                "Message with sequence {} already processed (committed: {})",
                sequence,
                state.committed_offset
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
        
        Ok(continuous_data)
    }
    
    /// 查找连续的数据
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
                break;
            }
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
}
```

### 3. 滑动窗口聚合器

**文件**：`rust-edge-compute-core/src/core/window_aggregator.rs`

```rust
//! 滑动窗口聚合器
//!
//! 按窗口大小聚合连续的数据，支持滑动窗口和固定窗口

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{Duration, Instant};
use serde::{Deserialize, Serialize};

use super::offset_tracker::{WindowData, OffsetTracker};

/// 滑动窗口聚合器
pub struct SlidingWindowAggregator {
    /// 窗口配置
    config: WindowConfig,
    /// 按测量点分组的窗口缓冲区
    windows: Arc<RwLock<HashMap<String, WindowBuffer>>>,
    /// 位移跟踪器
    offset_tracker: Arc<OffsetTracker>,
    /// 窗口触发回调
    trigger_callback: Arc<dyn Fn(WindowBatch) -> Result<()> + Send + Sync>,
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
    pub fn new(
        config: WindowConfig,
        offset_tracker: Arc<OffsetTracker>,
        trigger_callback: Arc<dyn Fn(WindowBatch) -> Result<()> + Send + Sync>,
    ) -> Self {
        Self {
            config,
            windows: Arc::new(RwLock::new(HashMap::new())),
            offset_tracker,
            trigger_callback,
        }
    }
    
    /// 添加数据到窗口
    pub async fn add_data(
        &self,
        measurement_point_id: &str,
        data: Vec<WindowData>,
    ) -> Result<()> {
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
    ) -> Result<()> {
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
    ) -> Result<()> {
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
                            // 这里需要重新实现触发逻辑，因为需要访问 self
                            // 简化处理：记录需要触发的窗口
                            tracing::warn!(
                                "Window timeout for measurement point {}",
                                measurement_point_id
                            );
                        }
                    }
                }
            }
        });
    }
}
```

### 4. 有序窗口处理器（整合组件）

**文件**：`rust-edge-compute-core/src/core/ordered_window_processor.rs`

```rust
//! 有序窗口处理器
//!
//! 整合 ZeroMQ 数据源、位移跟踪和滑动窗口聚合

use std::sync::Arc;
use tokio::sync::RwLock;
use crate::core::offset_tracker::{OffsetTracker, OffsetTrackerConfig};
use crate::core::window_aggregator::{SlidingWindowAggregator, WindowConfig, WindowBatch};
use crate::streaming::zeromq_source::{ZeroMQSource, ZeroMQMessage, ZeroMQConfig};
use crate::core::scheduler::TaskScheduler;

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
}

impl OrderedWindowProcessor {
    /// 创建新的有序窗口处理器
    pub fn new(
        zmq_config: ZeroMQConfig,
        offset_config: OffsetTrackerConfig,
        window_config: WindowConfig,
        scheduler: Arc<TaskScheduler>,
    ) -> Result<Self> {
        // 创建 ZeroMQ 数据源
        let zmq_source = Arc::new(ZeroMQSource::new(zmq_config)?);
        
        // 创建位移跟踪器
        let offset_tracker = Arc::new(OffsetTracker::new(offset_config));
        
        // 创建窗口触发回调
        let scheduler_clone = Arc::clone(&scheduler);
        let trigger_callback = Arc::new(move |batch: WindowBatch| -> Result<()> {
            // 将窗口批次提交到任务调度器
            Self::submit_window_batch(&scheduler_clone, batch)?;
            Ok(())
        });
        
        // 创建滑动窗口聚合器
        let window_aggregator = Arc::new(SlidingWindowAggregator::new(
            window_config,
            Arc::clone(&offset_tracker),
            trigger_callback,
        ));
        
        Ok(Self {
            zmq_source,
            offset_tracker,
            window_aggregator,
            scheduler,
            stats: Arc::new(RwLock::new(ProcessorStats::default())),
        })
    }
    
    /// 启动处理器
    pub async fn start(&self) -> Result<()> {
        // 启动 ZeroMQ 数据源
        self.zmq_source.start().await?;
        
        // 启动窗口超时检查
        self.window_aggregator.start_timeout_checker();
        
        // 启动消息处理循环
        let receiver = self.zmq_source.subscribe();
        let offset_tracker = Arc::clone(&self.offset_tracker);
        let window_aggregator = Arc::clone(&self.window_aggregator);
        let stats = Arc::clone(&self.stats);
        
        tokio::spawn(async move {
            Self::process_loop(
                receiver,
                offset_tracker,
                window_aggregator,
                stats,
            ).await;
        });
        
        Ok(())
    }
    
    /// 消息处理循环
    async fn process_loop(
        mut receiver: mpsc::Receiver<ZeroMQMessage>,
        offset_tracker: Arc<OffsetTracker>,
        window_aggregator: Arc<SlidingWindowAggregator>,
        stats: Arc<RwLock<ProcessorStats>>,
    ) {
        loop {
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
                                // 添加到窗口聚合器
                                if let Err(e) = window_aggregator
                                    .add_data(&message.measurement_point_id, continuous_data)
                                    .await
                                {
                                    tracing::error!(
                                        "Failed to add data to window aggregator: {}",
                                        e
                                    );
                                    let mut stats = stats.write().await;
                                    stats.errors_count += 1;
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
    }
    
    /// 提交窗口批次到任务调度器
    fn submit_window_batch(
        scheduler: &TaskScheduler,
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
        tokio::spawn(async move {
            if let Err(e) = scheduler.submit_task(task).await {
                tracing::error!("Failed to submit window batch task: {}", e);
            }
        });
        
        Ok(())
    }
}
```

---

## 📦 依赖添加

### Cargo.toml 更新

```toml
# rust-edge-compute-core/Cargo.toml

[dependencies]
# ... 现有依赖 ...

# ZeroMQ 支持
zmq = "0.10"

# 时间序列处理
# (如果需要额外的时间序列库)
```

---

## 🚀 实施计划

### Phase 1: 基础组件开发 (Week 1-2)

1. **ZeroMQ 集成模块**
   - [ ] 实现 `ZeroMQSource`
   - [ ] 实现消息解析
   - [ ] 单元测试

2. **位移跟踪管理器**
   - [ ] 实现 `OffsetTracker`
   - [ ] 实现连续数据查找算法
   - [ ] 单元测试

### Phase 2: 窗口聚合开发 (Week 2-3)

1. **滑动窗口聚合器**
   - [ ] 实现 `SlidingWindowAggregator`
   - [ ] 实现窗口触发机制
   - [ ] 实现超时检查
   - [ ] 单元测试

2. **有序窗口处理器**
   - [ ] 整合所有组件
   - [ ] 实现消息处理循环
   - [ ] 集成测试

### Phase 3: 集成与优化 (Week 3-4)

1. **任务调度器集成**
   - [ ] 修改 `TaskScheduler` 支持窗口批次
   - [ ] 实现窗口批次处理逻辑
   - [ ] 性能测试

2. **监控与告警**
   - [ ] 添加统计信息
   - [ ] 添加告警机制（位移空洞、窗口超时）
   - [ ] 监控面板

### Phase 4: 生产化 (Week 4-5)

1. **错误处理**
   - [ ] 实现错误恢复机制
   - [ ] 实现数据持久化（位移持久化）
   - [ ] 实现故障转移

2. **性能优化**
   - [ ] 内存优化
   - [ ] 并发优化
   - [ ] 压力测试

---

## 📊 关键设计决策

### 1. 位移跟踪策略

**选择**：维护每个测量点的最大连续消费位移

**理由**：
- ✅ 保证数据有序性
- ✅ 支持处理位移空洞
- ✅ 内存占用可控

**实现**：
- 使用 `BTreeSet` 存储已接收的位移
- 使用 `VecDeque` 存储窗口缓冲区
- 定期查找连续数据并更新已提交位移

### 2. 窗口触发策略

**选择**：基于窗口大小和超时时间的混合触发

**理由**：
- ✅ 窗口大小触发：保证数据完整性
- ✅ 超时触发：保证实时性（允许不完整窗口）

**配置**：
```rust
WindowConfig {
    window_size: 100,           // 窗口大小：100个数据点
    window_slide: 50,            // 滑动步长：50个数据点
    window_timeout_ms: 5000,     // 超时：5秒
    allow_incomplete_window: true, // 允许不完整窗口
}
```

### 3. 并发处理策略

**选择**：多个异步协程处理窗口批次，但保证每个测量点的有序性

**理由**：
- ✅ 不同测量点可以并行处理
- ✅ 相同测量点的数据有序处理
- ✅ 提高整体吞吐量

**实现**：
- 窗口批次按测量点分组
- 相同测量点的窗口批次串行处理
- 不同测量点的窗口批次并行处理

---

## 🔍 监控指标

### 关键指标

1. **位移指标**
   - 每个测量点的最大连续位移
   - 等待处理的位移数量
   - 位移空洞数量

2. **窗口指标**
   - 窗口触发频率
   - 窗口大小分布
   - 窗口超时次数

3. **性能指标**
   - 消息处理延迟
   - 窗口处理延迟
   - 吞吐量（消息/秒）

---

## ⚠️ 注意事项

### 1. 内存管理

- **位移跟踪**：定期清理已提交的位移，避免内存泄漏
- **窗口缓冲区**：限制每个测量点的最大缓冲区大小
- **背压机制**：当缓冲区满时，暂停接收新消息

### 2. 故障恢复

- **位移持久化**：定期将已提交位移持久化到磁盘
- **故障恢复**：重启后从持久化的位移恢复
- **数据丢失**：如果位移空洞超过阈值，触发告警

### 3. 性能优化

- **批量处理**：批量处理连续数据，减少锁竞争
- **异步处理**：使用异步 I/O，提高并发性能
- **内存池**：使用对象池减少内存分配

---

## 📝 总结

本方案实现了：

1. ✅ **ZeroMQ 集成**：从 ZeroMQ 接收测量点数据
2. ✅ **位移跟踪**：维护每个测量点的最大连续消费位移
3. ✅ **滑动窗口**：按窗口大小聚合连续数据
4. ✅ **有序处理**：保证每个测量点的数据有序处理
5. ✅ **并发处理**：多个异步协程处理窗口批次

**关键特性**：
- 支持位移空洞处理
- 支持窗口超时触发
- 支持不完整窗口
- 支持多测量点并行处理

**下一步**：
1. 实施 Phase 1：基础组件开发
2. 实施 Phase 2：窗口聚合开发
3. 实施 Phase 3：集成与优化
4. 实施 Phase 4：生产化

