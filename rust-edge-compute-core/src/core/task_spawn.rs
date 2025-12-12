//! 统一的任务生成包装器
//!
//! 为 tokio::spawn 提供统一的错误处理和日志记录

use std::future::Future;
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tracing::{error, warn, info, debug, trace, span, Level};

use crate::core::error::EdgeComputeError;

/// 任务执行错误
#[derive(Debug, Clone)]
pub enum TaskExecutionError {
    /// 任务被中止（Task was cancelled or panicked）
    Cancelled(String),
    /// 任务超时
    Timeout(String),
    /// 任务执行失败
    Failed(String),
}

impl std::fmt::Display for TaskExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskExecutionError::Cancelled(msg) => write!(f, "Task cancelled: {}", msg),
            TaskExecutionError::Timeout(msg) => write!(f, "Task timeout: {}", msg),
            TaskExecutionError::Failed(msg) => write!(f, "Task failed: {}", msg),
        }
    }
}

impl std::error::Error for TaskExecutionError {}

/// 任务生成配置
#[derive(Debug, Clone)]
pub struct SpawnConfig {
    /// 任务名称
    pub name: String,
    /// 是否记录成功日志
    pub log_success: bool,
    /// 成功日志级别
    pub success_log_level: Level,
    /// 是否在失败时记录详细错误信息
    pub detailed_error_logging: bool,
    /// 任务执行超时（秒），None 表示无超时
    pub timeout_secs: Option<u64>,
}

impl Default for SpawnConfig {
    fn default() -> Self {
        Self {
            name: "unnamed_task".to_string(),
            log_success: true,
            success_log_level: Level::INFO,
            detailed_error_logging: true,
            timeout_secs: None,
        }
    }
}

impl SpawnConfig {
    /// 创建新的配置
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    /// 设置是否记录成功
    pub fn with_log_success(mut self, log: bool) -> Self {
        self.log_success = log;
        self
    }

    /// 设置成功日志级别
    pub fn with_success_level(mut self, level: Level) -> Self {
        self.success_log_level = level;
        self
    }

    /// 设置详细错误日志
    pub fn with_detailed_errors(mut self, detailed: bool) -> Self {
        self.detailed_error_logging = detailed;
        self
    }

    /// 设置超时（秒）
    pub fn with_timeout(mut self, timeout_secs: u64) -> Self {
        self.timeout_secs = Some(timeout_secs);
        self
    }
}

/// 统一的任务生成包装器
pub struct TaskSpawner;

impl TaskSpawner {
    /// 生成一个任务，使用默认配置
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use rust_edge_compute::core::TaskSpawner;
    ///
    /// TaskSpawner::spawn_default(async {
    ///     println!("Hello from task!");
    ///     Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    /// });
    /// ```
    pub fn spawn_default<F>(future: F) -> JoinHandle<Result<(), TaskExecutionError>>
    where
        F: Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>> + Send + 'static,
    {
        Self::spawn_with_config(future, SpawnConfig::default())
    }

    /// 生成一个任务，使用指定配置
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use rust_edge_compute::core::{TaskSpawner, SpawnConfig};
    ///
    /// let config = SpawnConfig::new("my_task")
    ///     .with_timeout(30)
    ///     .with_log_success(true);
    ///
    /// TaskSpawner::spawn_with_config(async {
    ///     Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    /// }, config);
    /// ```
    pub fn spawn_with_config<F>(
        future: F,
        config: SpawnConfig,
    ) -> JoinHandle<Result<(), TaskExecutionError>>
    where
        F: Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>> + Send + 'static,
    {
        let span = span!(Level::TRACE, "task", name = %config.name);

        let config_clone = config.clone();
        let task_name = config.name.clone();

        tokio::spawn(async move {
            let _guard = span.enter();

            // 处理超时
            let result = if let Some(timeout_secs) = config_clone.timeout_secs {
                let duration = std::time::Duration::from_secs(timeout_secs);
                match tokio::time::timeout(duration, future).await {
                    Ok(Ok(())) => {
                        if config_clone.log_success {
                            match config_clone.success_log_level {
                                Level::ERROR => error!("Task '{}' completed successfully", task_name),
                                Level::WARN => warn!("Task '{}' completed successfully", task_name),
                                Level::INFO => info!("Task '{}' completed successfully", task_name),
                                Level::DEBUG => debug!("Task '{}' completed successfully", task_name),
                                Level::TRACE => trace!("Task '{}' completed successfully", task_name),
                            }
                        }
                        Ok(())
                    }
                    Ok(Err(e)) => {
                        let error_msg = e.to_string();
                        if config_clone.detailed_error_logging {
                            error!("Task '{}' failed: {}", task_name, error_msg);
                        } else {
                            warn!("Task '{}' failed", task_name);
                        }
                        Err(TaskExecutionError::Failed(error_msg))
                    }
                    Err(_) => {
                        let error_msg = format!(
                            "Task '{}' exceeded timeout of {}s",
                            task_name, timeout_secs
                        );
                        error!("{}", error_msg);
                        Err(TaskExecutionError::Timeout(error_msg))
                    }
                }
            } else {
                match future.await {
                    Ok(()) => {
                        if config_clone.log_success {
                            match config_clone.success_log_level {
                                Level::ERROR => error!("Task '{}' completed successfully", task_name),
                                Level::WARN => warn!("Task '{}' completed successfully", task_name),
                                Level::INFO => info!("Task '{}' completed successfully", task_name),
                                Level::DEBUG => debug!("Task '{}' completed successfully", task_name),
                                Level::TRACE => trace!("Task '{}' completed successfully", task_name),
                            }
                        }
                        Ok(())
                    }
                    Err(e) => {
                        let error_msg = e.to_string();
                        if config_clone.detailed_error_logging {
                            error!("Task '{}' failed: {}", task_name, error_msg);
                        } else {
                            warn!("Task '{}' failed", task_name);
                        }
                        Err(TaskExecutionError::Failed(error_msg))
                    }
                }
            };

            result
        })
    }

