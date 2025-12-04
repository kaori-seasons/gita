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

#[cfg(test)]
mod integration_tests;

pub use types::{TaskStatus, ResourceLimits, ContainerConfig, ComputeRequest, ComputeResponse, PerformanceThresholds, SecurityConfig};
pub use task_spawn::{TaskSpawner, SpawnConfig};
pub use error::{EdgeComputeError, Result};
pub use scheduler::{TaskScheduler, ScheduledTask, TaskPriority, QueueStatus};
pub use persistence::{PersistenceStore};
pub use shutdown::{ShutdownHandler};
pub use security::{UserSession, Permission, Role};
pub use tls::{TlsConfig, CertificateManager};
pub use encryption::{EncryptionManager, EncryptionConfig};
pub use audit::{AuditLogger, AuditEvent, AuditEventType, AuditSeverity, AuditResult, AuditConfig};
pub use metrics::{MetricsCollector, SystemMetrics, TaskMetrics};
pub use logging::{StructuredLogger, LogConfig};
pub use updates::{UpdateManager, UpdateConfig, UpdateStatus, VersionInfo};
pub use performance::{PerformanceMonitor, PerformanceConfig, LoadTestConfig, RampUpConfig};
pub use load_balancer::{LoadBalancer, LoadBalancerConfig, LoadBalancingStrategy};
pub use intelligent_scheduler::{IntelligentScheduler, LearningConfig, PredictionResult};
pub use zeromq_source::{ZeroMQDataSource, ZeroMQConfig};
pub use offset_tracker::{OffsetTracker, OffsetInfo};
pub use window_aggregator::{WindowAggregator, WindowConfig};
pub use ordered_window_processor::{OrderedWindowProcessor, ProcessingConfig};
pub use executor_registry::{ExecutorRegistry, AlgorithmExecutor};
pub use monitoring::{MonitoringManager, Metrics, HealthCheckResult};
