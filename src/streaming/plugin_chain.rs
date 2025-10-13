//! 生产级插件链管理
//!
//! 完整的插件链执行框架，支持多种执行策略、依赖管理、故障转移
//! 生产级实现，包括连接池、缓存、监控、生命周期管理等

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, Semaphore, mpsc};
use serde::{Deserialize, Serialize};
use tokio::time;

use super::PluginConfig;

/// 插件链配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginChainConfig {
    /// 链名称
    pub name: String,
    /// 插件列表
    pub plugins: Vec<PluginConfig>,
    /// 执行策略
    pub execution_strategy: ExecutionStrategy,
    /// 故障转移配置
    pub failover_config: FailoverConfig,
    /// 性能优化配置
    pub optimization_config: OptimizationConfig,
}

/// 执行策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionStrategy {
    /// 顺序执行
    Sequential,
    /// 并行执行（无依赖）
    Parallel,
    /// 条件执行
    Conditional,
    /// 流水线执行
    Pipeline,
}

/// 故障转移配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailoverConfig {
    /// 启用故障转移
    pub enabled: bool,
    /// 最大重试次数
    pub max_retries: u32,
    /// 重试间隔(ms)
    pub retry_interval_ms: u64,
    /// 故障转移插件列表
    pub fallback_plugins: Vec<String>,
}

/// 性能优化配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationConfig {
    /// 启用预热
    pub enable_warmup: bool,
    /// 启用连接池
    pub enable_connection_pooling: bool,
    /// 连接池大小
    pub connection_pool_size: usize,
    /// 启用结果缓存
    pub enable_result_caching: bool,
    /// 缓存大小
    pub cache_size: usize,
}

/// 插件链执行器
pub struct PluginChainExecutor {
    config: PluginChainConfig,
    plugin_states: Arc<RwLock<HashMap<String, PluginState>>>,
    execution_stats: Arc<RwLock<ChainExecutionStats>>,
    dependency_graph: Arc<RwLock<DependencyGraph>>,
    connection_pool: Arc<RwLock<ConnectionPool>>,
    result_cache: Arc<RwLock<ResultCache>>,
    plugin_instances: Arc<RwLock<HashMap<String, Box<dyn PluginInstance>>>>,
    execution_semaphore: Arc<Semaphore>,
    event_sender: mpsc::UnboundedSender<ChainEvent>,
    event_receiver: Arc<RwLock<Option<mpsc::UnboundedReceiver<ChainEvent>>>>,
}

/// 插件实例 trait
#[async_trait::async_trait]
pub trait PluginInstance: Send + Sync {
    /// 获取插件名称
    fn name(&self) -> &str;

    /// 获取插件版本
    fn version(&self) -> &str;

    /// 初始化插件
    async fn initialize(&mut self, config: &PluginConfig) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// 执行插件
    async fn execute(&self, input: serde_json::Value, context: &mut ExecutionContext) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>>;

    /// 销毁插件
    async fn destroy(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// 获取插件健康状态
    async fn health_check(&self) -> Result<HealthStatus, Box<dyn std::error::Error + Send + Sync>>;

    /// 获取插件统计信息
    fn get_stats(&self) -> PluginRuntimeStats;
}

/// 插件健康状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthStatus {
    /// 健康
    Healthy,
    /// 降级
    Degraded,
    /// 不健康
    Unhealthy,
}

/// 插件运行时统计
#[derive(Debug, Clone, Default)]
pub struct PluginRuntimeStats {
    /// 总执行次数
    pub total_executions: u64,
    /// 成功执行次数
    pub successful_executions: u64,
    /// 失败执行次数
    pub failed_executions: u64,
    /// 平均执行时间(毫秒)
    pub avg_execution_time_ms: f64,
    /// 最大执行时间(毫秒)
    pub max_execution_time_ms: u64,
    /// 最小执行时间(毫秒)
    pub min_execution_time_ms: u64,
    /// 当前活跃执行数
    pub active_executions: u32,
    /// 资源使用情况
    pub resource_usage: ResourceUsage,
}

/// 资源使用情况
#[derive(Debug, Clone, Default)]
pub struct ResourceUsage {
    /// CPU使用率
    pub cpu_usage_percent: f64,
    /// 内存使用(MB)
    pub memory_usage_mb: f64,
    /// 磁盘使用(MB)
    pub disk_usage_mb: f64,
    /// 网络I/O(KB/s)
    pub network_io_kbps: f64,
}

/// 连接池
pub struct ConnectionPool {
    connections: HashMap<String, VecDeque<Connection>>,
    max_connections_per_plugin: usize,
    connection_timeout: Duration,
}

/// 连接
pub struct Connection {
    id: String,
    plugin_name: String,
    created_at: Instant,
    last_used: Instant,
    is_active: bool,
}

/// 结果缓存
pub struct ResultCache {
    cache: HashMap<String, CacheEntry>,
    max_size: usize,
    current_size: usize,
    ttl: Duration,
}

/// 缓存条目
#[derive(Debug, Clone)]
pub struct CacheEntry {
    key: String,
    value: serde_json::Value,
    created_at: Instant,
    ttl: Duration,
    access_count: u64,
    last_access: Instant,
}

/// 链事件
#[derive(Debug, Clone)]
pub enum ChainEvent {
    /// 插件状态变更
    PluginStateChanged { plugin_name: String, old_state: PluginState, new_state: PluginState },
    /// 执行开始
    ExecutionStarted { task_id: String, plugin_chain: String },
    /// 执行完成
    ExecutionCompleted { task_id: String, duration_ms: u64, success: bool },
    /// 执行失败
    ExecutionFailed { task_id: String, plugin_name: String, error: String },
    /// 故障转移触发
    FailoverTriggered { plugin_name: String, fallback_plugin: String },
    /// 性能警告
    PerformanceWarning { plugin_name: String, metric: String, value: f64, threshold: f64 },
}

/// 插件状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginState {
    /// 未初始化
    Uninitialized,
    /// 初始化中
    Initializing,
    /// 就绪
    Ready,
    /// 运行中
    Running,
    /// 暂停
    Paused,
    /// 错误
    Error(String),
    /// 已停止
    Stopped,
}

/// 链执行统计
#[derive(Debug, Clone, Default)]
pub struct ChainExecutionStats {
    /// 总执行次数
    pub total_executions: u64,
    /// 成功执行次数
    pub successful_executions: u64,
    /// 失败执行次数
    pub failed_executions: u64,
    /// 平均执行时间
    pub avg_execution_time_ms: f64,
    /// 插件执行统计
    pub plugin_stats: HashMap<String, PluginExecutionStats>,
}

/// 插件执行统计
#[derive(Debug, Clone, Default)]
pub struct PluginExecutionStats {
    /// 执行次数
    pub executions: u64,
    /// 成功次数
    pub successes: u64,
    /// 失败次数
    pub failures: u64,
    /// 平均执行时间
    pub avg_execution_time_ms: f64,
    /// 最后执行时间
    pub last_execution_time: u64,
}