    /// 生成一个任务并等待其结果，包含详细的错误处理
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use rust_edge_compute::core::{TaskSpawner, SpawnConfig};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let config = SpawnConfig::new("my_task");
    ///     
    ///     TaskSpawner::spawn_and_wait(async {
    ///         Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    ///     }, config).await?;
    ///     
    ///     Ok(())
    /// }
    /// ```
    pub async fn spawn_and_wait<F>(
        future: F,
        config: SpawnConfig,
    ) -> Result<(), TaskExecutionError>
    where
        F: Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>> + Send + 'static,
    {
        let handle = Self::spawn_with_config(future, config);
        handle.await.map_err(|e| {
            if e.is_cancelled() {
                TaskExecutionError::Cancelled("Task was cancelled".to_string())
            } else if e.is_panic() {
                TaskExecutionError::Cancelled("Task panicked".to_string())
            } else {
                TaskExecutionError::Failed(e.to_string())
            }
        })?
    }

    /// 生成多个任务并等待所有任务完成
    ///
    /// 使用循环迭代而非 join_all 来避免引入的列表的大小调整
    /// 这种方法更高效，特别是对于大量任务时
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use rust_edge_compute::core::{TaskSpawner, SpawnConfig};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let tasks = vec![
    ///         async { Ok::<(), Box<dyn std::error::Error + Send + Sync>>(()) },
    ///         async { Ok::<(), Box<dyn std::error::Error + Send + Sync>>(()) },
    ///     ];
    ///
    ///     let config = SpawnConfig::new("batch_task");
    ///     let results = TaskSpawner::spawn_many(tasks, config).await;
    ///     
    ///     Ok(())
    /// }
    /// ```
    pub async fn spawn_many<F>(
        futures: Vec<F>,
        config: SpawnConfig,
    ) -> Vec<Result<(), TaskExecutionError>>
    where
        F: Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>> + Send + 'static,
    {
        // 预先分配足够的容量以避免动态重新调整
        let capacity = futures.len();
        let mut handles: Vec<JoinHandle<Result<(), TaskExecutionError>>> = Vec::with_capacity(capacity);

        // 使用循环迭代而非 join_all 来避免引入的列表的大小调整
        for (idx, future) in futures.into_iter().enumerate() {
            let task_config = SpawnConfig {
                name: format!("{}[{}]", config.name, idx),
                ..config.clone()
            };
            let handle = Self::spawn_with_config(future, task_config);
            // 预分配的容量确保这里不会触发 Vec 重新调整
            handles.push(handle);
        }

        // 通过直接迭代处理 handles，而不是使用 futures::future::join_all
        let mut results = Vec::with_capacity(capacity);
        for handle in handles {
            let task_result = handle.await.map_err(|e| {
                if e.is_cancelled() {
                    TaskExecutionError::Cancelled("Task was cancelled".to_string())
                } else if e.is_panic() {
                    TaskExecutionError::Cancelled("Task panicked".to_string())
                } else {
                    TaskExecutionError::Failed(e.to_string())
                }
            })?;
            results.push(task_result);
        }

        results
    }

    /// 生成多个任务并逐一处理结果
    ///
    /// 相比 spawn_many，这个版本收集不求结果，適合步骤潜在无償任务流
    /// 通过回调函数逐一处理每个任务的结果
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use rust_edge_compute::core::{TaskSpawner, SpawnConfig};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let tasks = vec![
    ///         async { Ok::<(), Box<dyn std::error::Error + Send + Sync>>(()) },
    ///         async { Ok::<(), Box<dyn std::error::Error + Send + Sync>>(()) },
    ///     ];
    ///
    ///     let config = SpawnConfig::new("stream_task");
    ///     TaskSpawner::spawn_many_streaming(tasks, config, |idx, result| {
    ///         match result {
    ///             Ok(_) => println!("Task {} completed", idx),
    ///             Err(e) => eprintln!("Task {} failed: {}", idx, e),
    ///         }
    ///     }).await;
    ///     
    ///     Ok(())
    /// }
    /// ```
    pub async fn spawn_many_streaming<F, CB>(
        futures: Vec<F>,
        config: SpawnConfig,
        mut callback: CB,
    ) where
        F: Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>> + Send + 'static,
        CB: FnMut(usize, Result<(), TaskExecutionError>),
    {
        // 预先分配足够的容量以避免 Vec 重新调整
        let capacity = futures.len();
        let mut handles: Vec<(usize, JoinHandle<Result<(), TaskExecutionError>>)> = 
            Vec::with_capacity(capacity);

        // 生成所有任务，須警死是阻塞的：多个任务宜先退出前中一个等待
        for (idx, future) in futures.into_iter().enumerate() {
            let task_config = SpawnConfig {
                name: format!("{}[{}]", config.name, idx),
                ..config.clone()
            };
            let handle = Self::spawn_with_config(future, task_config);
            handles.push((idx, handle));
        }

        // 每个任务完成后且立调用回调，逍可不需要收集了所有会候的结果
        for (idx, handle) in handles {
            match handle.await {
                Ok(result) => callback(idx, result),
                Err(e) => {
                    let err = if e.is_cancelled() {
                        TaskExecutionError::Cancelled("Task was cancelled".to_string())
                    } else if e.is_panic() {
                        TaskExecutionError::Cancelled("Task panicked".to_string())
                    } else {
                        TaskExecutionError::Failed(e.to_string())
                    };
                    callback(idx, Err(err));
                }
            }
        }
    }

    /// 生成一个任务，使用回调处理成功和失败
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use rust_edge_compute::core::{TaskSpawner, SpawnConfig};
    ///
    /// TaskSpawner::spawn_with_callback(
    ///     async {
    ///         Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    ///     },
    ///     SpawnConfig::new("my_task"),
    ///     |result| {
    ///         match result {
    ///             Ok(()) => println!("Task succeeded!"),
    ///             Err(e) => println!("Task failed: {}", e),
    ///         }
    ///     }
    /// );
    /// ```
    pub fn spawn_with_callback<F, CB>(
        future: F,
        config: SpawnConfig,
        callback: CB,
    ) -> JoinHandle<()>
    where
        F: Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>> + Send + 'static,
        CB: Fn(Result<(), TaskExecutionError>) + Send + 'static,
    {
        let handle = Self::spawn_with_config(future, config);

        tokio::spawn(async move {
            let result = handle.await.map_err(|e| {
                if e.is_cancelled() {
                    TaskExecutionError::Cancelled("Task was cancelled".to_string())
                } else if e.is_panic() {
                    TaskExecutionError::Cancelled("Task panicked".to_string())
                } else {
                    TaskExecutionError::Failed(e.to_string())
                }
            })?;
            
            callback(result);
            Ok::<(), TaskExecutionError>(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_spawn_default() {
        let handle = TaskSpawner::spawn_default(async {
            Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
        });

        let result = handle.await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_ok());
    }

    #[tokio::test]
    async fn test_spawn_with_error() {
        let handle = TaskSpawner::spawn_default(async {
            Err::<(), Box<dyn std::error::Error + Send + Sync>>(
                "Test error".into()
            )
        });

        let result = handle.await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_err());
    }

    #[tokio::test]
    async fn test_spawn_with_timeout() {
        let config = SpawnConfig::new("timeout_test").with_timeout(1);
        let handle = TaskSpawner::spawn_with_config(
            async {
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
            },
            config,
        );

        let result = handle.await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_err());
    }

    #[tokio::test]
    async fn test_spawn_and_wait() {
        let config = SpawnConfig::new("wait_test");
        let result = TaskSpawner::spawn_and_wait(
            async {
                Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
            },
            config,
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_spawn_many() {
        let tasks = vec![
            async { Ok::<(), Box<dyn std::error::Error + Send + Sync>>(()) },
            async { Ok::<(), Box<dyn std::error::Error + Send + Sync>>(()) },
            async { Ok::<(), Box<dyn std::error::Error + Send + Sync>>(()) },
        ];

        let config = SpawnConfig::new("batch_test");
        let results = TaskSpawner::spawn_many(tasks, config).await;

        assert_eq!(results.len(), 3);
        assert!(results.iter().all(|r| r.is_ok()));
    }

    #[tokio::test]
    async fn test_spawn_many_streaming() {
        use std::sync::{Arc, Mutex};

        let tasks = vec![
            async { Ok::<(), Box<dyn std::error::Error + Send + Sync>>(()) },
            async { Ok::<(), Box<dyn std::error::Error + Send + Sync>>(()) },
            async { Ok::<(), Box<dyn std::error::Error + Send + Sync>>(()) },
        ];

        let results = Arc::new(Mutex::new(Vec::new()));
        let results_clone = Arc::clone(&results);

        let config = SpawnConfig::new("stream_test");
        TaskSpawner::spawn_many_streaming(tasks, config, move |idx, result| {
            results_clone.lock().unwrap().push((idx, result.is_ok()));
        }).await;

        let final_results = results.lock().unwrap();
        assert_eq!(final_results.len(), 3);
        assert!(final_results.iter().all(|(_, ok)| *ok));
    }
}