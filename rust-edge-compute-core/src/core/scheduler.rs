//! 任务调度系统
//!
//! 提供任务队列、优先级调度和并发处理能力

use std::collections::BinaryHeap;
use std::cmp::Reverse;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, Semaphore};
use tokio::time::{timeout, Duration, sleep};
use crate::core::load_balancer::{LoadBalancer, LoadBalancerConfig};
use crate::core::types::LoadBalancingStrategy;
use crate::core::intelligent_scheduler::{IntelligentScheduler, LearningConfig, SchedulingDecision, SystemStateSnapshot, WorkerState, SchedulingResult};

// 添加fastrand依赖用于随机数生成
#[cfg(not(test))]
use fastrand;

use super::{ComputeRequest, ComputeResponse, Result};
use super::executor_registry::ExecutorRegistry;

/// 任务优先级
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

/// 调度任务
#[derive(Debug, Clone)]
pub struct ScheduledTask {
    /// 任务ID
    pub id: String,
    /// 优先级（用于排序，值越大优先级越高）
    pub priority: TaskPriority,
    /// 提交时间戳
    pub submitted_at: std::time::Instant,
    /// 计算请求
    pub request: ComputeRequest,
    /// 重试次数
    pub retry_count: u32,
    /// 最大重试次数
    pub max_retries: u32,
}

impl ScheduledTask {
    /// 创建新任务
    pub fn new(request: ComputeRequest) -> Self {
        Self {
            id: request.id.clone(),
            priority: TaskPriority::Normal,
            submitted_at: std::time::Instant::now(),
            request,
            retry_count: 0,
            max_retries: 3,
        }
    }

    /// 设置优先级
    pub fn with_priority(mut self, priority: TaskPriority) -> Self {
        self.priority = priority;
        self
    }

    /// 设置最大重试次数
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// 检查是否可以重试
    pub fn can_retry(&self) -> bool {
        self.retry_count < self.max_retries
    }

    /// 增加重试计数
    pub fn increment_retry(&mut self) {
        self.retry_count += 1;
    }

    /// 获取等待时间（用于优先级队列排序）
    pub fn wait_time(&self) -> Duration {
        self.submitted_at.elapsed()
    }
}

impl PartialEq for ScheduledTask {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for ScheduledTask {}

impl PartialOrd for ScheduledTask {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ScheduledTask {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // 首先按优先级排序（高优先级在前）
        match self.priority.cmp(&other.priority).reverse() {
            std::cmp::Ordering::Equal => {
                // 优先级相同时，按提交时间排序（早提交的在前）
                other.submitted_at.cmp(&self.submitted_at)
            }
            ordering => ordering,
        }
    }
}

/// 任务调度器
pub struct TaskScheduler {
    /// 任务队列
    task_queue: Arc<Mutex<BinaryHeap<Reverse<ScheduledTask>>>>,
    /// 工作线程信号量（控制并发数）
    worker_semaphore: Arc<Semaphore>,
    /// 任务发送器
    task_sender: mpsc::Sender<ScheduledTask>,
    /// 任务接收器
    task_receiver: Arc<Mutex<mpsc::Receiver<ScheduledTask>>>,
    /// 活动任务计数
    active_tasks: Arc<Mutex<std::collections::HashMap<String, ScheduledTask>>>,
    /// 调度器配置
    config: SchedulerConfig,
    /// 错误处理器
    error_handler: Option<Arc<super::ErrorHandler>>,
    /// Executor注册表
    executor_registry: Option<Arc<ExecutorRegistry>>,
    /// 负载均衡器
    load_balancer: Arc<LoadBalancer>,
    /// 智能调度器
    intelligent_scheduler: Arc<IntelligentScheduler>,
    /// 容器化算法执行器
    algorithm_executor: Arc<super::container::ContainerizedAlgorithmExecutor>,
    /// 工作线程性能监控任务句柄
    performance_monitor_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

/// 调度器配置
#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    /// 最大并发任务数
    pub max_concurrent_tasks: usize,
    /// 任务队列大小
    pub queue_size: usize,
    /// 任务超时时间（秒）
    pub task_timeout_seconds: u64,
    /// 默认最大重试次数
    pub default_max_retries: u32,
    /// 是否启用智能调度
    pub intelligent_scheduling_enabled: bool,
    /// 负载均衡器配置
    pub load_balancer_config: LoadBalancerConfig,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            max_concurrent_tasks: 10,
            queue_size: 1000,
            task_timeout_seconds: 300, // 5分钟
            default_max_retries: 3,
            intelligent_scheduling_enabled: false, // 默认禁用智能调度
            load_balancer_config: LoadBalancerConfig::default(),
        }
    }
}

