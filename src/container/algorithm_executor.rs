//! 容器化算法执行器
//!
//! 提供基于Youki容器的算法插件执行功能，支持：
//! - 算法插件的容器化运行
//! - 资源隔离和限制
//! - 安全沙箱环境
//! - 性能监控和日志收集
//! - 算法版本管理和更新

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, Mutex};
use tokio::process::Command;
use serde::{Deserialize, Serialize};

use crate::core::{ComputeRequest, ComputeResponse, ContainerConfig, Result};
use crate::ffi::MemoryManager;

/// 容器化算法执行器
pub struct ContainerizedAlgorithmExecutor {
    /// Youki容器管理器
    container_manager: Arc<super::YoukiContainerManager>,
    /// 算法插件注册表
    algorithm_registry: Arc<RwLock<AlgorithmRegistry>>,
    /// 内存管理器
    memory_manager: Arc<MemoryManager>,
    /// 执行统计
    stats: Arc<RwLock<ExecutionStats>>,
    /// 运行时配置
    runtime_config: RuntimeConfig,
}

/// 算法插件注册表
#[derive(Debug, Clone)]
pub struct AlgorithmRegistry {
    /// 算法映射：算法名 -> 算法信息
    algorithms: HashMap<String, AlgorithmInfo>,
    /// 插件镜像映射：算法名 -> 镜像信息
    plugin_images: HashMap<String, PluginImage>,
}

/// 算法信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlgorithmInfo {
    /// 算法名称
    pub name: String,
    /// 算法版本
    pub version: String,
    /// 算法描述
    pub description: String,
    /// 输入参数模式
    pub input_schema: serde_json::Value,
    /// 输出结果模式
    pub output_schema: serde_json::Value,
    /// 资源需求
    pub resource_requirements: ResourceRequirements,
    /// 执行超时时间（秒）
    pub timeout_seconds: u64,
    /// 最大并发执行数
    pub max_concurrent: usize,
}

/// 插件镜像信息
#[derive(Debug, Clone)]
pub struct PluginImage {
    /// 镜像名称
    pub image_name: String,
    /// 镜像版本
    pub image_version: String,
    /// 镜像路径
    pub image_path: PathBuf,
    /// 算法执行命令
    pub execute_command: Vec<String>,
    /// 环境变量
    pub environment: HashMap<String, String>,
    /// 挂载点
    pub mounts: Vec<MountPoint>,
}

/// 挂载点配置
#[derive(Debug, Clone)]
pub struct MountPoint {
    /// 主机路径
    pub host_path: PathBuf,
    /// 容器路径
    pub container_path: PathBuf,
    /// 挂载选项
    pub options: Vec<String>,
    /// 是否只读
    pub readonly: bool,
}

/// 资源需求配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    /// CPU核心数
    pub cpu_cores: f64,
    /// 内存限制（MB）
    pub memory_mb: u64,
    /// 磁盘空间限制（MB）
    pub disk_mb: u64,
    /// 网络带宽限制（Mbps）
    pub network_mbps: Option<u64>,
}

/// 运行时配置
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// 工作目录
    pub workspace_dir: PathBuf,
    /// 容器运行时目录
    pub runtime_dir: PathBuf,
    /// 插件目录
    pub plugins_dir: PathBuf,
    /// 默认超时时间
    pub default_timeout: Duration,
    /// 清理间隔
    pub cleanup_interval: Duration,
    /// 启用调试模式
    pub debug_mode: bool,
}

/// 执行统计
#[derive(Debug, Clone, Default)]
pub struct ExecutionStats {
    /// 总执行次数
    pub total_executions: usize,
    /// 成功执行次数
    pub successful_executions: usize,
    /// 失败执行次数
    pub failed_executions: usize,
    /// 超时执行次数
    pub timeout_executions: usize,
    /// 平均执行时间
    pub avg_execution_time_ms: f64,
    /// 资源使用统计
    pub resource_stats: ResourceStats,
}

/// 资源使用统计
#[derive(Debug, Clone, Default)]
pub struct ResourceStats {
    /// 峰值CPU使用率
    pub peak_cpu_usage: f64,
    /// 峰值内存使用量
    pub peak_memory_usage: usize,
    /// 总CPU时间
    pub total_cpu_time_ms: u64,
    /// 总I/O操作数
    pub total_io_operations: u64,
}

