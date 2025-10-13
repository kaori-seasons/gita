//! 核心数据类型定义

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::time::Instant;

/// 计算任务请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeRequest {
    /// 任务唯一标识符
    pub id: String,
    /// 算法名称
    pub algorithm: String,
    /// 输入参数
    pub parameters: serde_json::Value,
    /// 请求超时时间（秒）
    pub timeout_seconds: Option<u64>,
}

/// 计算任务响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeResponse {
    /// 任务ID
    pub task_id: String,
    /// 执行状态
    pub status: TaskStatus,
    /// 执行结果
    pub result: Option<serde_json::Value>,
    /// 执行时间（毫秒）
    pub execution_time_ms: Option<u64>,
    /// 错误信息
    pub error: Option<String>,
}

/// 任务执行状态
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    /// 等待执行
    Pending,
    /// 正在执行
    Running,
    /// 执行成功
    Completed,
    /// 执行失败
    Failed,
    /// 任务超时
    Timeout,
    /// 任务取消
    Cancelled,
}

/// 容器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerConfig {
    /// 容器名称
    pub name: String,
    /// 镜像名称
    pub image: String,
    /// 环境变量
    pub env: Vec<String>,
    /// 挂载卷
    pub volumes: Vec<VolumeMount>,
    /// 资源限制
    pub resources: ResourceLimits,
    /// 安全配置
    pub security: SecurityConfig,
}

/// 卷挂载配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeMount {
    /// 主机路径
    pub host_path: String,
    /// 容器路径
    pub container_path: String,
    /// 只读模式
    pub read_only: bool,
}

/// 资源限制
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// CPU核心数限制
    pub cpu_cores: Option<f64>,
    /// 内存限制（MB）
    pub memory_mb: Option<u64>,
    /// 磁盘限制（MB）
    pub disk_mb: Option<u64>,
}

/// 安全配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// 无根模式
    pub rootless: bool,
    /// Seccomp配置
    pub seccomp: Option<String>,
    /// AppArmor配置
    pub apparmor: Option<String>,
    /// 网络隔离
    pub network_isolation: bool,
}

/// 算法执行上下文
#[derive(Debug)]
pub struct ExecutionContext {
    /// 任务ID
    pub task_id: String,
    /// 容器ID
    pub container_id: String,
    /// 工作目录
    pub working_dir: String,
    /// 开始时间
    pub start_time: std::time::Instant,
}

impl Default for ComputeRequest {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            algorithm: String::new(),
            parameters: serde_json::Value::Null,
            timeout_seconds: Some(300), // 5分钟默认超时
        }
    }
}

impl ComputeResponse {
    /// 创建成功的响应
    pub fn success(task_id: String, result: serde_json::Value, execution_time_ms: u64) -> Self {
        Self {
            task_id,
            status: TaskStatus::Completed,
            result: Some(result),
            execution_time_ms: Some(execution_time_ms),
            error: None,
        }
    }

    /// 创建失败的响应
    pub fn failure(task_id: String, error: String) -> Self {
        Self {
            task_id,
            status: TaskStatus::Failed,
            result: None,
            execution_time_ms: None,
            error: Some(error),
        }
    }
}

/// 负载均衡策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LoadBalancingStrategy {
    /// 轮询调度
    RoundRobin,
    /// 最少连接调度
    LeastConnections,
    /// 权重调度
    Weighted,
    /// 随机调度
    Random,
    /// 自适应调度（基于实时性能）
    Adaptive,
    /// 负载感知调度（基于负载预测）
    LoadAware,
    /// 响应时间感知调度
    ResponseTimeAware,
    /// 资源感知调度
    ResourceAware,
}

/// 动态策略调整器
#[derive(Debug, Clone)]
pub struct DynamicStrategyAdjuster {
    /// 当前系统负载水平
    pub system_load_level: f64,
    /// 策略调整历史
    pub strategy_history: Vec<(LoadBalancingStrategy, Instant)>,
    /// 性能阈值
    pub performance_thresholds: PerformanceThresholds,
    /// 最后调整时间
    pub last_adjustment: Instant,
}

/// 性能阈值
#[derive(Debug, Clone)]
pub struct PerformanceThresholds {
    /// 高负载阈值
    pub high_load_threshold: f64,
    /// 低负载阈值
    pub low_load_threshold: f64,
    /// 高响应时间阈值 (毫秒)
    pub high_response_time_threshold: f64,
    /// 低响应时间阈值 (毫秒)
    pub low_response_time_threshold: f64,
    /// 最小调整间隔 (秒)
    pub min_adjustment_interval: u64,
}

