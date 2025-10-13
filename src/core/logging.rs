//! 日志系统配置和结构化日志

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// 日志配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// 日志级别
    pub level: String,
    /// 是否启用控制台输出
    pub console_output: bool,
    /// 文件输出配置
    pub file_output: Option<FileOutputConfig>,
    /// JSON格式输出
    pub json_format: bool,
    /// 包含源代码位置
    pub include_location: bool,
    /// 自定义字段
    pub custom_fields: HashMap<String, String>,
}

/// 文件输出配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileOutputConfig {
    /// 日志文件路径
    pub path: String,
    /// 轮转策略
    pub rotation: RotationStrategy,
    /// 最大文件大小（MB）
    pub max_size_mb: Option<u64>,
    /// 保留文件数量
    pub max_files: Option<usize>,
}

/// 轮转策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RotationStrategy {
    /// 按小时轮转
    Hourly,
    /// 按天轮转
    Daily,
    /// 从不轮转
    Never,
    /// 按分钟轮转
    Minutely,
}

/// 日志初始化器
pub struct Logger;

impl Logger {
    /// 初始化日志系统
    pub fn init(config: &LoggingConfig) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut layers = Vec::new();

        // 创建过滤器
        let filter = EnvFilter::try_from_env(&format!("RUST_LOG,{}", config.level))
            .unwrap_or_else(|_| EnvFilter::new(&config.level));

        // 控制台输出层
        if config.console_output {
            if config.json_format {
                let console_layer = tracing_subscriber::fmt::layer()
                    .json()
                    .with_current_span(false)
                    .with_span_list(false);

                layers.push(console_layer.boxed());
            } else {
                let console_layer = tracing_subscriber::fmt::layer()
                    .with_target(false)
                    .with_thread_ids(false)
                    .with_thread_names(false)
                    .compact();

                layers.push(console_layer.boxed());
            }
        }

        // 文件输出层
        if let Some(file_config) = &config.file_output {
            let rotation = match file_config.rotation {
                RotationStrategy::Hourly => Rotation::HOURLY,
                RotationStrategy::Daily => Rotation::DAILY,
                RotationStrategy::Never => Rotation::NEVER,
                RotationStrategy::Minutely => Rotation::MINUTELY,
            };

            // 确保日志目录存在
            if let Some(parent) = Path::new(&file_config.path).parent() {
                std::fs::create_dir_all(parent)?;
            }

            let file_appender = RollingFileAppender::new(rotation, "", &file_config.path);

            let file_layer = tracing_subscriber::fmt::layer()
                .with_writer(file_appender)
                .with_ansi(false)
                .with_target(true)
                .with_thread_ids(true)
                .with_thread_names(true)
                .with_file(true)
                .with_line_number(config.include_location);

            if config.json_format {
                let file_layer = file_layer.json();
                layers.push(file_layer.boxed());
            } else {
                layers.push(file_layer.boxed());
            }
        }

        // 注册层
        let registry = tracing_subscriber::registry().with(filter);

        let mut registry = registry;
        for layer in layers {
            registry = registry.with(layer);
        }

        registry.init();

        tracing::info!("Logger initialized with level: {}", config.level);
        if config.console_output {
            tracing::info!("Console output: enabled");
        }
        if let Some(file_config) = &config.file_output {
            tracing::info!("File output: {} (rotation: {:?})",
                file_config.path, file_config.rotation);
        }

        Ok(())
    }

    /// 刷新日志缓冲区
    pub fn flush() {
        tracing::info!("Flushing log buffers...");
        // 在实际实现中，这里可能需要刷新特定的日志后端
    }

    /// 关闭日志系统
    pub fn shutdown() {
        tracing::info!("Shutting down logger...");
        Self::flush();
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            console_output: true,
            file_output: Some(FileOutputConfig {
                path: "logs/app.log".to_string(),
                rotation: RotationStrategy::Daily,
                max_size_mb: Some(100),
                max_files: Some(30),
            }),
            json_format: false,
            include_location: true,
            custom_fields: HashMap::new(),
        }
    }
}

/// 结构化日志宏
#[macro_export]
macro_rules! log_request {
    ($method:expr, $path:expr, $status:expr, $duration:expr) => {
        tracing::info!(
            method = $method,
            path = $path,
            status = $status,
            duration_ms = $duration.as_millis(),
            "HTTP request"
        );
    };
}