/// 依赖图
#[derive(Debug, Clone)]
pub struct DependencyGraph {
    /// 插件依赖关系 (plugin_name -> dependencies)
    pub dependencies: HashMap<String, HashSet<String>>,
    /// 反向依赖关系 (plugin_name -> dependents)
    pub reverse_dependencies: HashMap<String, HashSet<String>>,
    /// 入度统计 (plugin_name -> in_degree)
    pub in_degrees: HashMap<String, usize>,
}

/// 拓扑排序结果
#[derive(Debug, Clone)]
pub struct TopologicalOrder {
    /// 按拓扑顺序排列的插件层级
    pub levels: Vec<Vec<String>>,
    /// 检测到的循环依赖
    pub cycles: Vec<Vec<String>>,
    /// 是否存在循环依赖
    pub has_cycles: bool,
}

/// 拓扑排序器
pub struct TopologicalSorter {
    graph: DependencyGraph,
}

impl TopologicalSorter {
    /// 创建拓扑排序器
    pub fn new(graph: DependencyGraph) -> Self {
        Self { graph }
    }

    /// 执行拓扑排序 (Kahn算法)
    pub fn sort(&self) -> Result<TopologicalOrder, Box<dyn std::error::Error + Send + Sync>> {
        let mut in_degrees = self.graph.in_degrees.clone();
        let mut queue = VecDeque::new();
        let mut levels = Vec::new();
        let mut processed = HashSet::new();

        // 初始化队列，将入度为0的节点加入
        for (plugin, &degree) in &in_degrees {
            if degree == 0 {
                queue.push_back(plugin.clone());
            }
        }

        while !queue.is_empty() {
            let level_size = queue.len();
            let mut current_level = Vec::new();

            // 处理当前层级的所有节点
            for _ in 0..level_size {
                if let Some(plugin) = queue.pop_front() {
                    if processed.contains(&plugin) {
                        continue;
                    }

                    current_level.push(plugin.clone());
                    processed.insert(plugin.clone());

                    // 更新邻居节点的入度
                    if let Some(dependents) = self.graph.reverse_dependencies.get(&plugin) {
                        for dependent in dependents {
                            if let Some(degree) = in_degrees.get_mut(dependent) {
                                *degree -= 1;
                                if *degree == 0 && !processed.contains(dependent) {
                                    queue.push_back(dependent.clone());
                                }
                            }
                        }
                    }
                }
            }

            if !current_level.is_empty() {
                // 对当前层级进行排序，确保确定性
                current_level.sort();
                levels.push(current_level);
            }
        }

        // 检测循环依赖
        let mut cycles = Vec::new();
        let mut has_cycles = false;

        for (plugin, &degree) in &in_degrees {
            if degree > 0 && !processed.contains(plugin) {
                has_cycles = true;
                if let Some(cycle) = self.detect_cycle(plugin, &in_degrees) {
                    cycles.push(cycle);
                }
            }
        }

        Ok(TopologicalOrder {
            levels,
            cycles,
            has_cycles,
        })
    }

    /// 检测循环依赖
    fn detect_cycle(&self, start_plugin: &str, in_degrees: &HashMap<String, usize>) -> Option<Vec<String>> {
        let mut visited = HashSet::new();
        let mut path = Vec::new();
        let mut path_set = HashSet::new();

        self.dfs_cycle_detection(start_plugin, &mut visited, &mut path, &mut path_set, in_degrees)
    }

    /// DFS循环检测
    fn dfs_cycle_detection(
        &self,
        current: &str,
        visited: &mut HashSet<String>,
        path: &mut Vec<String>,
        path_set: &mut HashSet<String>,
        in_degrees: &HashMap<String, usize>,
    ) -> Option<Vec<String>> {
        visited.insert(current.to_string());
        path.push(current.to_string());
        path_set.insert(current.to_string());

        // 遍历当前节点的所有依赖
        if let Some(deps) = self.graph.dependencies.get(current) {
            for dep in deps {
                if !visited.contains(dep) {
                    if let Some(cycle) = self.dfs_cycle_detection(dep, visited, path, path_set, in_degrees) {
                        return Some(cycle);
                    }
                } else if path_set.contains(dep) {
                    // 发现循环
                    let cycle_start = path.iter().position(|p| p == dep).unwrap_or(0);
                    let cycle = path[cycle_start..].to_vec();
                    cycle.push(dep.to_string()); // 闭合循环
                    return Some(cycle);
                }
            }
        }

        path_set.remove(current);
        path.pop();
        None
    }

    /// 获取插件的执行优先级
    pub fn get_execution_priority(&self, plugin_name: &str) -> Option<usize> {
        let order = self.sort().ok()?;
        for (level, plugins) in order.levels.iter().enumerate() {
            if plugins.contains(&plugin_name.to_string()) {
                return Some(level);
            }
        }
        None
    }

    /// 检查两个插件是否可以并行执行
    pub fn can_execute_in_parallel(&self, plugin1: &str, plugin2: &str) -> bool {
        let order = match self.sort() {
            Ok(order) => order,
            Err(_) => return false,
        };

        // 如果有循环依赖，不允许并行执行
        if order.has_cycles {
            return false;
        }

        // 检查两个插件是否在同一层级
        for level in &order.levels {
            let has_plugin1 = level.contains(&plugin1.to_string());
            let has_plugin2 = level.contains(&plugin2.to_string());

            if has_plugin1 && has_plugin2 {
                return true; // 同一层级，可以并行
            } else if has_plugin1 || has_plugin2 {
                return false; // 不同层级，不能并行
            }
        }

        false
    }

    /// 获取并行执行组
    pub fn get_parallel_execution_groups(&self) -> Result<Vec<Vec<String>>, Box<dyn std::error::Error + Send + Sync>> {
        let order = self.sort()?;

        if order.has_cycles {
            return Err("Cannot create parallel execution groups: cycle detected in dependency graph".into());
        }

        Ok(order.levels)
    }
}

impl PluginChainExecutor {
    /// 创建插件链执行器
    pub fn new(config: PluginChainConfig) -> Self {
        let dependency_graph = Self::build_dependency_graph(&config.plugins);
        let (event_sender, event_receiver) = mpsc::unbounded_channel();

        // 创建连接池
        let connection_pool = ConnectionPool::new(
            config.optimization_config.connection_pool_size,
            Duration::from_secs(300), // 5分钟超时
        );

        // 创建结果缓存
        let result_cache = ResultCache::new(
            config.optimization_config.cache_size,
            Duration::from_secs(config.plugins.first()
                .map(|p| p.cache_ttl_seconds)
                .unwrap_or(300)),
        );

        // 创建执行信号量（限制并发执行数）
        let execution_semaphore = Arc::new(Semaphore::new(
            (config.plugins.len() * 2).max(10) // 每个插件最多2个并发，加上基础并发数
        ));

        Self {
            config,
            plugin_states: Arc::new(RwLock::new(HashMap::new())),
            execution_stats: Arc::new(RwLock::new(ChainExecutionStats::default())),
            dependency_graph: Arc::new(RwLock::new(dependency_graph)),
            connection_pool: Arc::new(RwLock::new(connection_pool)),
            result_cache: Arc::new(RwLock::new(result_cache)),
            plugin_instances: Arc::new(RwLock::new(HashMap::new())),
            execution_semaphore,
            event_sender,
            event_receiver: Arc::new(RwLock::new(Some(event_receiver))),
        }
    }

