//! 错误处理定义

use std::collections::HashMap;
use thiserror::Error;
use serde::{Deserialize, Serialize};

/// 框架错误类型
#[derive(Error, Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum EdgeComputeError {
    /// 配置错误
    #[error("Configuration error: {message}")]
    Config { message: String, source: Option<String> },

    /// 任务调度错误
    #[error("Task scheduling error: {message}")]
    TaskScheduling {
        message: String,
        task_id: Option<String>,
        queue_size: Option<usize>
    },

    /// 容器错误
    #[error("Container error: {message}")]
    Container {
        message: String,
        container_id: Option<String>,
        operation: Option<String>
    },

    /// FFI调用错误
    #[error("FFI call error: {message}")]
    FfiCall {
        message: String,
        function: Option<String>,
        language: Option<String>
    },

    /// I/O错误
    #[error("I/O error: {message}")]
    Io {
        message: String,
        operation: Option<String>,
        path: Option<String>
    },

    /// 序列化错误
    #[error("Serialization error: {message}")]
    Serialization {
        message: String,
        data_type: Option<String>
    },

    /// HTTP错误
    #[error("HTTP error: {status_code} - {message}")]
    Http {
        status_code: u16,
        message: String,
        endpoint: Option<String>
    },

    /// 超时错误
    #[error("Timeout error: {message}")]
    Timeout {
        message: String,
        duration_seconds: Option<u64>,
        operation: Option<String>
    },

    /// 算法执行错误
    #[error("Algorithm execution error: {message}")]
    AlgorithmExecution {
        message: String,
        algorithm: Option<String>,
        input_size: Option<usize>
    },

    /// 资源不足错误
    #[error("Resource exhausted: {resource}")]
    ResourceExhausted {
        resource: String,
        requested: Option<u64>,
        available: Option<u64>
    },

    /// 验证错误
    #[error("Validation error: {message}")]
    Validation {
        message: String,
        field: Option<String>,
        value: Option<String>
    },

    /// 网络错误
    #[error("Network error: {message}")]
    Network {
        message: String,
        host: Option<String>,
        port: Option<u16>
    },

    /// 认证错误
    #[error("Authentication error: {message}")]
    Authentication {
        message: String,
        user: Option<String>
    },

    /// 授权错误
    #[error("Authorization error: {message}")]
    Authorization {
        message: String,
        permission: Option<String>,
        resource: Option<String>
    },

    /// 数据库错误
    #[error("Database error: {message}")]
    Database {
        message: String,
        operation: Option<String>,
        table: Option<String>
    },

    /// 外部服务错误
    #[error("External service error: {message}")]
    ExternalService {
        message: String,
        service: Option<String>,
        status_code: Option<u16>
    },

    /// 未知错误
    #[error("Unknown error: {message}")]
    Unknown {
        message: String,
        context: Option<HashMap<String, String>>
    },
}

/// 错误严重程度
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorSeverity {
    /// 低严重程度 - 不影响核心功能
    Low,
    /// 中等严重程度 - 影响部分功能
    Medium,
    /// 高严重程度 - 影响核心功能
    High,
    /// 严重错误 - 系统无法正常工作
    Critical,
}

/// 错误统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorStats {
    /// 错误类型计数
    pub error_counts: HashMap<String, u64>,
    /// 最近错误列表
    pub recent_errors: Vec<ErrorRecord>,
    /// 错误率（过去5分钟）
    pub error_rate: f64,
    /// 最后更新时间
    pub last_updated: std::time::SystemTime,
}

/// 错误记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorRecord {
    /// 错误ID
    pub id: String,
    /// 错误类型
    pub error_type: String,
    /// 错误消息
    pub message: String,
    /// 严重程度
    pub severity: ErrorSeverity,
    /// 发生时间
    pub timestamp: std::time::SystemTime,
    /// 相关上下文
    pub context: HashMap<String, String>,
    /// 堆栈跟踪（如果可用）
    pub stack_trace: Option<String>,
}