#[macro_export]
macro_rules! log_error {
    ($error:expr, $context:expr) => {
        tracing::error!(
            error = %$error,
            context = $context,
            "Application error"
        );
    };
}

#[macro_export]
macro_rules! log_security_event {
    ($event:expr, $user:expr, $resource:expr) => {
        tracing::warn!(
            event = $event,
            user = $user,
            resource = $resource,
            "Security event"
        );
    };
}

#[macro_export]
macro_rules! log_performance {
    ($operation:expr, $duration:expr, $details:expr) => {
        tracing::info!(
            operation = $operation,
            duration_ms = $duration.as_millis(),
            details = $details,
            "Performance measurement"
        );
    };
}

/// 审计日志记录器
pub struct AuditLogRecorder {
    config: LoggingConfig,
}

impl AuditLogRecorder {
    /// 创建新的审计日志记录器
    pub fn new(config: LoggingConfig) -> Self {
        Self { config }
    }

    /// 记录审计事件
    pub fn record_audit_event(
        &self,
        event_type: &str,
        user: Option<&str>,
        action: &str,
        resource: &str,
        result: &str,
        details: HashMap<String, String>,
    ) {
        let span = tracing::info_span!(
            "audit",
            event_type = event_type,
            user = user.unwrap_or("anonymous"),
            action = action,
            resource = resource,
            result = result,
            details = ?details
        );

        let _enter = span.enter();
        tracing::info!("Audit event recorded");
    }

    /// 记录认证事件
    pub fn record_authentication(
        &self,
        user: Option<&str>,
        success: bool,
        client_ip: Option<&str>,
        user_agent: Option<&str>,
    ) {
        let mut details = HashMap::new();
        if let Some(ip) = client_ip {
            details.insert("client_ip".to_string(), ip.to_string());
        }
        if let Some(ua) = user_agent {
            details.insert("user_agent".to_string(), ua.to_string());
        }

        let result = if success { "success" } else { "failure" };

        self.record_audit_event(
            "authentication",
            user,
            "login",
            "auth",
            result,
            details,
        );
    }

    /// 记录授权事件
    pub fn record_authorization(
        &self,
        user: &str,
        permission: &str,
        resource: &str,
        granted: bool,
    ) {
        let mut details = HashMap::new();
        details.insert("permission".to_string(), permission.to_string());

        let result = if granted { "granted" } else { "denied" };

        self.record_audit_event(
            "authorization",
            Some(user),
            "access",
            resource,
            result,
            details,
        );
    }

    /// 记录数据访问事件
    pub fn record_data_access(
        &self,
        user: &str,
        operation: &str,
        resource: &str,
        record_count: Option<usize>,
    ) {
        let mut details = HashMap::new();
        if let Some(count) = record_count {
            details.insert("record_count".to_string(), count.to_string());
        }

        self.record_audit_event(
            "data_access",
            Some(user),
            operation,
            resource,
            "success",
            details,
        );
    }

    /// 记录配置变更事件
    pub fn record_config_change(
        &self,
        user: &str,
        config_key: &str,
        old_value: Option<&str>,
        new_value: Option<&str>,
    ) {
        let mut details = HashMap::new();
        if let Some(old) = old_value {
            details.insert("old_value".to_string(), old.to_string());
        }
        if let Some(new) = new_value {
            details.insert("new_value".to_string(), new.to_string());
        }

        self.record_audit_event(
            "config_change",
            Some(user),
            "modify",
            config_key,
            "success",
            details,
        );
    }
}

/// 性能日志记录器
pub struct PerformanceLogger;

impl PerformanceLogger {
    /// 记录请求性能
    pub fn record_request_performance(
        method: &str,
        path: &str,
        status: u16,
        duration: std::time::Duration,
        response_size: Option<usize>,
    ) {
        let span = tracing::info_span!(
            "http_request",
            method = method,
            path = path,
            status = status,
            duration_ms = duration.as_millis(),
            response_size = response_size
        );

        let _enter = span.enter();

        if duration.as_millis() > 1000 {
            tracing::warn!("Slow request detected");
        } else if duration.as_millis() > 100 {
            tracing::info!("Request completed");
        } else {
            tracing::debug!("Request completed");
        }
    }