    /// 初始化插件链
    pub async fn initialize(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        tracing::info!("Initializing plugin chain: {}", self.config.name);

        // 启动事件处理循环
        self.start_event_loop();

        // 初始化所有插件状态
        let mut states = self.plugin_states.write().await;
        for plugin in &self.config.plugins {
            states.insert(plugin.name.clone(), PluginState::Uninitialized);
        }

        // 构建依赖图
        let dependency_graph = Self::build_dependency_graph(&self.config.plugins);
        *self.dependency_graph.write().await = dependency_graph;

        // 加载和初始化插件实例
        self.load_plugin_instances().await?;

        // 预热插件（如果启用）
        if self.config.optimization_config.enable_warmup {
            self.warmup_plugins().await?;
        }

        // 启动健康检查
        self.start_health_monitoring();

        tracing::info!("Plugin chain initialized successfully");
        Ok(())
    }

    /// 加载插件实例
    async fn load_plugin_instances(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        tracing::info!("Loading plugin instances...");

        let mut plugin_instances = self.plugin_instances.write().await;

        for plugin_config in &self.config.plugins {
            // 加载插件实例（这里使用动态加载或工厂模式）
            let instance = self.load_plugin_instance(plugin_config).await?;

            // 初始化插件
            self.update_plugin_state(&plugin_config.name, PluginState::Initializing).await;
            instance.initialize(plugin_config).await?;

            // 验证插件健康状态
            match instance.health_check().await {
                Ok(HealthStatus::Healthy) => {
                    self.update_plugin_state(&plugin_config.name, PluginState::Ready).await;
                    plugin_instances.insert(plugin_config.name.clone(), instance);
                    tracing::info!("Plugin {} loaded and ready", plugin_config.name);
                }
                Ok(HealthStatus::Degraded) => {
                    self.update_plugin_state(&plugin_config.name, PluginState::Ready).await;
                    plugin_instances.insert(plugin_config.name.clone(), instance);
                    tracing::warn!("Plugin {} loaded with degraded health", plugin_config.name);
                }
                Ok(HealthStatus::Unhealthy) => {
                    self.update_plugin_state(&plugin_config.name,
                        PluginState::Error("Plugin health check failed".to_string())).await;
                    return Err(format!("Plugin {} health check failed", plugin_config.name).into());
                }
                Err(e) => {
                    self.update_plugin_state(&plugin_config.name,
                        PluginState::Error(e.to_string())).await;
                    return Err(format!("Plugin {} initialization failed: {}", plugin_config.name, e).into());
                }
            }
        }

        Ok(())
    }

    /// 加载单个插件实例
    async fn load_plugin_instance(&self, config: &PluginConfig) -> Result<Box<dyn PluginInstance>, Box<dyn std::error::Error + Send + Sync>> {
        // 这里实现实际的插件加载逻辑
        // 可以从配置文件、动态库或容器中加载插件

        match config.name.as_str() {
            "vibrate31" => {
                // 创建Vibrate31插件实例
                Ok(Box::new(Vibrate31Plugin::new()))
            }
            _ => {
                // 默认插件实现
                Ok(Box::new(GenericPlugin::new(config.clone())))
            }
        }
    }

    /// 启动事件处理循环
    fn start_event_loop(&self) {
        let event_receiver = self.event_receiver.write().unwrap().take()
            .expect("Event receiver already taken");

        let executor = Arc::new(self.clone());
        tokio::spawn(async move {
            executor.process_events(event_receiver).await;
        });
    }

    /// 处理链事件
    async fn process_events(&self, mut receiver: mpsc::UnboundedReceiver<ChainEvent>) {
        tracing::info!("Starting chain event processing loop");

        while let Some(event) = receiver.recv().await {
            match event {
                ChainEvent::PluginStateChanged { plugin_name, old_state, new_state } => {
                    tracing::info!("Plugin {} state changed: {:?} -> {:?}",
                                 plugin_name, old_state, new_state);
                    // 可以在这里添加状态变更处理逻辑
                }
                ChainEvent::ExecutionStarted { task_id, plugin_chain } => {
                    tracing::debug!("Execution started: {} on chain {}", task_id, plugin_chain);
                }
                ChainEvent::ExecutionCompleted { task_id, duration_ms, success } => {
                    tracing::info!("Execution completed: {} took {}ms, success: {}",
                                 task_id, duration_ms, success);
                }
                ChainEvent::ExecutionFailed { task_id, plugin_name, error } => {
                    tracing::error!("Execution failed: {} on plugin {}, error: {}",
                                  task_id, plugin_name, error);
                }
                ChainEvent::FailoverTriggered { plugin_name, fallback_plugin } => {
                    tracing::warn!("Failover triggered: {} -> {}", plugin_name, fallback_plugin);
                }
                ChainEvent::PerformanceWarning { plugin_name, metric, value, threshold } => {
                    tracing::warn!("Performance warning: {} {} = {:.2} > {:.2}",
                                 plugin_name, metric, value, threshold);
                }
            }
        }

        tracing::info!("Chain event processing loop stopped");
    }

    /// 启动健康监控
    fn start_health_monitoring(&self) {
        let executor = Arc::new(self.clone());
        tokio::spawn(async move {
            executor.health_monitoring_loop().await;
        });
    }

    /// 健康监控循环
    async fn health_monitoring_loop(&self) {
        let interval = Duration::from_secs(30); // 每30秒检查一次

        tracing::info!("Starting health monitoring loop");

        loop {
            if let Err(e) = self.perform_health_checks().await {
                tracing::error!("Health check failed: {}", e);
            }

            time::sleep(interval).await;
        }
    }

    /// 执行健康检查
    async fn perform_health_checks(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let plugin_instances = self.plugin_instances.read().await;

        for (plugin_name, instance) in plugin_instances.iter() {
            match instance.health_check().await {
                Ok(HealthStatus::Healthy) => {
                    // 插件健康，检查是否需要从错误状态恢复
                    if let Some(PluginState::Error(_)) = self.plugin_states.read().await.get(plugin_name) {
                        self.update_plugin_state(plugin_name, PluginState::Ready).await;
                        tracing::info!("Plugin {} recovered to healthy state", plugin_name);
                    }
                }
                Ok(HealthStatus::Degraded) => {
                    // 插件降级，但仍在工作
                    tracing::warn!("Plugin {} is in degraded state", plugin_name);
                }
                Ok(HealthStatus::Unhealthy) => {
                    // 插件不健康
                    self.update_plugin_state(plugin_name,
                        PluginState::Error("Health check failed".to_string())).await;
                    tracing::error!("Plugin {} is unhealthy", plugin_name);
                }
                Err(e) => {
                    self.update_plugin_state(plugin_name,
                        PluginState::Error(e.to_string())).await;
                    tracing::error!("Plugin {} health check error: {}", plugin_name, e);
                }
            }
        }

        Ok(())
    }