/// 执行结果
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    /// 执行ID
    pub execution_id: String,
    /// 容器ID
    pub container_id: String,
    /// 执行状态
    pub status: ExecutionStatus,
    /// 执行结果
    pub result: Option<serde_json::Value>,
    /// 错误信息
    pub error_message: Option<String>,
    /// 执行时间
    pub execution_time_ms: u64,
    /// 资源使用情况
    pub resource_usage: ResourceUsage,
    /// 开始时间
    pub started_at: Instant,
    /// 结束时间
    pub finished_at: Instant,
}

/// 执行状态
#[derive(Debug, Clone, PartialEq)]
pub enum ExecutionStatus {
    /// 成功
    Success,
    /// 失败
    Failed,
    /// 超时
    Timeout,
    /// 取消
    Cancelled,
    /// 资源不足
    ResourceExhausted,
}

/// 资源使用情况
#[derive(Debug, Clone)]
pub struct ResourceUsage {
    /// CPU使用率
    pub cpu_usage_percent: f64,
    /// 内存使用量（MB）
    pub memory_usage_mb: u64,
    /// I/O操作数
    pub io_operations: u64,
    /// 网络流量（字节）
    pub network_bytes: u64,
}

impl ContainerizedAlgorithmExecutor {
    /// 创建新的容器化算法执行器
    pub fn new(
        container_manager: Arc<super::YoukiContainerManager>,
        memory_manager: Arc<MemoryManager>,
    ) -> Self {
        let runtime_config = RuntimeConfig {
            workspace_dir: PathBuf::from("./workspace"),
            runtime_dir: PathBuf::from("./runtime"),
            plugins_dir: PathBuf::from("./plugins"),
            default_timeout: Duration::from_secs(300), // 5分钟
            cleanup_interval: Duration::from_secs(3600), // 1小时
            debug_mode: false,
        };

        Self {
            container_manager,
            algorithm_registry: Arc::new(RwLock::new(AlgorithmRegistry::new())),
            memory_manager,
            stats: Arc::new(RwLock::new(ExecutionStats::default())),
            runtime_config,
        }
    }

    /// 注册算法插件
    pub async fn register_algorithm(&self, info: AlgorithmInfo, image: PluginImage) -> Result<()> {
        let mut registry = self.algorithm_registry.write().await;

        // 验证算法信息
        self.validate_algorithm_info(&info)?;

        // 验证镜像信息
        self.validate_plugin_image(&image)?;

        // 检查镜像是否存在
        if !self.check_image_exists(&image).await? {
            return Err(format!("Plugin image not found: {}", image.image_name).into());
        }

        registry.algorithms.insert(info.name.clone(), info);
        registry.plugin_images.insert(info.name.clone(), image);

        tracing::info!("Registered algorithm plugin: {}", info.name);
        Ok(())
    }