    /// 记录数据库操作性能
    pub fn record_db_performance(
        operation: &str,
        table: &str,
        duration: std::time::Duration,
        record_count: Option<usize>,
    ) {
        let span = tracing::info_span!(
            "db_operation",
            operation = operation,
            table = table,
            duration_ms = duration.as_millis(),
            record_count = record_count
        );

        let _enter = span.enter();

        if duration.as_millis() > 500 {
            tracing::warn!("Slow database operation detected");
        } else {
            tracing::debug!("Database operation completed");
        }
    }

    /// 记录任务执行性能
    pub fn record_task_performance(
        task_id: &str,
        algorithm: &str,
        duration: std::time::Duration,
        success: bool,
    ) {
        let span = tracing::info_span!(
            "task_execution",
            task_id = task_id,
            algorithm = algorithm,
            duration_ms = duration.as_millis(),
            success = success
        );

        let _enter = span.enter();

        if success {
            tracing::info!("Task execution completed");
        } else {
            tracing::error!("Task execution failed");
        }
    }

    /// 记录系统资源使用情况
    pub fn record_system_resources(
        cpu_usage: f64,
        memory_usage_mb: f64,
        disk_usage_mb: f64,
    ) {
        tracing::info!(
            cpu_usage_percent = cpu_usage,
            memory_usage_mb = memory_usage_mb,
            disk_usage_mb = disk_usage_mb,
            "System resource usage"
        );
    }
}

/// 日志轮转管理器
pub struct LogRotationManager {
    config: LoggingConfig,
}

impl LogRotationManager {
    /// 创建新的日志轮转管理器
    pub fn new(config: LoggingConfig) -> Self {
        Self { config }
    }

    /// 检查并执行日志轮转
    pub async fn check_rotation(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(file_config) = &self.config.file_output {
            let log_path = Path::new(&file_config.path);

            // 检查文件是否存在
            if !log_path.exists() {
                return Ok(());
            }

            // 获取文件大小
            let metadata = tokio::fs::metadata(log_path).await?;
            let file_size_mb = metadata.len() / (1024 * 1024);

            // 检查是否需要轮转
            if let Some(max_size) = file_config.max_size_mb {
                if file_size_mb >= max_size {
                    tracing::info!("Log file size {}MB exceeds limit {}MB, rotating...",
                        file_size_mb, max_size);
                    self.rotate_log_file().await?;
                }
            }

            // 清理旧的日志文件
            if let Some(max_files) = file_config.max_files {
                self.cleanup_old_logs(max_files).await?;
            }
        }

        Ok(())
    }

    /// 执行日志文件轮转
    async fn rotate_log_file(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(file_config) = &self.config.file_output {
            let log_path = Path::new(&file_config.path);

            // 生成新的文件名（带时间戳）
            let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
            let backup_path = log_path.with_extension(format!("{}.log", timestamp));

            // 重命名当前日志文件
            tokio::fs::rename(log_path, &backup_path).await?;

            tracing::info!("Log file rotated: {} -> {}",
                log_path.display(), backup_path.display());
        }

        Ok(())
    }

    /// 清理旧的日志文件
    async fn cleanup_old_logs(&self, max_files: usize) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(file_config) = &self.config.file_output {
            let log_path = Path::new(&file_config.path);

            if let Some(parent_dir) = log_path.parent() {
                let mut entries = tokio::fs::read_dir(parent_dir).await?;
                let mut log_files = Vec::new();

                // 收集所有日志文件
                while let Some(entry) = entries.next_entry().await? {
                    let path = entry.path();
                    if let Some(extension) = path.extension() {
                        if extension == "log" && path.to_string_lossy().contains(&log_path.file_stem().unwrap().to_string_lossy()) {
                            if let Ok(metadata) = entry.metadata().await {
                                log_files.push((path, metadata.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH)));
                            }
                        }
                    }
                }

                // 按修改时间排序（最新的在前面）
                log_files.sort_by(|a, b| b.1.cmp(&a.1));

                // 删除超出限制的文件
                if log_files.len() > max_files {
                    for (path, _) in log_files.iter().skip(max_files) {
                        tokio::fs::remove_file(path).await?;
                        tracing::info!("Removed old log file: {}", path.display());
                    }
                }
            }
        }

        Ok(())
    }
}