/// 错误恢复策略
#[derive(Debug, Clone)]
pub enum RecoveryStrategy {
    /// 无恢复操作
    None,
    /// 重试操作
    Retry {
        max_attempts: u32,
        delay_ms: u64,
        backoff_multiplier: f64,
    },
    /// 降级操作
    Fallback {
        fallback_function: String,
    },
    /// 通知管理员
    Alert {
        message: String,
        channels: Vec<String>,
    },
    /// 停止服务
    Shutdown {
        reason: String,
    },
}

/// 错误处理器
#[derive(Clone)]
pub struct ErrorHandler {
    stats: std::sync::Arc<std::sync::Mutex<ErrorStats>>,
    recovery_strategies: HashMap<String, RecoveryStrategy>,
    persistence_store: Option<std::sync::Arc<super::PersistenceStore>>,
}

impl ErrorHandler {
    /// 创建新的错误处理器
    pub fn new() -> Self {
        Self {
            stats: std::sync::Arc::new(std::sync::Mutex::new(ErrorStats {
                error_counts: HashMap::new(),
                recent_errors: Vec::new(),
                error_rate: 0.0,
                last_updated: std::time::SystemTime::now(),
            })),
            recovery_strategies: Self::default_strategies(),
            persistence_store: None,
        }
    }

    /// 设置持久化存储
    pub fn with_persistence_store(mut self, store: std::sync::Arc<super::PersistenceStore>) -> Self {
        self.persistence_store = Some(store);
        self
    }

    /// 处理错误
    pub async fn handle_error(&self, error: EdgeComputeError) -> RecoveryStrategy {
        // 记录错误统计
        self.record_error(&error).await;

        // 记录错误日志
        self.log_error(&error);

        // 返回恢复策略
        self.get_recovery_strategy(&error)
    }

    /// 记录错误统计
    async fn record_error(&self, error: &EdgeComputeError) {
        let mut stats = self.stats.lock().unwrap();

        // 更新错误计数
        let error_type = self.get_error_type_name(error);
        *stats.error_counts.entry(error_type.clone()).or_insert(0) += 1;

        // 创建错误记录
        let record = ErrorRecord {
            id: uuid::Uuid::new_v4().to_string(),
            error_type: error_type.clone(),
            message: error.to_string(),
            severity: self.get_error_severity(error),
            timestamp: std::time::SystemTime::now(),
            context: self.extract_error_context(error),
            stack_trace: None, // 在生产环境中可以添加堆栈跟踪
        };

        // 持久化存储错误记录
        if let Some(ref store) = self.persistence_store {
            if let Err(e) = store.store_error_record(&record).await {
                tracing::error!("Failed to persist error record: {}", e);
            }
        }

        stats.recent_errors.push(record);

        // 保持最近错误列表的大小
        if stats.recent_errors.len() > 100 {
            stats.recent_errors.remove(0);
        }

        // 更新时间戳
        stats.last_updated = std::time::SystemTime::now();

        // 计算错误率（简化实现）
        stats.error_rate = stats.error_counts.values().sum::<u64>() as f64 / 300.0; // 过去5分钟

        // 持久化错误统计
        if let Some(ref store) = self.persistence_store {
            if let Err(e) = store.store_error_stats(&*stats).await {
                tracing::error!("Failed to persist error stats: {}", e);
            }
        }
    }

    /// 记录错误日志
    fn log_error(&self, error: &EdgeComputeError) {
        let severity = self.get_error_severity(error);

        match severity {
            ErrorSeverity::Low => tracing::debug!("Low severity error: {}", error),
            ErrorSeverity::Medium => tracing::warn!("Medium severity error: {}", error),
            ErrorSeverity::High => tracing::error!("High severity error: {}", error),
            ErrorSeverity::Critical => tracing::error!("CRITICAL ERROR: {}", error),
        }
    }

    /// 获取错误类型名称
    fn get_error_type_name(&self, error: &EdgeComputeError) -> String {
        match error {
            EdgeComputeError::Config { .. } => "Config",
            EdgeComputeError::TaskScheduling { .. } => "TaskScheduling",
            EdgeComputeError::Container { .. } => "Container",
            EdgeComputeError::FfiCall { .. } => "FfiCall",
            EdgeComputeError::Io { .. } => "Io",
            EdgeComputeError::Serialization { .. } => "Serialization",
            EdgeComputeError::Http { .. } => "Http",
            EdgeComputeError::Timeout { .. } => "Timeout",
            EdgeComputeError::AlgorithmExecution { .. } => "AlgorithmExecution",
            EdgeComputeError::ResourceExhausted { .. } => "ResourceExhausted",
            EdgeComputeError::Validation { .. } => "Validation",
            EdgeComputeError::Network { .. } => "Network",
            EdgeComputeError::Authentication { .. } => "Authentication",
            EdgeComputeError::Authorization { .. } => "Authorization",
            EdgeComputeError::Database { .. } => "Database",
            EdgeComputeError::ExternalService { .. } => "ExternalService",
            EdgeComputeError::Unknown { .. } => "Unknown",
        }.to_string()
    }

