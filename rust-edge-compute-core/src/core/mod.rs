//! 核心模块
//!
//! 包含框架的核心类型、错误定义和共享数据结构

pub mod types;
pub mod error;
pub mod scheduler;
pub mod persistence;
pub mod shutdown;
pub mod security;
pub mod tls;
pub mod encryption;
pub mod audit;
pub mod metrics;
pub mod logging;
pub mod updates;
pub mod performance;
pub mod load_balancer;
pub mod intelligent_scheduler;
pub mod zeromq_source;
pub mod offset_tracker;
pub mod window_aggregator;
pub mod ordered_window_processor;
pub mod executor_registry;
pub mod task_spawn;
pub mod monitoring;
pub mod executor_trait;

#[cfg(test)]
mod integration_tests;

pub use types::{TaskStatus, ResourceLimits, ContainerConfig};
pub use task_spawn::{TaskSpawner, SpawnConfig};
pub use scheduler::{TaskScheduler, ScheduledTask, TaskPriority};
pub use persistence::{PersistenceStore};
pub use encryption::{EncryptionManager, EncryptionConfig};
pub use audit::{AuditLogger, AuditEvent};
pub use metrics::{MetricsCollector};
pub use updates::{UpdateManager};
pub use performance::{PerformanceConfig};
pub use load_balancer::{LoadBalancer, LoadBalancerConfig};
pub use intelligent_scheduler::{IntelligentScheduler};
pub use offset_tracker::{OffsetTracker};
pub use monitoring::{MonitoringManager, Metrics};
pub use executor_trait::{Executor, ResourceRequirements, HealthStatus};