impl TaskScheduler {
    /// 创建新的任务调度器
    pub fn new(config: SchedulerConfig) -> Self {
        let (tx, rx) = mpsc::channel(config.queue_size);

        // 根据配置决定是否启用智能调度
        let mut lb_config = config.load_balancer_config.clone();
        lb_config.intelligent_scheduling_enabled = config.intelligent_scheduling_enabled;

        let load_balancer = Arc::new(LoadBalancer::new(lb_config));
        let intelligent_scheduler = if config.intelligent_scheduling_enabled {
            Arc::new(IntelligentScheduler::new(LearningConfig::default()))
        } else {
            // 如果不启用智能调度，创建一个空的占位符
            Arc::new(IntelligentScheduler::new(LearningConfig {
                learning_rate: 0.0,
                history_window_size: 0,
                min_training_samples: usize::MAX,
                prediction_window_seconds: 0,
                model_update_interval_seconds: 0,
            }))
        };

        // 初始化容器化算法执行器 - 使用纯Youki API
        let container_manager = Arc::new(super::container::YoukiContainerManager::new(
            std::path::PathBuf::from("./runtime")
        ));
        let memory_manager = Arc::new(crate::ffi::MemoryManager::new());
        let algorithm_executor = Arc::new(super::container::ContainerizedAlgorithmExecutor::new(
            container_manager,
            memory_manager,
        ));

        Self {
            task_queue: Arc::new(Mutex::new(BinaryHeap::new())),
            worker_semaphore: Arc::new(Semaphore::new(config.max_concurrent_tasks)),
            task_sender: tx,
            task_receiver: Arc::new(Mutex::new(rx)),
            active_tasks: Arc::new(Mutex::new(std::collections::HashMap::new())),
            config,
            error_handler: None,
            load_balancer,
            intelligent_scheduler,
            algorithm_executor,
            performance_monitor_handle: Arc::new(Mutex::new(None)),
        }
    }

    /// 设置错误处理器
    pub fn with_error_handler(mut self, error_handler: Arc<super::ErrorHandler>) -> Self {
        self.error_handler = Some(error_handler);
        self
    }

    /// 提交任务到调度器
    pub async fn submit_task(&self, mut task: ScheduledTask) -> Result<String> {
        // 设置默认重试次数
        if task.max_retries == 0 {
            task.max_retries = self.config.default_max_retries;
        }

        let task_id = task.id.clone();

        // 添加到队列
        {
            let mut queue = self.task_queue.lock().await;
            queue.push(Reverse(task.clone()));
        }

        // 添加到活动任务
        {
            let mut active = self.active_tasks.lock().await;
            active.insert(task_id.clone(), task);
        }

        // 发送到处理通道
        self.task_sender.send(task).await
            .map_err(|e| format!("Failed to send task: {}", e))?;

        tracing::info!("Task {} submitted to scheduler", task_id);
        Ok(task_id)
    }

    /// 启动调度器
    pub async fn start(&self) -> Result<()> {
        tracing::info!("Starting task scheduler with {} workers", self.config.max_concurrent_tasks);

        // 注册工作线程到负载均衡器
        for worker_id in 0..self.config.max_concurrent_tasks {
            self.load_balancer.register_worker(worker_id).await;
        }

        // 启动工作线程
        for worker_id in 0..self.config.max_concurrent_tasks {
            let receiver = Arc::clone(&self.task_receiver);
            let semaphore = Arc::clone(&self.worker_semaphore);
            let active_tasks = Arc::clone(&self.active_tasks);
            let config = self.config.clone();
            let load_balancer = Arc::clone(&self.load_balancer);
            let executor_registry = self.executor_registry.clone();
            let algorithm_executor = Arc::clone(&self.algorithm_executor);
            let error_handler = self.error_handler.clone();

            tokio::spawn(async move {
                Self::worker_loop(
                    worker_id, 
                    receiver, 
                    semaphore, 
                    active_tasks, 
                    config, 
                    load_balancer,
                    executor_registry,
                    algorithm_executor,
                    error_handler,
                ).await;
            });
        }

        // 启动性能监控任务
        self.start_performance_monitor();

        // 如果启用了智能调度，才启动模型更新任务
        if self.config.intelligent_scheduling_enabled {
            self.start_model_update_task();
        }

        Ok(())
    }