    /// 获取错误严重程度
    fn get_error_severity(&self, error: &EdgeComputeError) -> ErrorSeverity {
        match error {
            EdgeComputeError::Config { .. } => ErrorSeverity::High,
            EdgeComputeError::TaskScheduling { .. } => ErrorSeverity::Medium,
            EdgeComputeError::Container { .. } => ErrorSeverity::High,
            EdgeComputeError::FfiCall { .. } => ErrorSeverity::High,
            EdgeComputeError::Io { .. } => ErrorSeverity::Medium,
            EdgeComputeError::Serialization { .. } => ErrorSeverity::Medium,
            EdgeComputeError::Http { .. } => ErrorSeverity::Low,
            EdgeComputeError::Timeout { .. } => ErrorSeverity::Medium,
            EdgeComputeError::AlgorithmExecution { .. } => ErrorSeverity::Medium,
            EdgeComputeError::ResourceExhausted { .. } => ErrorSeverity::High,
            EdgeComputeError::Validation { .. } => ErrorSeverity::Low,
            EdgeComputeError::Network { .. } => ErrorSeverity::Medium,
            EdgeComputeError::Authentication { .. } => ErrorSeverity::High,
            EdgeComputeError::Authorization { .. } => ErrorSeverity::High,
            EdgeComputeError::Database { .. } => ErrorSeverity::Critical,
            EdgeComputeError::ExternalService { .. } => ErrorSeverity::Medium,
            EdgeComputeError::Unknown { .. } => ErrorSeverity::Medium,
        }
    }

    /// 提取错误上下文
    fn extract_error_context(&self, error: &EdgeComputeError) -> HashMap<String, String> {
        let mut context = HashMap::new();

        match error {
            EdgeComputeError::TaskScheduling { task_id, queue_size, .. } => {
                if let Some(task_id) = task_id {
                    context.insert("task_id".to_string(), task_id.clone());
                }
                if let Some(queue_size) = queue_size {
                    context.insert("queue_size".to_string(), queue_size.to_string());
                }
            }
            EdgeComputeError::Container { container_id, operation, .. } => {
                if let Some(container_id) = container_id {
                    context.insert("container_id".to_string(), container_id.clone());
                }
                if let Some(operation) = operation {
                    context.insert("operation".to_string(), operation.clone());
                }
            }
            EdgeComputeError::FfiCall { function, language, .. } => {
                if let Some(function) = function {
                    context.insert("function".to_string(), function.clone());
                }
                if let Some(language) = language {
                    context.insert("language".to_string(), language.clone());
                }
            }
            _ => {}
        }

        context
    }

    /// 获取恢复策略
    fn get_recovery_strategy(&self, error: &EdgeComputeError) -> RecoveryStrategy {
        let error_type = self.get_error_type_name(error);
        self.recovery_strategies.get(&error_type)
            .cloned()
            .unwrap_or(RecoveryStrategy::None)
    }

    /// 默认恢复策略
    fn default_strategies() -> HashMap<String, RecoveryStrategy> {
        let mut strategies = HashMap::new();

        // 网络错误 - 重试策略
        strategies.insert(
            "Network".to_string(),
            RecoveryStrategy::Retry {
                max_attempts: 3,
                delay_ms: 1000,
                backoff_multiplier: 2.0,
            }
        );

        // 超时错误 - 重试策略
        strategies.insert(
            "Timeout".to_string(),
            RecoveryStrategy::Retry {
                max_attempts: 2,
                delay_ms: 500,
                backoff_multiplier: 1.5,
            }
        );

        // 容器错误 - 通知管理员
        strategies.insert(
            "Container".to_string(),
            RecoveryStrategy::Alert {
                message: "Container operation failed".to_string(),
                channels: vec!["log".to_string(), "admin".to_string()],
            }
        );

        // 数据库错误 - 降级策略
        strategies.insert(
            "Database".to_string(),
            RecoveryStrategy::Fallback {
                fallback_function: "use_memory_cache".to_string(),
            }
        );

        strategies
    }

