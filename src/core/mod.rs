//! 核心模块 - 包含任务调度、指标收集、错误处理等核心功能

pub mod types;
pub mod error;
pub mod scheduler;
pub mod task_spawn;
pub mod logging;
pub mod tls;
pub mod audit;
pub mod performance;
pub mod persistence;
pub mod metrics_collector;
pub mod load_balancer;
pub mod intelligent_scheduler;
pub mod allocator;

// 重新导出常用类型和函数
pub use types::{ComputeRequest, ComputeResponse, TaskStatus, ContainerConfig, LoadBalancingStrategy};
pub use scheduler::{TaskScheduler, SchedulerConfig, ScheduledTask, TaskPriority, QueueStatus};
pub use error::{EdgeComputeError, ErrorHandler, RecoveryStrategy};
pub use task_spawn::{TaskSpawner, SpawnConfig};
pub use logging::{LogManager, LogLevel};
pub use persistence::{PersistenceManager, PersistenceStore};
pub use audit::AuditLogger;
pub use performance::PerformanceMetrics;
pub use metrics_collector::{CoreMetrics, GLOBAL_METRICS, metric_names};

// 导入与关闭相关的类型（如果存在）
pub use crate::core::error::{ShutdownManager, ShutdownConfig, ShutdownHooks, ShutdownHook, ShutdownError, ShutdownSignal, SignalHandler};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