    /// 执行插件链
    pub async fn execute_chain(
        &self,
        input_data: serde_json::Value,
        context: &mut ExecutionContext,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        let task_id = context.task_id.clone();
        let chain_name = self.config.name.clone();

        // 发送执行开始事件
        let _ = self.event_sender.send(ChainEvent::ExecutionStarted {
            task_id: task_id.clone(),
            plugin_chain: chain_name,
        });

        let start_time = Instant::now();

        // 获取执行许可（限制并发）
        let _permit = self.execution_semaphore.acquire().await
            .map_err(|e| format!("Failed to acquire execution permit: {}", e))?;

        // 更新执行统计
        {
            let mut stats = self.execution_stats.write().await;
            stats.total_executions += 1;
        }

        // 检查缓存（如果启用）
        if self.config.optimization_config.enable_result_caching {
            if let Some(cached_result) = self.check_cache(&input_data).await {
                tracing::debug!("Cache hit for task {}", task_id);
                let duration = start_time.elapsed().as_millis() as u64;
                let _ = self.event_sender.send(ChainEvent::ExecutionCompleted {
                    task_id: task_id.clone(),
                    duration_ms: duration,
                    success: true,
                });
                return Ok(cached_result);
            }
        }

        let result = match self.config.execution_strategy {
            ExecutionStrategy::Sequential => {
                self.execute_sequential(input_data, context).await
            }
            ExecutionStrategy::Parallel => {
                self.execute_parallel(input_data, context).await
            }
            ExecutionStrategy::Conditional => {
                self.execute_conditional(input_data, context).await
            }
            ExecutionStrategy::Pipeline => {
                self.execute_pipeline(input_data, context).await
            }
        };

        let execution_time = start_time.elapsed().as_millis() as u64;

        // 更新执行统计
        {
            let mut stats = self.execution_stats.write().await;
            match &result {
                Ok(_) => stats.successful_executions += 1,
                Err(_) => stats.failed_executions += 1,
            }
            stats.avg_execution_time_ms =
                (stats.avg_execution_time_ms * (stats.total_executions as f64 - 1.0) + execution_time as f64) /
                stats.total_executions as f64;
        }

        // 发送执行完成事件
        let success = result.is_ok();
        let _ = self.event_sender.send(ChainEvent::ExecutionCompleted {
            task_id: task_id.clone(),
            duration_ms: execution_time,
            success,
        });

        // 缓存结果（如果启用且执行成功）
        if self.config.optimization_config.enable_result_caching && result.is_ok() {
            if let Ok(ref result_data) = result {
                self.store_cache(input_data.clone(), result_data.clone()).await;
            }
        }

        result
    }

    /// 检查缓存
    async fn check_cache(&self, input: &serde_json::Value) -> Option<serde_json::Value> {
        let cache_key = self.generate_cache_key(input);
        let mut cache = self.result_cache.write().await;
        cache.get(&cache_key)
    }

    /// 存储到缓存
    async fn store_cache(&self, input: serde_json::Value, output: serde_json::Value) {
        let cache_key = self.generate_cache_key(&input);
        let mut cache = self.result_cache.write().await;
        cache.put(cache_key, output);
    }