    /// 获取错误统计
    pub async fn get_stats(&self) -> ErrorStats {
        let mut stats = self.stats.lock().unwrap().clone();

        // 如果有持久化存储，尝试从存储中恢复数据
        if let Some(ref store) = self.persistence_store {
            if let Ok(Some(persisted_stats)) = store.load_error_stats().await {
                // 合并持久化的统计数据
                for (error_type, count) in persisted_stats.error_counts {
                    *stats.error_counts.entry(error_type).or_insert(0) += count;
                }

                // 加载最近的错误记录
                if let Ok(error_records) = store.load_error_records(50).await {
                    // 合并并排序错误记录
                    stats.recent_errors.extend(error_records);
                    stats.recent_errors.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
                    stats.recent_errors.truncate(100);
                }
            }
        }

        stats
    }

    /// 重置错误统计
    pub async fn reset_stats(&self) {
        let mut stats = self.stats.lock().unwrap();
        stats.error_counts.clear();
        stats.recent_errors.clear();
        stats.error_rate = 0.0;
        stats.last_updated = std::time::SystemTime::now();

        // 持久化重置后的统计数据
        if let Some(ref store) = self.persistence_store {
            if let Err(e) = store.store_error_stats(&*stats).await {
                tracing::error!("Failed to persist reset error stats: {}", e);
            }
        }
    }
}

/// 容器相关错误
#[derive(Error, Debug)]
pub enum ContainerError {
    #[error("Container not found: {id}")]
    NotFound { id: String },

    #[error("Container creation failed: {reason}")]
    CreationFailed { reason: String },

    #[error("Container execution failed: {reason}")]
    ExecutionFailed { reason: String },

    #[error("Container timeout")]
    Timeout,

    #[error("Container resource limit exceeded")]
    ResourceLimitExceeded,

    #[error("Container security violation: {violation}")]
    SecurityViolation { violation: String },
}

/// FFI相关错误
#[derive(Error, Debug)]
pub enum FfiError {
    #[error("Function not found: {function}")]
    FunctionNotFound { function: String },

    #[error("Parameter conversion error: {param}")]
    ParameterConversion { param: String },

    #[error("C++ exception: {message}")]
    CppException { message: String },

    #[error("Memory allocation failed")]
    MemoryAllocationFailed,

    #[error("Bridge initialization failed")]
    BridgeInitFailed,
}

/// 任务调度错误
#[derive(Error, Debug)]
pub enum TaskError {
    #[error("Task queue full")]
    QueueFull,

    #[error("Task cancelled")]
    Cancelled,

    #[error("Task dependency not satisfied: {dependency}")]
    DependencyNotSatisfied { dependency: String },

    #[error("Task priority invalid")]
    InvalidPriority,

    #[error("Task timeout")]
    Timeout,
}

impl From<ContainerError> for EdgeComputeError {
    fn from(error: ContainerError) -> Self {
        EdgeComputeError::Container {
            message: error.to_string(),
            container_id: None,
            operation: None,
        }
    }
}

impl From<FfiError> for EdgeComputeError {
    fn from(error: FfiError) -> Self {
        EdgeComputeError::FfiCall {
            message: error.to_string(),
            function: None,
            language: None,
        }
    }
}

impl From<TaskError> for EdgeComputeError {
    fn from(error: TaskError) -> Self {
        EdgeComputeError::TaskScheduling {
            message: error.to_string(),
            task_id: None,
            queue_size: None,
        }
    }
}

impl From<&str> for EdgeComputeError {
    fn from(message: &str) -> Self {
        EdgeComputeError::Unknown {
            message: message.to_string(),
            context: None,
        }
    }
}

impl From<String> for EdgeComputeError {
    fn from(message: String) -> Self {
        EdgeComputeError::Unknown {
            message,
            context: None,
        }
    }
}
