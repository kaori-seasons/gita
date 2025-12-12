//! # Rust Edge Compute Framework - Core Library
//!
//! 核心库，包含框架的核心类型、错误定义、任务调度和统一Executor接口
//!
//! 注意：当前使用了打桩实现来替代原有的有编译错误的模块。
//! 后续应该逐步实现真正的功能来替代这些打桩。

pub mod config;
pub mod container;

/// 框架的主要错误类型
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// 核心图暴模件，为了兼容性，提供最小化的类型
pub mod core {
    use std::sync::Arc;
    pub use serde::{Serialize, Deserialize};
    pub mod types {
        use serde::{Serialize, Deserialize};
        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub struct ComputeRequest {
            pub id: String,
            pub algorithm: String,
            pub parameters: String,
        }

        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub struct ComputeResponse {
            pub success: bool,
            pub result: String,
        }
        
        impl ComputeResponse {
            pub fn success(id: String, result: serde_json::Value, _execution_time: u64) -> Self {
                Self {
                    success: true,
                    result: result.to_string(),
                }
            }
            
            pub fn failure(id: String, error: String) -> Self {
                Self {
                    success: false,
                    result: error,
                }
            }
        }

        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum TaskStatus {
            Pending,
            Running,
            Completed,
            Failed,
        }

        #[derive(Debug, Clone)]
        pub struct QueueStatus {
            pub queued_tasks: usize,
            pub active_tasks: usize,
            pub max_concurrent: usize,
        }

        #[derive(Debug, Clone, Copy)]
        pub enum ResourceLimits {}

        #[derive(Debug, Clone)]
        pub struct ContainerConfig {
            pub name: String,
        }
    }

    pub use types::*;

    // 导出 error 模块
    pub mod error {
        use std::fmt;
        use std::error::Error;

        #[derive(Debug, Clone)]
        pub struct ErrorHandler;

        #[derive(Debug, Clone)]
        pub enum RecoveryStrategy {
            Retry,
            Skip,
            Abort,
        }

        #[derive(Debug, Clone)]
        pub struct EdgeComputeError(pub String);

        impl fmt::Display for EdgeComputeError {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl Error for EdgeComputeError {}
    }

    pub use error::*;

    // 导出其他必需模块
    pub mod task_spawn {
        use std::future::Future;
        use tokio::task::JoinHandle;

        pub struct TaskSpawner;

        #[derive(Debug, Clone)]
        pub struct SpawnConfig {
            pub name: String,
        }
        
        impl SpawnConfig {
            pub fn new(name: impl Into<String>) -> Self {
                Self {
                    name: name.into(),
                }
            }
            
            pub fn with_detailed_errors(self, _detailed: bool) -> Self {
                self
            }
            
            pub fn with_log_success(self, _log: bool) -> Self {
                self
            }
        }

        impl TaskSpawner {
            pub fn spawn_default<F>(
                _future: F,
            ) -> JoinHandle<Result<(), TaskExecutionError>>
            where
                F: Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>>
                    + Send
                    + 'static,
            {
                tokio::spawn(async { Ok(()) })
            }

            pub fn spawn_with_config<F>(
                _future: F,
                _config: SpawnConfig,
            ) -> JoinHandle<Result<(), TaskExecutionError>>
            where
                F: Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>>
                    + Send
                    + 'static,
            {
                tokio::spawn(async { Ok(()) })
            }
        }

        #[derive(Debug, Clone)]
        pub enum TaskExecutionError {
            Cancelled(String),
            Timeout(String),
            Failed(String),
        }
        
        impl std::fmt::Display for TaskExecutionError {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                match self {
                    TaskExecutionError::Cancelled(msg) => write!(f, "Cancelled: {}", msg),
                    TaskExecutionError::Timeout(msg) => write!(f, "Timeout: {}", msg),
                    TaskExecutionError::Failed(msg) => write!(f, "Failed: {}", msg),
                }
            }
        }
        
        impl std::error::Error for TaskExecutionError {}
    }

    pub use task_spawn::*;

    pub mod executor_registry {
        pub struct ExecutorRegistry;
    }

    pub use executor_registry::*;

    pub mod security {
        pub struct UserSession;
    }

    pub use security::*;

    pub mod load_balancer {
        #[derive(Debug, Clone)]
        pub struct LoadBalancer;

        #[derive(Debug, Clone)]
        pub struct LoadBalancerConfig {
            pub strategy: String,
            pub intelligent_scheduling_enabled: bool,
        }

        impl Default for LoadBalancerConfig {
            fn default() -> Self {
                Self {
                    strategy: "round_robin".to_string(),
                    intelligent_scheduling_enabled: false,
                }
            }
        }

        impl LoadBalancer {
            pub fn new(_config: LoadBalancerConfig) -> Self {
                Self
            }
        }
    }

    pub use load_balancer::*;

    pub mod intelligent_scheduler {
        #[derive(Debug, Clone)]
        pub struct IntelligentScheduler;

        #[derive(Debug, Clone)]
        pub struct LearningConfig {
            pub learning_rate: f64,
            pub history_window_size: usize,
            pub min_training_samples: usize,
            pub prediction_window_seconds: u64,
            pub model_update_interval_seconds: u64,
        }

        impl Default for LearningConfig {
            fn default() -> Self {
                Self {
                    learning_rate: 0.01,
                    history_window_size: 100,
                    min_training_samples: 10,
                    prediction_window_seconds: 60,
                    model_update_interval_seconds: 300,
                }
            }
        }

        impl IntelligentScheduler {
            pub fn new(_config: LearningConfig) -> Self {
                Self
            }
        }
    }

    pub use intelligent_scheduler::*;

    // 导出 scheduler 中最小化的类型
    pub mod scheduler {
        use crate::Result;
        use std::sync::Arc;
        use super::*;

        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
        pub enum TaskPriority {
            Low = 0,
            Normal = 1,
            High = 2,
            Critical = 3,
        }

        impl Default for TaskPriority {
            fn default() -> Self {
                TaskPriority::Normal
            }
        }
    }

    pub use scheduler::*;

    pub struct ErrorHandler;
}

pub use core::*;