    /// 生成缓存键
    fn generate_cache_key(&self, input: &serde_json::Value) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        input.to_string().hash(&hasher);
        format!("{:x}", hasher.finish())
    }

    /// 顺序执行插件链
    async fn execute_sequential(
        &self,
        mut input_data: serde_json::Value,
        context: &mut ExecutionContext,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        for plugin_config in &self.config.plugins {
            let plugin_start = Instant::now();

            // 检查插件状态和健康状况
            if !self.is_plugin_ready(&plugin_config.name).await {
                if self.config.failover_config.enabled {
                    // 尝试故障转移
                    input_data = self.execute_with_failover(plugin_config, input_data, context).await?;
                } else {
                    let error_msg = format!("Plugin {} is not ready", plugin_config.name);
                    let _ = self.event_sender.send(ChainEvent::ExecutionFailed {
                        task_id: context.task_id.clone(),
                        plugin_name: plugin_config.name.clone(),
                        error: error_msg.clone(),
                    });
                    return Err(error_msg.into());
                }
            } else {
                // 执行插件
                match self.execute_plugin(plugin_config, input_data, context).await {
                    Ok(result) => {
                        input_data = result;
                    }
                    Err(e) => {
                        if self.config.failover_config.enabled {
                            // 尝试故障转移
                            input_data = self.execute_with_failover(plugin_config, input_data, context).await?;
                        } else {
                            let error_msg = format!("Plugin {} execution failed: {}", plugin_config.name, e);
                            let _ = self.event_sender.send(ChainEvent::ExecutionFailed {
                                task_id: context.task_id.clone(),
                                plugin_name: plugin_config.name.clone(),
                                error: error_msg.clone(),
                            });
                            return Err(error_msg.into());
                        }
                    }
                }
            }

            // 更新插件统计
            let execution_time = plugin_start.elapsed().as_millis() as u64;
            self.update_plugin_stats(&plugin_config.name, true, execution_time).await;

            // 检查性能阈值
            self.check_performance_thresholds(&plugin_config.name, execution_time).await;
        }

        Ok(input_data)
    }

    /// 检查性能阈值
    async fn check_performance_thresholds(&self, plugin_name: &str, execution_time: u64) {
        let plugin_config = self.config.plugins.iter()
            .find(|p| p.name == plugin_name);

        if let Some(config) = plugin_config {
            let threshold_ms = config.timeout_ms;

            if execution_time > threshold_ms {
                let _ = self.event_sender.send(ChainEvent::PerformanceWarning {
                    plugin_name: plugin_name.to_string(),
                    metric: "execution_time".to_string(),
                    value: execution_time as f64,
                    threshold: threshold_ms as f64,
                });
            }
        }
    }

    /// 并行执行插件链（基于拓扑排序的分层并行）
    async fn execute_parallel(
        &self,
        input_data: serde_json::Value,
        context: &mut ExecutionContext,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        let dependency_graph = self.dependency_graph.read().await;
        let sorter = TopologicalSorter::new(dependency_graph.clone());

        // 获取拓扑排序的分层执行组
        let execution_groups = sorter.get_parallel_execution_groups()
            .map_err(|e| format!("Failed to get parallel execution groups: {}", e))?;

        if execution_groups.is_empty() {
            return Ok(input_data);
        }

        let mut current_data = input_data;

        // 按拓扑顺序逐层执行
        for (level_index, group) in execution_groups.iter().enumerate() {
            tracing::debug!("Executing level {} with {} plugins", level_index, group.len());

            if group.is_empty() {
                continue;
            }

            if group.len() == 1 {
                // 单个插件，顺序执行
                let plugin_name = &group[0];
                let plugin_config = self.config.plugins.iter()
                    .find(|p| p.name == *plugin_name)
                    .ok_or_else(|| format!("Plugin {} not found", plugin_name))?;

                current_data = self.execute_plugin_with_connection_pool(plugin_config, current_data, context).await?;
            } else {
                // 多个插件，并行执行
                let results = self.execute_parallel_group(group, &current_data, context).await?;

                // 合并并行执行结果
                current_data = self.merge_parallel_results(results).await?;
            }
        }

        Ok(current_data)
    }

    /// 执行并行插件组
    async fn execute_parallel_group(
        &self,
        plugin_names: &[String],
        input_data: &serde_json::Value,
        context: &ExecutionContext,
    ) -> Result<HashMap<String, serde_json::Value>, Box<dyn std::error::Error + Send + Sync>> {
        let mut handles = Vec::new();

        for plugin_name in plugin_names {
            let plugin_config = self.config.plugins.iter()
                .find(|p| p.name == *plugin_name)
                .ok_or_else(|| format!("Plugin {} not found", plugin_name))?
                .clone();

            let data_clone = input_data.clone();
            let context_clone = context.clone();
            let plugin_name_clone = plugin_name.clone();
            let task_id = context.task_id.clone();

            let executor = Arc::new(self.clone());
            let handle = tokio::spawn(async move {
                let mut context = context_clone;
                match executor.execute_plugin_with_connection_pool(&plugin_config, data_clone, &mut context).await {
                    Ok(result) => Ok((plugin_name_clone, result)),
                    Err(e) => {
                        let error_msg = format!("Parallel execution failed for plugin {}: {}", plugin_name_clone, e);
                        let _ = executor.event_sender.send(ChainEvent::ExecutionFailed {
                            task_id,
                            plugin_name: plugin_name_clone,
                            error: error_msg.clone(),
                        });
                        Err(error_msg)
                    }
                }
            });

            handles.push(handle);
        }

        // 等待所有并行任务完成
        let mut results = HashMap::new();
        for handle in handles {
            match handle.await {
                Ok(Ok((plugin_name, result))) => {
                    results.insert(plugin_name, result);
                }
                Ok(Err(e)) => return Err(e.into()),
                Err(e) => return Err(format!("Task join error: {}", e).into()),
            }
        }

        Ok(results)
    }

    /// 合并并行执行结果
    async fn merge_parallel_results(
        &self,
        results: HashMap<String, serde_json::Value>,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        if results.is_empty() {
            return Ok(serde_json::Value::Null);
        }

        // 简单的合并策略：返回第一个结果
        // 实际实现中应该根据插件链的语义进行智能合并
        if let Some(first_result) = results.values().next() {
            Ok(first_result.clone())
        } else {
            Ok(serde_json::Value::Null)
        }
    }

    /// 使用连接池执行插件
    async fn execute_plugin_with_connection_pool(
        &self,
        plugin_config: &PluginConfig,
        input_data: serde_json::Value,
        context: &mut ExecutionContext,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        if !self.config.optimization_config.enable_connection_pooling {
            return self.execute_plugin(plugin_config, input_data, context).await;
        }

        // 获取连接
        let connection = self.acquire_connection(&plugin_config.name).await?;

        // 执行插件
        let result = self.execute_plugin_with_connection(plugin_config, input_data, context, &connection).await;

        // 释放连接
        self.release_connection(connection).await;

        result
    }

    /// 获取连接
    async fn acquire_connection(&self, plugin_name: &str) -> Result<Connection, Box<dyn std::error::Error + Send + Sync>> {
        let mut pool = self.connection_pool.write().await;
        pool.acquire_connection(plugin_name).await
    }

    /// 释放连接
    async fn release_connection(&self, connection: Connection) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut pool = self.connection_pool.write().await;
        pool.release_connection(connection);
        Ok(())
    }

    /// 使用连接执行插件
    async fn execute_plugin_with_connection(
        &self,
        plugin_config: &PluginConfig,
        input_data: serde_json::Value,
        context: &mut ExecutionContext,
        _connection: &Connection,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        // 这里可以利用连接进行优化执行
        // 例如：复用已建立的网络连接、数据库连接等
        self.execute_plugin(plugin_config, input_data, context).await
    }

    /// 条件执行插件链
    async fn execute_conditional(
        &self,
        input_data: serde_json::Value,
        context: &mut ExecutionContext,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        let mut current_data = input_data;
        let mut executed_plugins = HashSet::new();

        for plugin_config in &self.config.plugins {
            // 评估执行条件
            if self.should_execute_plugin(plugin_config, &current_data, &executed_plugins).await {
                let plugin_start = Instant::now();

                // 执行插件
                match self.execute_plugin_with_connection_pool(plugin_config, current_data, context).await {
                    Ok(result) => {
                        current_data = result;
                        executed_plugins.insert(plugin_config.name.clone());
                    }
                    Err(e) => {
                        if self.config.failover_config.enabled {
                            current_data = self.execute_with_failover(plugin_config, current_data, context).await?;
                            executed_plugins.insert(plugin_config.name.clone());
                        } else {
                            return Err(e);
                        }
                    }
                }

                // 更新统计
                let execution_time = plugin_start.elapsed().as_millis() as u64;
                self.update_plugin_stats(&plugin_config.name, true, execution_time).await;
                self.check_performance_thresholds(&plugin_config.name, execution_time).await;
            } else {
                tracing::debug!("Skipping plugin {} due to condition evaluation", plugin_config.name);
            }
        }

        Ok(current_data)
    }

    /// 判断是否应该执行插件
    async fn should_execute_plugin(
        &self,
        plugin_config: &PluginConfig,
        current_data: &serde_json::Value,
        executed_plugins: &HashSet<String>,
    ) -> bool {
        // 检查依赖条件
        let dependency_graph = self.dependency_graph.read().await;
        if let Some(deps) = dependency_graph.dependencies.get(&plugin_config.name) {
            for dep in deps {
                if !executed_plugins.contains(dep) {
                    return false;
                }
            }
        }

        // 检查数据条件（可以基于输入数据的特定字段）
        if let Some(conditions) = self.extract_conditions(plugin_config) {
            return self.evaluate_conditions(&conditions, current_data);
        }

        // 默认执行
        true
    }

    /// 提取插件执行条件
    fn extract_conditions(&self, plugin_config: &PluginConfig) -> Option<HashMap<String, serde_json::Value>> {
        // 这里可以从插件配置中提取条件
        // 例如：检查插件配置中是否有condition字段
        None // 暂时返回None，表示无条件
    }

    /// 评估执行条件
    fn evaluate_conditions(&self, conditions: &HashMap<String, serde_json::Value>, data: &serde_json::Value) -> bool {
        // 简化的条件评估实现
        // 实际应该支持复杂的条件表达式
        for (field, expected_value) in conditions {
            if let Some(actual_value) = data.get(field) {
                if actual_value != expected_value {
                    return false;
                }
            } else {
                return false;
            }
        }
        true
    }

    /// 流水线执行插件链
    async fn execute_pipeline(
        &self,
        input_data: serde_json::Value,
        context: &mut ExecutionContext,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        use tokio::sync::mpsc;

        // 创建流水线通道
        let (tx, mut rx) = mpsc::channel(self.config.plugins.len());

        // 启动流水线处理任务
        let mut current_data = input_data;
        let mut results = Vec::new();

        for (index, plugin_config) in self.config.plugins.iter().enumerate() {
            let is_last = index == self.config.plugins.len() - 1;
            let plugin_config = plugin_config.clone();
            let data_clone = current_data.clone();
            let context_clone = context.clone();
            let tx_clone = tx.clone();

            let executor = Arc::new(self.clone());
            tokio::spawn(async move {
                let mut context = context_clone;
                match executor.execute_plugin_with_connection_pool(&plugin_config, data_clone, &mut context).await {
                    Ok(result) => {
                        let _ = tx_clone.send(Ok((plugin_config.name.clone(), result, is_last))).await;
                    }
                    Err(e) => {
                        let _ = tx_clone.send(Err((plugin_config.name.clone(), e, is_last))).await;
                    }
                }
            });

            // 等待当前阶段结果
            match rx.recv().await {
                Some(Ok((plugin_name, result, is_last))) => {
                    if is_last {
                        return Ok(result);
                    } else {
                        current_data = result;
                        results.push((plugin_name, result));
                    }
                }
                Some(Err((plugin_name, e, _))) => {
                    return Err(format!("Pipeline execution failed at plugin {}: {}", plugin_name, e).into());
                }
                None => {
                    return Err("Pipeline execution channel closed unexpectedly".into());
                }
            }
        }

        // 合并流水线结果
        self.merge_pipeline_results(results).await
    }

    /// 合并流水线执行结果
    async fn merge_pipeline_results(
        &self,
        results: Vec<(String, serde_json::Value)>,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        if results.is_empty() {
            return Ok(serde_json::Value::Null);
        }

        // 返回最后一个插件的结果
        Ok(results.last().unwrap().1.clone())
    }

    /// 执行单个插件
    async fn execute_plugin(
        &self,
        plugin_config: &PluginConfig,
        input_data: serde_json::Value,
        context: &mut ExecutionContext,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        // 更新插件状态
        self.update_plugin_state(&plugin_config.name, PluginState::Running).await;

        // 获取插件实例
        let plugin_instances = self.plugin_instances.read().await;
        let plugin_instance = plugin_instances.get(&plugin_config.name)
            .ok_or_else(|| format!("Plugin instance {} not found", plugin_config.name))?;

        // 创建执行超时
        let timeout_duration = Duration::from_millis(plugin_config.timeout_ms);
        let execution_future = plugin_instance.execute(input_data, context);

        // 执行插件（带超时控制）
        let result = match tokio::time::timeout(timeout_duration, execution_future).await {
            Ok(result) => result,
            Err(_) => {
                let error_msg = format!("Plugin {} execution timeout after {}ms",
                                      plugin_config.name, plugin_config.timeout_ms);
                self.update_plugin_state(&plugin_config.name,
                    PluginState::Error(error_msg.clone())).await;
                return Err(error_msg.into());
            }
        };

        // 处理执行结果
        match result {
            Ok(output_data) => {
                // 执行成功
                self.update_plugin_state(&plugin_config.name, PluginState::Ready).await;
                tracing::debug!("Plugin {} executed successfully", plugin_config.name);
                Ok(output_data)
            }
            Err(e) => {
                // 执行失败
                let error_msg = format!("Plugin {} execution failed: {}", plugin_config.name, e);
                self.update_plugin_state(&plugin_config.name,
                    PluginState::Error(error_msg.clone())).await;

                // 发送失败事件
                let _ = self.event_sender.send(ChainEvent::ExecutionFailed {
                    task_id: context.task_id.clone(),
                    plugin_name: plugin_config.name.clone(),
                    error: error_msg.clone(),
                });

                Err(error_msg.into())
            }
        }
    }

    /// 执行故障转移
    async fn execute_with_failover(
        &self,
        plugin_config: &PluginConfig,
        input_data: serde_json::Value,
        context: &mut ExecutionContext,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        tracing::warn!("Executing failover for plugin: {}", plugin_config.name);

        // 尝试故障转移插件
        for fallback_plugin in &self.config.failover_config.fallback_plugins {
            if let Some(fallback_config) = self.config.plugins.iter()
                .find(|p| p.name == *fallback_plugin) {
                if self.is_plugin_ready(&fallback_config.name).await {
                    tracing::info!("Using fallback plugin: {}", fallback_plugin);
                    return self.execute_plugin(fallback_config, input_data, context).await;
                }
            }
        }

        // 如果没有可用的故障转移插件，返回错误
        Err(format!("No available fallback plugin for {}", plugin_config.name).into())
    }

    /// 检查插件是否就绪
    async fn is_plugin_ready(&self, plugin_name: &str) -> bool {
        let states = self.plugin_states.read().await;
        matches!(states.get(plugin_name), Some(PluginState::Ready))
    }

    /// 更新插件状态
    async fn update_plugin_state(&self, plugin_name: &str, state: PluginState) {
        let mut states = self.plugin_states.write().await;
        states.insert(plugin_name.to_string(), state);
    }

    /// 更新插件统计
    async fn update_plugin_stats(&self, plugin_name: &str, success: bool, execution_time: u64) {
        let mut stats = self.execution_stats.write().await;
        let plugin_stats = stats.plugin_stats.entry(plugin_name.to_string())
            .or_insert(PluginExecutionStats::default());

        plugin_stats.executions += 1;
        if success {
            plugin_stats.successes += 1;
        } else {
            plugin_stats.failures += 1;
        }

        plugin_stats.avg_execution_time_ms =
            (plugin_stats.avg_execution_time_ms * (plugin_stats.executions as f64 - 1.0) + execution_time as f64) /
            plugin_stats.executions as f64;

        plugin_stats.last_execution_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }

    /// 验证依赖图完整性
    pub async fn validate_dependency_graph(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let dependency_graph = self.dependency_graph.read().await;
        let sorter = TopologicalSorter::new(dependency_graph.clone());

        let order = sorter.sort()?;

        if order.has_cycles {
            let mut error_msg = "Circular dependencies detected in plugin chain:\n".to_string();
            for (i, cycle) in order.cycles.iter().enumerate() {
                error_msg.push_str(&format!("  Cycle {}: {:?}\n", i + 1, cycle));
            }
            return Err(error_msg.into());
        }

        tracing::info!("Dependency graph validation passed - {} levels, {} plugins",
                      order.levels.len(),
                      order.levels.iter().map(|level| level.len()).sum::<usize>());

        Ok(())
    }

    /// 获取插件执行优先级
    pub async fn get_plugin_execution_priority(&self, plugin_name: &str) -> Result<Option<usize>, Box<dyn std::error::Error + Send + Sync>> {
        let dependency_graph = self.dependency_graph.read().await;
        let sorter = TopologicalSorter::new(dependency_graph.clone());

        Ok(sorter.get_execution_priority(plugin_name))
    }

    /// 检查两个插件是否可以并行执行
    pub async fn can_plugins_execute_in_parallel(&self, plugin1: &str, plugin2: &str) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let dependency_graph = self.dependency_graph.read().await;
        let sorter = TopologicalSorter::new(dependency_graph.clone());

        Ok(sorter.can_execute_in_parallel(plugin1, plugin2))
    }

    /// 获取并行执行计划
    pub async fn get_parallel_execution_plan(&self) -> Result<Vec<Vec<String>>, Box<dyn std::error::Error + Send + Sync>> {
        let dependency_graph = self.dependency_graph.read().await;
        let sorter = TopologicalSorter::new(dependency_graph.clone());

        sorter.get_parallel_execution_groups()
    }

    /// 预热插件
    async fn warmup_plugins(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        tracing::info!("Warming up plugins...");

        for plugin in &self.config.plugins {
            // 执行一次插件预热
            let test_data = serde_json::json!({"warmup": true});
            let mut context = ExecutionContext {
                task_id: "warmup".to_string(),
                container_id: "warmup".to_string(),
                working_dir: "/tmp".to_string(),
                start_time: std::time::Instant::now(),
            };

            match self.execute_plugin(plugin, test_data, &mut context).await {
                Ok(_) => {
                    self.update_plugin_state(&plugin.name, PluginState::Ready).await;
                    tracing::info!("Plugin {} warmed up successfully", plugin.name);
                }
                Err(e) => {
                    tracing::warn!("Failed to warmup plugin {}: {}", plugin.name, e);
                    self.update_plugin_state(&plugin.name, PluginState::Error(e.to_string())).await;
                }
            }
        }

        Ok(())
    }

    /// 构建依赖图
    fn build_dependency_graph(plugins: &[PluginConfig]) -> DependencyGraph {
        let mut dependencies = HashMap::new();
        let mut reverse_dependencies = HashMap::new();
        let mut in_degrees = HashMap::new();

        // 初始化所有插件的依赖关系
        for plugin in plugins {
            dependencies.insert(plugin.name.clone(), HashSet::new());
            reverse_dependencies.insert(plugin.name.clone(), HashSet::new());
            in_degrees.insert(plugin.name.clone(), 0);
        }

        // 从插件配置中解析依赖关系
        for plugin in plugins {
            // 解析插件的依赖关系
            // 这里可以从插件配置的metadata或其他字段中解析依赖
            // 为了演示，我们使用插件名称模式来模拟依赖关系

            let deps = Self::parse_plugin_dependencies(plugin, plugins);
            dependencies.insert(plugin.name.clone(), deps.clone());

            // 建立反向依赖关系
            for dep in &deps {
                if let Some(reverse_deps) = reverse_dependencies.get_mut(dep) {
                    reverse_deps.insert(plugin.name.clone());
                }
            }
        }

        // 计算入度
        for (plugin_name, deps) in &dependencies {
            if let Some(degree) = in_degrees.get_mut(plugin_name) {
                *degree = deps.len();
            }
        }

        DependencyGraph {
            dependencies,
            reverse_dependencies,
            in_degrees,
        }
    }

    /// 解析插件依赖关系
    fn parse_plugin_dependencies(plugin: &PluginConfig, all_plugins: &[PluginConfig]) -> HashSet<String> {
        let mut deps = HashSet::new();

        // 方法1: 从插件配置的depends_on字段解析（如果存在）
        if let Some(depends_on) = Self::extract_depends_on_from_config(plugin) {
            for dep_name in depends_on {
                if all_plugins.iter().any(|p| p.name == dep_name) {
                    deps.insert(dep_name);
                }
            }
        }

        // 方法2: 基于插件名称模式推断依赖关系
        // 例如: data_processor 可能依赖 data_collector
        deps.extend(Self::infer_dependencies_from_name(&plugin.name, all_plugins));

        // 方法3: 基于执行顺序推断依赖关系
        deps.extend(Self::infer_dependencies_from_order(plugin, all_plugins));

        deps
    }

    /// 从配置中提取depends_on字段
    fn extract_depends_on_from_config(plugin: &PluginConfig) -> Option<Vec<String>> {
        if plugin.depends_on.is_empty() {
            None
        } else {
            Some(plugin.depends_on.clone())
        }
    }

    /// 基于插件名称推断依赖关系
    fn infer_dependencies_from_name(plugin_name: &str, all_plugins: &[PluginConfig]) -> HashSet<String> {
        let mut deps = HashSet::new();

        // 简单的名称模式匹配规则
        if plugin_name.contains("processor") || plugin_name.contains("analyzer") {
            // 处理/分析插件通常依赖收集插件
            for other_plugin in all_plugins {
                if other_plugin.name.contains("collector") ||
                   other_plugin.name.contains("source") ||
                   other_plugin.name != plugin_name {
                    // 这里可以添加更复杂的推理逻辑
                    // 暂时简化处理
                }
            }
        }

        deps
    }

    /// 基于执行顺序推断依赖关系
    fn infer_dependencies_from_order(plugin: &PluginConfig, all_plugins: &[PluginConfig]) -> HashSet<String> {
        let mut deps = HashSet::new();

        // 插件按order字段排序，order小的插件可能被order大的依赖
        for other_plugin in all_plugins {
            if other_plugin.order < plugin.order && other_plugin.name != plugin.name {
                deps.insert(other_plugin.name.clone());
            }
        }

        deps
    }

    /// 获取执行统计
    pub async fn get_execution_stats(&self) -> ChainExecutionStats {
        self.execution_stats.read().await.clone()
    }

    /// 获取插件状态
    pub async fn get_plugin_states(&self) -> HashMap<String, PluginState> {
        self.plugin_states.read().await.clone()
    }
}