    /// 执行算法任务
    pub async fn execute_algorithm(
        &self,
        request: ComputeRequest,
    ) -> Result<ExecutionResult> {
        let start_time = Instant::now();
        let execution_id = format!("exec_{}", uuid::Uuid::new_v4());

        tracing::info!("Starting containerized algorithm execution: {}", execution_id);

        // 1. 查找算法信息
        let (algorithm_info, plugin_image) = {
            let registry = self.algorithm_registry.read().await;
            let algorithm = registry.algorithms.get(&request.algorithm)
                .ok_or_else(|| format!("Algorithm not found: {}", request.algorithm))?;
            let image = registry.plugin_images.get(&request.algorithm)
                .ok_or_else(|| format!("Plugin image not found: {}", request.algorithm))?;

            (algorithm.clone(), image.clone())
        };

        // 2. 验证输入参数
        self.validate_algorithm_input(&algorithm_info, &request)?;

        // 3. 准备执行环境
        let execution_dir = self.runtime_config.workspace_dir
            .join("executions")
            .join(&execution_id);
        tokio::fs::create_dir_all(&execution_dir).await
            .map_err(|e| format!("Failed to create execution directory: {}", e))?;

        // 4. 准备输入数据文件
        let input_file = execution_dir.join("input.json");
        let input_data = serde_json::json!({
            "execution_id": execution_id,
            "algorithm": request.algorithm,
            "parameters": request.parameters,
            "metadata": {
                "submitted_at": chrono::Utc::now().to_rfc3339(),
                "timeout_seconds": algorithm_info.timeout_seconds,
            }
        });

        tokio::fs::write(&input_file, input_data.to_string()).await
            .map_err(|e| format!("Failed to write input data: {}", e))?;

        // 5. 创建容器配置
        let container_config = self.create_container_config(
            &algorithm_info,
            &plugin_image,
            &execution_dir,
            &input_file,
        )?;

        // 6. 创建并启动容器
        let container_id = self.container_manager.create_container(
            container_config,
            request.algorithm.clone(),
        ).await?;

        // 7. 等待算法执行完成
        let timeout_duration = Duration::from_secs(algorithm_info.timeout_seconds);
        let execution_result = tokio::time::timeout(
            timeout_duration,
            self.wait_for_execution_completion(&container_id, &execution_dir)
        ).await;

        let end_time = Instant::now();
        let execution_time = end_time.duration_since(start_time).as_millis() as u64;

        // 8. 处理执行结果
        let result = match execution_result {
            Ok(Ok(output_data)) => {
                // 执行成功
                let mut stats = self.stats.write().await;
                stats.total_executions += 1;
                stats.successful_executions += 1;
                stats.avg_execution_time_ms = (stats.avg_execution_time_ms * (stats.total_executions as f64 - 1.0) + execution_time as f64) / stats.total_executions as f64;

                ExecutionResult {
                    execution_id: execution_id.clone(),
                    container_id,
                    status: ExecutionStatus::Success,
                    result: Some(output_data),
                    error_message: None,
                    execution_time_ms: execution_time,
                    resource_usage: self.collect_resource_usage(&container_id).await,
                    started_at: start_time,
                    finished_at: end_time,
                }
            }
            Ok(Err(e)) => {
                // 执行失败
                let mut stats = self.stats.write().await;
                stats.total_executions += 1;
                stats.failed_executions += 1;

                ExecutionResult {
                    execution_id: execution_id.clone(),
                    container_id,
                    status: ExecutionStatus::Failed,
                    result: None,
                    error_message: Some(e.to_string()),
                    execution_time_ms: execution_time,
                    resource_usage: self.collect_resource_usage(&container_id).await,
                    started_at: start_time,
                    finished_at: end_time,
                }
            }
            Err(_) => {
                // 执行超时
                let mut stats = self.stats.write().await;
                stats.total_executions += 1;
                stats.timeout_executions += 1;

                ExecutionResult {
                    execution_id: execution_id.clone(),
                    container_id,
                    status: ExecutionStatus::Timeout,
                    result: None,
                    error_message: Some(format!("Execution timeout after {} seconds", algorithm_info.timeout_seconds)),
                    execution_time_ms: execution_time,
                    resource_usage: self.collect_resource_usage(&container_id).await,
                    started_at: start_time,
                    finished_at: end_time,
                }
            }
        };

        // 9. 清理容器
        if let Err(e) = self.container_manager.stop_container(&container_id).await {
            tracing::warn!("Failed to stop container {}: {}", container_id, e);
        }

        if let Err(e) = self.container_manager.destroy_container(&container_id).await {
            tracing::warn!("Failed to destroy container {}: {}", container_id, e);
        }

        // 10. 清理执行目录（延迟清理，避免影响调试）
        if !self.runtime_config.debug_mode {
            let _ = tokio::spawn(async move {
                tokio::time::sleep(Duration::from_secs(60)).await; // 延迟1分钟清理
                let _ = tokio::fs::remove_dir_all(&execution_dir).await;
            });
        }

        tracing::info!("Containerized algorithm execution completed: {}", execution_id);
        Ok(result)
    }

    /// 获取算法列表
    pub async fn list_algorithms(&self) -> Vec<AlgorithmInfo> {
        let registry = self.algorithm_registry.read().await;
        registry.algorithms.values().cloned().collect()
    }