    /// 启动性能监控任务
    fn start_performance_monitor(&self) {
        let load_balancer = Arc::clone(&self.load_balancer);
        let update_interval = self.config.load_balancer_config.update_interval_ms;

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(update_interval));
            let mut strategy_check_interval = tokio::time::interval(Duration::from_secs(30)); // 每30秒检查一次策略调整

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        // 定期清理和监控
                        load_balancer.cleanup_expired_workers().await;

                        let status = load_balancer.get_status().await;
                        if status.total_workers > 0 {
                            tracing::debug!(
                                "Load balancer status: {} workers, {} available, {} total requests",
                                status.total_workers,
                                status.available_workers,
                                status.stats.total_requests
                            );
                        }
                    }
                    _ = strategy_check_interval.tick() => {
                        // 动态调整调度策略
                        if let Some(new_strategy) = load_balancer.adjust_strategy_dynamically().await {
                            // 注意：这里无法直接修改配置，但可以记录建议
                            tracing::info!("Strategy adjustment recommended: {:?}", new_strategy);
                        }
                    }
                }
            }
        });

        let mut monitor_handle = self.performance_monitor_handle.lock().unwrap();
        *monitor_handle = Some(handle);
    }

    /// 启动智能调度器模型更新任务
    fn start_model_update_task(&self) {
        let intelligent_scheduler = Arc::clone(&self.intelligent_scheduler);

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(3600)); // 每小时更新一次模型

            loop {
                interval.tick().await;
                intelligent_scheduler.update_model().await;
            }
        });

        // 这里可以存储句柄以便后续管理
        tracing::info!("Started intelligent scheduler model update task");
    }

    /// 工作线程循环
    async fn worker_loop(
        worker_id: usize,
        receiver: Arc<Mutex<mpsc::Receiver<ScheduledTask>>>,
        semaphore: Arc<Semaphore>,
        active_tasks: Arc<Mutex<std::collections::HashMap<String, ScheduledTask>>>,
        config: SchedulerConfig,
        load_balancer: Arc<LoadBalancer>,
        executor_registry: Option<Arc<ExecutorRegistry>>,
        algorithm_executor: Arc<super::container::ContainerizedAlgorithmExecutor>,
        error_handler: Option<Arc<super::ErrorHandler>>,
    ) {
        tracing::info!("Worker {} started", worker_id);

        // 工作线程性能监控变量
        let mut cpu_usage = 0.0;
        let mut memory_usage = 0.0;
        let mut total_response_time = 0.0;
        let mut task_count = 0u64;

        loop {
            // 获取信号量许可
            let permit = semaphore.acquire().await.unwrap();

            // 接收任务
            let task = {
                let mut rx = receiver.lock().await;
                rx.recv().await
            };

            match task {
                Some(mut task) => {
                    let task_start = std::time::Instant::now();
                    tracing::info!("Worker {} processing task {}", worker_id, task.id);

                    // 更新负载均衡器状态（增加连接数）
                    load_balancer.release_worker(worker_id).await; // 先释放之前的连接

                    // 执行任务（带超时）
                    let timeout_duration = Duration::from_secs(config.task_timeout_seconds);
                    let executor_registry_clone = executor_registry.clone();
                    let algorithm_executor_clone = Arc::clone(&algorithm_executor);
                    let result = timeout(
                        timeout_duration, 
                        Self::execute_task(
                            task.clone(),
                            executor_registry_clone,
                            algorithm_executor_clone,
                        )
                    ).await;

                    let task_duration = task_start.elapsed().as_millis() as f64;
                    task_count += 1;
                    total_response_time = (total_response_time * (task_count - 1) as f64 + task_duration) / task_count as f64;

                    // 模拟CPU和内存使用率（实际应用中应该从系统获取）
                    cpu_usage = (cpu_usage * 0.8) + (fastrand::f64() * 0.2); // 随机波动
                    memory_usage = (memory_usage * 0.9) + (fastrand::f64() * 0.1);

                    // 更新负载均衡器状态
                    load_balancer.update_worker_status(
                        worker_id,
                        cpu_usage,
                        memory_usage,
                        task_duration,
                        true, // 假设工作线程是健康的
                    ).await;

                    match result {
                        Ok(execution_result) => {
                            match execution_result {
                                Ok(response) => {
                                    tracing::info!("Task {} completed successfully in {:.2}ms", task.id, task_duration);

                                    // 记录成功任务分配
                                    load_balancer.record_task_assignment(worker_id, true).await;

                                    // 从活动任务中移除
                                    let mut active = active_tasks.lock().await;
                                    active.remove(&task.id);
                                }
                                Err(e) => {
                                    tracing::error!("Task {} execution failed: {}", task.id, e);

                                    // 记录失败任务分配
                                    load_balancer.record_task_assignment(worker_id, false).await;

                                    let error = super::EdgeComputeError::AlgorithmExecution {
                                        message: format!("Task {} execution failed: {}", task.id, e),
                                        algorithm: Some(task.request.algorithm.clone()),
                                        input_size: Some(task.request.parameters.to_string().len()),
                                    };

                                    // 报告错误
                                    if let Some(ref error_handler) = error_handler {
                                        let strategy = error_handler.handle_error(error.clone()).await;
                                        match strategy {
                                            super::RecoveryStrategy::Retry { .. } if task.can_retry() => {
                                                task.increment_retry();
                                                tracing::info!("Retrying task {} (attempt {})", task.id, task.retry_count);

                                                // 重新提交任务
                                                let mut active = active_tasks.lock().await;
                                                active.insert(task.id.clone(), task.clone());

                                                // 这里应该有一个重试队列，但为了简化，我们直接重新执行
                                                let executor_registry_clone = executor_registry.clone();
                                                let algorithm_executor_clone = Arc::clone(&algorithm_executor);
                                                let _ = Self::execute_task(
                                                    task,
                                                    executor_registry_clone,
                                                    algorithm_executor_clone,
                                                ).await;
                                            }
                                            _ => {
                                                tracing::error!("Task {} failed permanently after {} retries",
                                                              task.id, task.retry_count);
                                                let mut active = active_tasks.lock().await;
                                                active.remove(&task.id);
                                            }
                                        }
                                    } else {
                                        // 没有错误处理器，使用默认逻辑
                                        if task.can_retry() {
                                            task.increment_retry();
                                            tracing::info!("Retrying task {} (attempt {})", task.id, task.retry_count);

                                            let mut active = active_tasks.lock().await;
                                            active.insert(task.id.clone(), task.clone());

                                            let _ = self.execute_task(task).await;
                                        } else {
                                            tracing::error!("Task {} failed permanently after {} retries",
                                                          task.id, task.retry_count);
                                            let mut active = active_tasks.lock().await;
                                            active.remove(&task.id);
                                        }
                                    }
                                }
                            }
                        }
                        Err(_) => {
                            tracing::error!("Task {} timed out after {:.2}ms", task.id, task_duration);

                            // 记录超时任务分配（算作失败）
                            load_balancer.record_task_assignment(worker_id, false).await;

                            // 超时处理逻辑
                            let mut active = active_tasks.lock().await;
                            active.remove(&task.id);
                        }
                    }
                }
                None => {
                    tracing::info!("Worker {} received shutdown signal", worker_id);
                    break;
                }
            }

            // 释放信号量许可
            drop(permit);
        }

        tracing::info!("Worker {} stopped", worker_id);
    }

    /// 执行单个任务
    async fn execute_task(
        task: ScheduledTask,
        executor_registry: Option<Arc<ExecutorRegistry>>,
        algorithm_executor: Arc<super::container::ContainerizedAlgorithmExecutor>,
    ) -> Result<ComputeResponse> {
        let start_time = std::time::Instant::now();

        // 优先使用Executor注册表
        if let Some(ref registry) = executor_registry {
            // 根据算法选择executor
            if let Some(executor) = registry.select_executor(&task.request.algorithm).await {
                tracing::info!("Using executor {} for algorithm {}", executor.name(), task.request.algorithm);
                let response = executor.execute(task.request.clone()).await?;
                let execution_time = start_time.elapsed().as_millis() as u64;
                return Ok(ComputeResponse {
                    task_id: task.id,
                    status: response.status,
                    result: response.result,
                    execution_time_ms: Some(execution_time),
                    error: response.error,
                });
            }
        }

        // 如果没有找到executor，回退到容器化算法执行器
        tracing::warn!("No executor found for algorithm {}, falling back to container executor", task.request.algorithm);
        let execution_result = algorithm_executor.execute_algorithm(task.request).await?;

        let execution_time = start_time.elapsed().as_millis() as u64;

        match execution_result.status {
            super::container::ExecutionStatus::Success => {
                Ok(ComputeResponse::success(
                    task.id,
                    execution_result.result.unwrap_or(serde_json::Value::Null),
                    execution_time,
                ))
            }
            super::container::ExecutionStatus::Timeout => {
                Ok(ComputeResponse::failure(
                    task.id,
                    "Algorithm execution timeout".to_string(),
                ))
            }
            super::container::ExecutionStatus::Failed => {
                Ok(ComputeResponse::failure(
                    task.id,
                    execution_result.error_message.unwrap_or("Algorithm execution failed".to_string()),
                ))
            }
            _ => {
                Ok(ComputeResponse::failure(
                    task.id,
                    "Algorithm execution failed with unknown status".to_string(),
                ))
            }
        }
    }

    /// 获取队列状态
    pub async fn get_queue_status(&self) -> QueueStatus {
        let queue = self.task_queue.lock().await;
        let active = self.active_tasks.lock().await;

        QueueStatus {
            queued_tasks: queue.len(),
            active_tasks: active.len(),
            max_concurrent: self.config.max_concurrent_tasks,
        }
    }

    /// 取消任务
    pub async fn cancel_task(&self, task_id: &str) -> Result<bool> {
        let mut active = self.active_tasks.lock().await;
        if active.remove(task_id).is_some() {
            tracing::info!("Task {} cancelled", task_id);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// 获取任务状态
    pub async fn get_task_status(&self, task_id: &str) -> Option<TaskStatus> {
        let active = self.active_tasks.lock().await;
        active.get(task_id).map(|task| TaskStatus {
            id: task.id.clone(),
            priority: task.priority,
            submitted_at: task.submitted_at,
            retry_count: task.retry_count,
            status: "running".to_string(),
        })
    }

    /// 启用智能调度（注意：需要在重新启动服务后生效）
    pub async fn enable_intelligent_scheduling(&self) -> Result<()> {
        if self.config.intelligent_scheduling_enabled {
            tracing::info!("Intelligent scheduling is already enabled");
            return Ok(());
        }

        tracing::info!("Intelligent scheduling enable requested - please restart service to apply");
        Ok(())
    }

    /// 禁用智能调度（注意：需要在重新启动服务后生效）
    pub async fn disable_intelligent_scheduling(&self) -> Result<()> {
        if !self.config.intelligent_scheduling_enabled {
            tracing::info!("Intelligent scheduling is already disabled");
            return Ok(());
        }

        tracing::info!("Intelligent scheduling disable requested - please restart service to apply");
        Ok(())
    }

    /// 获取智能调度状态
    pub fn get_intelligent_scheduling_status(&self) -> IntelligentSchedulingStatus {
        IntelligentSchedulingStatus {
            enabled: self.config.intelligent_scheduling_enabled,
            strategy: self.config.load_balancer_config.strategy.clone(),
            has_sufficient_data: if self.config.intelligent_scheduling_enabled {
                // 这里可以检查是否有足够的训练数据
                true // 简化处理
            } else {
                false
            },
        }
    }
}

/// 队列状态
#[derive(Debug, Clone, serde::Serialize)]
pub struct QueueStatus {
    /// 队列中任务数
    pub queued_tasks: usize,
    /// 活动任务数
    pub active_tasks: usize,
    /// 最大并发数
    pub max_concurrent: usize,
}

/// 任务状态信息
#[derive(Debug, Clone, serde::Serialize)]
pub struct TaskStatus {
    /// 任务ID
    pub id: String,
    /// 优先级
    pub priority: TaskPriority,
    /// 提交时间
    pub submitted_at: std::time::Instant,
    /// 重试次数
    pub retry_count: u32,
    /// 当前状态
    pub status: String,
}

/// 智能调度状态信息
#[derive(Debug, Clone, serde::Serialize)]
pub struct IntelligentSchedulingStatus {
    /// 是否启用智能调度
    pub enabled: bool,
    /// 当前调度策略
    pub strategy: LoadBalancingStrategy,
    /// 是否有足够的训练数据
    pub has_sufficient_data: bool,
}

impl Default for TaskScheduler {
    fn default() -> Self {
        Self::new(SchedulerConfig::default())
    }
}