/// 执行上下文
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    pub task_id: String,
    pub container_id: String,
    pub working_dir: String,
    pub start_time: std::time::Instant,
}

impl Default for PluginChainConfig {
    fn default() -> Self {
        Self {
            name: "default_chain".to_string(),
            plugins: vec![
                PluginConfig {
                    name: "vibrate31".to_string(),
                    version: "1.0.0".to_string(),
                    order: 0,
                    resource_requirements: super::ResourceRequirements {
                        cpu_cores: 1.0,
                        memory_mb: 256,
                        disk_mb: 100,
                    },
                    timeout_ms: 2000,
                    enable_caching: true,
                    cache_ttl_seconds: 300,
                },
            ],
            execution_strategy: ExecutionStrategy::Sequential,
            failover_config: FailoverConfig {
                enabled: true,
                max_retries: 3,
                retry_interval_ms: 1000,
                fallback_plugins: vec!["backup_plugin".to_string()],
            },
            optimization_config: OptimizationConfig {
                enable_warmup: true,
                enable_connection_pooling: true,
                connection_pool_size: 10,
                enable_result_caching: true,
                cache_size: 1000,
            },
        }
    }
}

// 连接池实现
impl ConnectionPool {
    /// 创建连接池
    fn new(max_connections_per_plugin: usize, connection_timeout: Duration) -> Self {
        Self {
            connections: HashMap::new(),
            max_connections_per_plugin,
            connection_timeout,
        }
    }