    /// 获取执行统计
    pub async fn get_execution_stats(&self) -> ExecutionStats {
        self.stats.read().await.clone()
    }

    /// 卸载算法插件
    pub async fn unregister_algorithm(&self, algorithm_name: &str) -> Result<()> {
        let mut registry = self.algorithm_registry.write().await;

        registry.algorithms.remove(algorithm_name);
        registry.plugin_images.remove(algorithm_name);

        tracing::info!("Unregistered algorithm plugin: {}", algorithm_name);
        Ok(())
    }

    /// 验证算法信息
    fn validate_algorithm_info(&self, info: &AlgorithmInfo) -> Result<()> {
        if info.name.is_empty() {
            return Err("Algorithm name cannot be empty".into());
        }

        if info.timeout_seconds == 0 {
            return Err("Timeout must be greater than 0".into());
        }

        if info.max_concurrent == 0 {
            return Err("Max concurrent must be greater than 0".into());
        }

        Ok(())
    }

    /// 验证插件镜像
    fn validate_plugin_image(&self, image: &PluginImage) -> Result<()> {
        if image.image_name.is_empty() {
            return Err("Image name cannot be empty".into());
        }

        if image.execute_command.is_empty() {
            return Err("Execute command cannot be empty".into());
        }

        Ok(())
    }

    /// 检查镜像是否存在
    async fn check_image_exists(&self, image: &PluginImage) -> Result<bool> {
        // 这里应该检查镜像文件是否存在
        // 暂时简化为检查路径是否存在
        Ok(tokio::fs::try_exists(&image.image_path).await.unwrap_or(false))
    }

    /// 验证算法输入
    fn validate_algorithm_input(&self, algorithm: &AlgorithmInfo, request: &ComputeRequest) -> Result<()> {
        // 这里应该根据算法的输入模式验证请求参数
        // 暂时简化为基本检查
        if request.algorithm != algorithm.name {
            return Err(format!("Algorithm name mismatch: expected {}, got {}",
                             algorithm.name, request.algorithm).into());
        }

        Ok(())
    }

    /// 创建容器配置
    fn create_container_config(
        &self,
        algorithm: &AlgorithmInfo,
        image: &PluginImage,
        execution_dir: &Path,
        input_file: &Path,
    ) -> Result<ContainerConfig> {
        let container_id = format!("alg_{}", uuid::Uuid::new_v4());

        // 创建环境变量
        let mut env = image.environment.clone();
        env.insert("ALGORITHM_NAME".to_string(), algorithm.name.clone());
        env.insert("ALGORITHM_VERSION".to_string(), algorithm.version.clone());
        env.insert("EXECUTION_TIMEOUT".to_string(), algorithm.timeout_seconds.to_string());
        env.insert("INPUT_FILE".to_string(), "/input/input.json".to_string());
        env.insert("OUTPUT_FILE".to_string(), "/output/result.json".to_string());

        // 创建挂载点
        let mut mounts = image.mounts.clone();

        // 添加输入文件挂载
        mounts.push(MountPoint {
            host_path: input_file.parent().unwrap().to_path_buf(),
            container_path: PathBuf::from("/input"),
            options: vec!["ro".to_string()],
            readonly: true,
        });

        // 添加输出目录挂载
        let output_dir = execution_dir.join("output");
        tokio::fs::create_dir_all(&output_dir).await
            .map_err(|e| format!("Failed to create output directory: {}", e))?;

        mounts.push(MountPoint {
            host_path: output_dir,
            container_path: PathBuf::from("/output"),
            options: vec!["rw".to_string()],
            readonly: false,
        });

        Ok(ContainerConfig {
            name: container_id,
            image: image.image_path.to_string_lossy().to_string(),
            command: image.execute_command.clone(),
            env: env.into_iter().map(|(k, v)| format!("{}={}", k, v)).collect(),
            working_dir: "/".to_string(),
            cpu_limit: Some(algorithm.resource_requirements.cpu_cores),
            memory_limit: Some(algorithm.resource_requirements.memory_mb * 1024 * 1024), // 转换为字节
            network_enabled: true,
            privileged: false,
        })
    }