impl Default for PerformanceThresholds {
    fn default() -> Self {
        Self {
            high_load_threshold: 0.8,
            low_load_threshold: 0.3,
            high_response_time_threshold: 200.0,
            low_response_time_threshold: 50.0,
            min_adjustment_interval: 60, // 1分钟
        }
    }
}

impl Default for DynamicStrategyAdjuster {
    fn default() -> Self {
        Self {
            system_load_level: 0.0,
            strategy_history: Vec::new(),
            performance_thresholds: PerformanceThresholds::default(),
            last_adjustment: Instant::now(),
        }
    }
}

/// 工作线程信息
#[derive(Debug, Clone)]
pub struct WorkerInfo {
    /// 工作线程ID
    pub id: usize,
    /// 当前连接数（活跃任务数）
    pub current_connections: usize,
    /// 最大连接数
    pub max_connections: usize,
    /// CPU使用率 (0.0-1.0)
    pub cpu_usage: f64,
    /// 内存使用率 (0.0-1.0)
    pub memory_usage: f64,
    /// 平均响应时间 (毫秒)
    pub avg_response_time: f64,
    /// 权重 (用于权重调度)
    pub weight: usize,
    /// 是否健康
    pub is_healthy: bool,
    /// 最后更新时间
    pub last_updated: Instant,
    /// 总处理任务数
    pub total_tasks: u64,
    /// 成功处理任务数
    pub successful_tasks: u64,
}

impl WorkerInfo {
    /// 创建新的工作线程信息
    pub fn new(id: usize, max_connections: usize) -> Self {
        Self {
            id,
            current_connections: 0,
            max_connections,
            cpu_usage: 0.0,
            memory_usage: 0.0,
            avg_response_time: 0.0,
            weight: 1,
            is_healthy: true,
            last_updated: Instant::now(),
            total_tasks: 0,
            successful_tasks: 0,
        }
    }

    /// 计算工作线程负载分数 (0.0-1.0, 越高负载越重)
    pub fn load_score(&self) -> f64 {
        if !self.is_healthy {
            return 1.0; // 不健康的工作线程负载为100%
        }

        let connection_load = self.current_connections as f64 / self.max_connections as f64;
        let cpu_load = self.cpu_usage;
        let memory_load = self.memory_usage;

        // 综合负载计算 (加权平均)
        (connection_load * 0.4) + (cpu_load * 0.3) + (memory_load * 0.3)
    }

    /// 计算工作线程容量 (0.0-1.0, 越高容量越大)
    pub fn capacity_score(&self) -> f64 {
        if !self.is_healthy || self.current_connections >= self.max_connections {
            return 0.0; // 无可用容量
        }

        // 基于权重和当前负载计算容量
        let weight_factor = self.weight as f64 / 10.0; // 假设最大权重为10
        let available_capacity = 1.0 - self.load_score();

        weight_factor * available_capacity
    }

    /// 增加连接数
    pub fn increment_connections(&mut self) {
        if self.current_connections < self.max_connections {
            self.current_connections += 1;
        }
    }

    /// 减少连接数
    pub fn decrement_connections(&mut self) {
        if self.current_connections > 0 {
            self.current_connections -= 1;
        }
    }

    /// 更新性能指标
    pub fn update_metrics(&mut self, cpu_usage: f64, memory_usage: f64, response_time: f64) {
        self.cpu_usage = cpu_usage;
        self.memory_usage = memory_usage;

        // 滑动平均响应时间
        if self.total_tasks > 0 {
            self.avg_response_time = (self.avg_response_time * 0.8) + (response_time * 0.2);
        } else {
            self.avg_response_time = response_time;
        }

        self.last_updated = Instant::now();
    }

    /// 记录任务结果
    pub fn record_task_result(&mut self, success: bool) {
        self.total_tasks += 1;
        if success {
            self.successful_tasks += 1;
        }
    }

    /// 获取成功率
    pub fn success_rate(&self) -> f64 {
        if self.total_tasks == 0 {
            return 1.0;
        }
        self.successful_tasks as f64 / self.total_tasks as f64
    }

    /// 检查是否过期（超过30秒没有更新）
    pub fn is_expired(&self) -> bool {
        self.last_updated.elapsed().as_secs() > 30
    }
}