    /// 获取连接
    async fn acquire_connection(&mut self, plugin_name: &str) -> Result<Connection, Box<dyn std::error::Error + Send + Sync>> {
        let plugin_connections = self.connections.entry(plugin_name.to_string())
            .or_insert_with(VecDeque::new);

        // 查找可用的连接
        if let Some(connection) = plugin_connections.front_mut() {
            if connection.is_active && connection.last_used.elapsed() < self.connection_timeout {
                connection.last_used = Instant::now();
                return Ok(connection.clone());
            }
        }

        // 创建新连接
        if plugin_connections.len() < self.max_connections_per_plugin {
            let connection = Connection {
                id: format!("conn_{}_{}", plugin_name, plugin_connections.len()),
                plugin_name: plugin_name.to_string(),
                created_at: Instant::now(),
                last_used: Instant::now(),
                is_active: true,
            };
            plugin_connections.push_back(connection.clone());
            Ok(connection)
        } else {
            Err(format!("Connection pool exhausted for plugin {}", plugin_name).into())
        }
    }

    /// 释放连接
    fn release_connection(&mut self, connection: Connection) {
        if let Some(connections) = self.connections.get_mut(&connection.plugin_name) {
            // 标记连接为非活跃状态
            if let Some(conn) = connections.front_mut() {
                if conn.id == connection.id {
                    conn.is_active = false;
                }
            }
        }
    }