    /// 等待执行完成
    async fn wait_for_execution_completion(
        &self,
        container_id: &str,
        execution_dir: &Path,
    ) -> Result<serde_json::Value> {
        // 这里应该通过容器日志或文件监控来检测执行完成
        // 暂时简化为等待一段时间，然后检查输出文件

        tokio::time::sleep(Duration::from_millis(100)).await; // 短暂等待

        let output_file = execution_dir.join("output").join("result.json");

        // 尝试读取输出文件，最多重试10次
        for _ in 0..10 {
            if tokio::fs::try_exists(&output_file).await.unwrap_or(false) {
                let content = tokio::fs::read_to_string(&output_file).await
                    .map_err(|e| format!("Failed to read output file: {}", e))?;

                return serde_json::from_str(&content)
                    .map_err(|e| format!("Failed to parse output JSON: {}", e).into());
            }

            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        Err("Algorithm execution did not produce output file within timeout".into())
    }

    /// 收集资源使用情况
    async fn collect_resource_usage(&self, container_id: &str) -> ResourceUsage {
        // 这里应该从容器运行时收集实际的资源使用情况
        // 暂时返回模拟数据

        ResourceUsage {
            cpu_usage_percent: 45.2,
            memory_usage_mb: 128,
            io_operations: 1024,
            network_bytes: 8192,
        }
    }
}

impl AlgorithmRegistry {
    /// 创建新的算法注册表
    pub fn new() -> Self {
        Self {
            algorithms: HashMap::new(),
            plugin_images: HashMap::new(),
        }
    }
}

impl Default for ContainerizedAlgorithmExecutor {
    fn default() -> Self {
        let container_manager = Arc::new(super::YoukiContainerManager::new(PathBuf::from("./runtime")));
        let memory_manager = Arc::new(crate::ffi::MemoryManager::new());

        Self::new(container_manager, memory_manager)
    }
}

/// 便捷的算法插件构建器
pub struct AlgorithmPluginBuilder {
    info: AlgorithmInfo,
    image: PluginImage,
}

impl AlgorithmPluginBuilder {
    /// 创建新的插件构建器
    pub fn new(name: &str, version: &str) -> Self {
        Self {
            info: AlgorithmInfo {
                name: name.to_string(),
                version: version.to_string(),
                description: String::new(),
                input_schema: serde_json::Value::Null,
                output_schema: serde_json::Value::Null,
                resource_requirements: ResourceRequirements {
                    cpu_cores: 1.0,
                    memory_mb: 256,
                    disk_mb: 1024,
                    network_mbps: Some(10),
                },
                timeout_seconds: 300,
                max_concurrent: 10,
            },
            image: PluginImage {
                image_name: format!("{}-plugin", name),
                image_version: version.to_string(),
                image_path: PathBuf::new(),
                execute_command: vec!["/usr/local/bin/algorithm".to_string()],
                environment: HashMap::new(),
                mounts: Vec::new(),
            },
        }
    }

    /// 设置描述
    pub fn description(mut self, desc: &str) -> Self {
        self.info.description = desc.to_string();
        self
    }

    /// 设置资源需求
    pub fn resources(mut self, cpu: f64, memory_mb: u64) -> Self {
        self.info.resource_requirements.cpu_cores = cpu;
        self.info.resource_requirements.memory_mb = memory_mb;
        self
    }

    /// 设置超时时间
    pub fn timeout(mut self, seconds: u64) -> Self {
        self.info.timeout_seconds = seconds;
        self
    }

    /// 设置镜像路径
    pub fn image_path(mut self, path: PathBuf) -> Self {
        self.image.image_path = path;
        self
    }

    /// 设置执行命令
    pub fn execute_command(mut self, cmd: Vec<String>) -> Self {
        self.image.execute_command = cmd;
        self
    }

    /// 添加环境变量
    pub fn env(mut self, key: &str, value: &str) -> Self {
        self.image.environment.insert(key.to_string(), value.to_string());
        self
    }

    /// 构建算法插件
    pub fn build(self) -> (AlgorithmInfo, PluginImage) {
        (self.info, self.image)
    }
}
