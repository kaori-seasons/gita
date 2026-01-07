//! 核心模块 - 包含任务调度、指标收集、错误处理等核心功能

pub mod allocator;
pub mod audit;
pub mod error;
pub mod intelligent_scheduler;
pub mod load_balancer;
pub mod logging;
pub mod metrics_collector;
pub mod performance;
pub mod persistence;
pub mod scheduler;
pub mod task_spawn;
pub mod tls;
pub mod types;

// 重新导出常用类型和函数
pub use error::{EdgeComputeError, ErrorHandler, RecoveryStrategy};
pub use scheduler::{ScheduledTask, SchedulerConfig, TaskPriority, TaskScheduler};
pub use types::{
    ComputeRequest, ComputeResponse, ContainerConfig,
};
// Removed invalid imports: task_spawn, logging
pub use persistence::{PersistenceManager, PersistenceStore};

// 导入与关闭相关的类型（如果存在）
// pub use crate::core::error::{ShutdownManager, ShutdownConfig, ShutdownHooks, ShutdownHook, ShutdownError, ShutdownSignal, SignalHandler};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