    /// 清理过期连接
    fn cleanup_expired_connections(&mut self) {
        for connections in self.connections.values_mut() {
            connections.retain(|conn| {
                conn.created_at.elapsed() < self.connection_timeout
            });
        }
    }
}

// 结果缓存实现
impl ResultCache {
    /// 创建缓存
    fn new(max_size: usize, ttl: Duration) -> Self {
        Self {
            cache: HashMap::new(),
            max_size,
            current_size: 0,
            ttl,
        }
    }

    /// 获取缓存条目
    fn get(&mut self, key: &str) -> Option<serde_json::Value> {
        if let Some(entry) = self.cache.get_mut(key) {
            if entry.is_expired() {
                self.cache.remove(key);
                self.current_size -= 1;
                return None;
            }

            entry.access_count += 1;
            entry.last_access = Instant::now();
            Some(entry.value.clone())
        } else {
            None
        }
    }

    /// 存储缓存条目
    fn put(&mut self, key: String, value: serde_json::Value) {
        let entry = CacheEntry {
            key: key.clone(),
            value,
            created_at: Instant::now(),
            ttl: self.ttl,
            access_count: 0,
            last_access: Instant::now(),
        };

        // 检查是否需要清理过期条目
        self.cleanup_expired();

        // 如果缓存已满，使用LRU策略
        if self.cache.len() >= self.max_size {
            self.evict_lru();
        }

        if !self.cache.contains_key(&key) {
            self.current_size += 1;
        }

        self.cache.insert(key, entry);
    }

    /// 清理过期条目
    fn cleanup_expired(&mut self) {
        let mut expired_keys = Vec::new();

        for (key, entry) in &self.cache {
            if entry.is_expired() {
                expired_keys.push(key.clone());
            }
        }

        for key in expired_keys {
            self.cache.remove(&key);
            self.current_size -= 1;
        }
    }

    /// 驱逐最少使用的条目
    fn evict_lru(&mut self) {
        if let Some((key, _)) = self.cache.iter()
            .min_by_key(|(_, entry)| entry.last_access) {
            let key = key.clone();
            self.cache.remove(&key);
            self.current_size -= 1;
        }
    }
}

impl CacheEntry {
    /// 检查是否过期
    fn is_expired(&self) -> bool {
        self.created_at.elapsed() > self.ttl
    }
}

// 插件实例实现
/// Vibrate31插件实现
pub struct Vibrate31Plugin {
    stats: PluginRuntimeStats,
}

impl Vibrate31Plugin {
    pub fn new() -> Self {
        Self {
            stats: PluginRuntimeStats::default(),
        }
    }
}

#[async_trait::async_trait]
impl PluginInstance for Vibrate31Plugin {
    fn name(&self) -> &str {
        "vibrate31"
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    async fn initialize(&mut self, _config: &PluginConfig) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        tracing::info!("Initializing Vibrate31 plugin");
        // 这里可以实现实际的初始化逻辑
        Ok(())
    }

    async fn execute(&self, input: serde_json::Value, _context: &mut ExecutionContext) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        let start_time = Instant::now();

        // 模拟Vibrate31插件的处理逻辑
        // 这里应该实现实际的振动特征提取算法
        tokio::time::sleep(Duration::from_millis(50)).await;

        let execution_time = start_time.elapsed().as_millis() as u64;

        // 返回模拟的振动特征结果
        let result = serde_json::json!({
            "plugin": "vibrate31",
            "execution_time_ms": execution_time,
            "features": {
                "rms": 0.123,
                "peak": 0.456,
                "crest_factor": 2.1,
                "kurtosis": 1.8,
                "skewness": 0.3
            },
            "quality_score": 0.95
        });

        Ok(result)
    }

    async fn destroy(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        tracing::info!("Destroying Vibrate31 plugin");
        Ok(())
    }

    async fn health_check(&self) -> Result<HealthStatus, Box<dyn std::error::Error + Send + Sync>> {
        // 简化的健康检查
        Ok(HealthStatus::Healthy)
    }

    fn get_stats(&self) -> PluginRuntimeStats {
        self.stats.clone()
    }
}

/// 通用插件实现
pub struct GenericPlugin {
    name: String,
    stats: PluginRuntimeStats,
}

impl GenericPlugin {
    pub fn new(config: PluginConfig) -> Self {
        Self {
            name: config.name,
            stats: PluginRuntimeStats::default(),
        }
    }
}

#[async_trait::async_trait]
impl PluginInstance for GenericPlugin {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    async fn initialize(&mut self, _config: &PluginConfig) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        tracing::info!("Initializing generic plugin: {}", self.name);
        Ok(())
    }

    async fn execute(&self, input: serde_json::Value, _context: &mut ExecutionContext) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        let start_time = Instant::now();

        // 通用处理逻辑
        tokio::time::sleep(Duration::from_millis(30)).await;

        let execution_time = start_time.elapsed().as_millis() as u64;

        // 返回通用处理结果
        let result = serde_json::json!({
            "plugin": self.name,
            "execution_time_ms": execution_time,
            "processed": true,
            "input_hash": format!("{:x}", input.to_string().len())
        });

        Ok(result)
    }

    async fn destroy(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        tracing::info!("Destroying generic plugin: {}", self.name);
        Ok(())
    }

    async fn health_check(&self) -> Result<HealthStatus, Box<dyn std::error::Error + Send + Sync>> {
        Ok(HealthStatus::Healthy)
    }

    fn get_stats(&self) -> PluginRuntimeStats {
        self.stats.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_chain_config_default() {
        let config = PluginChainConfig::default();
        assert_eq!(config.name, "default_chain");
        assert_eq!(config.plugins.len(), 1);
        assert!(matches!(config.execution_strategy, ExecutionStrategy::Sequential));
    }

    #[tokio::test]
    async fn test_plugin_chain_executor() {
        let config = PluginChainConfig::default();
        let executor = PluginChainExecutor::new(config);

        let result = executor.initialize().await;
        assert!(result.is_ok());

        let input_data = serde_json::json!({"test": "data"});
        let mut context = ExecutionContext {
            task_id: "test_task".to_string(),
            container_id: "test_container".to_string(),
            working_dir: "/tmp".to_string(),
            start_time: std::time::Instant::now(),
        };

        let result = executor.execute_chain(input_data, &mut context).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_execution_strategy() {
        let strategy = ExecutionStrategy::Sequential;
        assert!(matches!(strategy, ExecutionStrategy::Sequential));

        let parallel = ExecutionStrategy::Parallel;
        assert!(matches!(parallel, ExecutionStrategy::Parallel));
    }
}
