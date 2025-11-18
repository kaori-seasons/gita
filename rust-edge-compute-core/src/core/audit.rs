//! 审计日志和安全事件记录

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info, warn};

/// 审计事件类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditEventType {
    /// 认证事件
    Authentication,
    /// 授权事件
    Authorization,
    /// 访问控制事件
    AccessControl,
    /// 数据访问事件
    DataAccess,
    /// 配置变更事件
    ConfigurationChange,
    /// 系统事件
    SystemEvent,
    /// 安全事件
    SecurityEvent,
    /// 错误事件
    ErrorEvent,
}

/// 审计事件严重程度
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditSeverity {
    /// 低严重程度
    Low,
    /// 中等严重程度
    Medium,
    /// 高严重程度
    High,
    /// 严重安全事件
    Critical,
}

/// 审计事件记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    /// 事件ID
    pub id: String,
    /// 事件类型
    pub event_type: AuditEventType,
    /// 严重程度
    pub severity: AuditSeverity,
    /// 时间戳
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// 用户ID（如果适用）
    pub user_id: Option<String>,
    /// 会话ID
    pub session_id: Option<String>,
    /// 客户端IP
    pub client_ip: Option<String>,
    /// 用户代理
    pub user_agent: Option<String>,
    /// 操作描述
    pub action: String,
    /// 资源标识
    pub resource: String,
    /// 操作结果
    pub result: AuditResult,
    /// 详细信息
    pub details: HashMap<String, String>,
    /// 相关元数据
    pub metadata: HashMap<String, String>,
}

/// 审计结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditResult {
    /// 成功
    Success,
    /// 失败
    Failure(String),
    /// 拒绝访问
    Denied,
    /// 警告
    Warning(String),
}

/// 审计配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditConfig {
    /// 是否启用审计
    pub enabled: bool,
    /// 日志文件路径
    pub log_file_path: Option<String>,
    /// 最大日志文件大小（MB）
    pub max_file_size_mb: u64,
    /// 保留的日志文件数量
    pub max_backup_files: usize,
    /// 是否记录成功事件
    pub log_success_events: bool,
    /// 是否记录失败事件
    pub log_failure_events: bool,
    /// 是否记录警告事件
    pub log_warning_events: bool,
    /// 敏感信息过滤器
    pub sensitive_fields: Vec<String>,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            log_file_path: Some("logs/audit.log".to_string()),
            max_file_size_mb: 100,
            max_backup_files: 10,
            log_success_events: true,
            log_failure_events: true,
            log_warning_events: true,
            sensitive_fields: vec![
                "password".to_string(),
                "token".to_string(),
                "secret".to_string(),
                "key".to_string(),
                "authorization".to_string(),
            ],
        }
    }
}

/// 审计日志器
pub struct AuditLogger {
    config: AuditConfig,
    events: Arc<Mutex<Vec<AuditEvent>>>,
}

impl AuditLogger {
    /// 创建新的审计日志器
    pub fn new(config: AuditConfig) -> Self {
        Self {
            config,
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// 记录审计事件
    pub async fn log_event(&self, mut event: AuditEvent) {
        if !self.config.enabled {
            return;
        }

        // 过滤敏感信息
        self.filter_sensitive_data(&mut event);

        // 添加到内存缓冲区
        {
            let mut events = self.events.lock().await;
            events.push(event.clone());

            // 如果缓冲区过大，写入文件
            if events.len() >= 100 {
                self.flush_events().await;
            }
        }

        // 根据事件类型和结果记录到tracing日志
        self.log_to_tracing(&event);
    }

    /// 记录认证事件
    pub async fn log_authentication(&self, user_id: Option<&str>, result: &AuditResult, details: HashMap<String, String>) {
        let event = AuditEvent {
            id: uuid::Uuid::new_v4().to_string(),
            event_type: AuditEventType::Authentication,
            severity: match result {
                AuditResult::Success => AuditSeverity::Low,
                AuditResult::Failure(_) => AuditSeverity::High,
                AuditResult::Denied => AuditSeverity::High,
                AuditResult::Warning(_) => AuditSeverity::Medium,
            },
            timestamp: chrono::Utc::now(),
            user_id: user_id.map(|s| s.to_string()),
            session_id: details.get("session_id").cloned(),
            client_ip: details.get("client_ip").cloned(),
            user_agent: details.get("user_agent").cloned(),
            action: "authentication".to_string(),
            resource: "auth".to_string(),
            result: result.clone(),
            details,
            metadata: HashMap::new(),
        };

        self.log_event(event).await;
    }

    /// 记录授权事件
    pub async fn log_authorization(&self, user_id: &str, resource: &str, action: &str, result: &AuditResult) {
        let event = AuditEvent {
            id: uuid::Uuid::new_v4().to_string(),
            event_type: AuditEventType::Authorization,
            severity: match result {
                AuditResult::Success => AuditSeverity::Low,
                AuditResult::Failure(_) => AuditSeverity::High,
                AuditResult::Denied => AuditSeverity::Medium,
                AuditResult::Warning(_) => AuditSeverity::Medium,
            },
            timestamp: chrono::Utc::now(),
            user_id: Some(user_id.to_string()),
            session_id: None,
            client_ip: None,
            user_agent: None,
            action: action.to_string(),
            resource: resource.to_string(),
            result: result.clone(),
            details: HashMap::new(),
            metadata: HashMap::new(),
        };

        self.log_event(event).await;
    }

    /// 记录API访问事件
    pub async fn log_api_access(&self, user_id: Option<&str>, method: &str, path: &str, status_code: u16, client_ip: Option<&str>) {
        let severity = if status_code >= 400 {
            AuditSeverity::Medium
        } else {
            AuditSeverity::Low
        };

        let result = if status_code >= 200 && status_code < 300 {
            AuditResult::Success
        } else if status_code >= 400 && status_code < 500 {
            AuditResult::Failure(format!("Client error: {}", status_code))
        } else if status_code >= 500 {
            AuditResult::Failure(format!("Server error: {}", status_code))
        } else {
            AuditResult::Warning(format!("Unexpected status: {}", status_code))
        };

        let event = AuditEvent {
            id: uuid::Uuid::new_v4().to_string(),
            event_type: AuditEventType::AccessControl,
            severity,
            timestamp: chrono::Utc::now(),
            user_id: user_id.map(|s| s.to_string()),
            session_id: None,
            client_ip: client_ip.map(|s| s.to_string()),
            user_agent: None,
            action: method.to_string(),
            resource: path.to_string(),
            result,
            details: HashMap::new(),
            metadata: HashMap::new(),
        };

        self.log_event(event).await;
    }

    /// 记录配置变更事件
    pub async fn log_config_change(&self, user_id: &str, config_key: &str, old_value: Option<&str>, new_value: Option<&str>) {
        let mut details = HashMap::new();
        if let Some(old) = old_value {
            details.insert("old_value".to_string(), old.to_string());
        }
        if let Some(new) = new_value {
            details.insert("new_value".to_string(), new.to_string());
        }

        let event = AuditEvent {
            id: uuid::Uuid::new_v4().to_string(),
            event_type: AuditEventType::ConfigurationChange,
            severity: AuditSeverity::Medium,
            timestamp: chrono::Utc::now(),
            user_id: Some(user_id.to_string()),
            session_id: None,
            client_ip: None,
            user_agent: None,
            action: "config_change".to_string(),
            resource: config_key.to_string(),
            result: AuditResult::Success,
            details,
            metadata: HashMap::new(),
        };

        self.log_event(event).await;
    }

    /// 记录安全事件
    pub async fn log_security_event(&self, event_type: &str, severity: AuditSeverity, details: HashMap<String, String>) {
        let event = AuditEvent {
            id: uuid::Uuid::new_v4().to_string(),
            event_type: AuditEventType::SecurityEvent,
            severity,
            timestamp: chrono::Utc::now(),
            user_id: details.get("user_id").cloned(),
            session_id: details.get("session_id").cloned(),
            client_ip: details.get("client_ip").cloned(),
            user_agent: details.get("user_agent").cloned(),
            action: event_type.to_string(),
            resource: details.get("resource").unwrap_or(&"unknown".to_string()).clone(),
            result: AuditResult::Success,
            details,
            metadata: HashMap::new(),
        };

        self.log_event(event).await;
    }

    /// 过滤敏感信息
    fn filter_sensitive_data(&self, event: &mut AuditEvent) {
        for field in &self.config.sensitive_fields {
            if let Some(value) = event.details.get_mut(field) {
                *value = "***FILTERED***".to_string();
            }
            if let Some(value) = event.metadata.get_mut(field) {
                *value = "***FILTERED***".to_string();
            }
        }
    }

    /// 记录到tracing日志
    fn log_to_tracing(&self, event: &AuditEvent) {
        let level = match event.severity {
            AuditSeverity::Low => tracing::Level::DEBUG,
            AuditSeverity::Medium => tracing::Level::INFO,
            AuditSeverity::High => tracing::Level::WARN,
            AuditSeverity::Critical => tracing::Level::ERROR,
        };

        let message = format!(
            "AUDIT [{}] {} {} {} {} - {:?}",
            event.event_type.as_ref(),
            event.user_id.as_deref().unwrap_or("anonymous"),
            event.action,
            event.resource,
            match &event.result {
                AuditResult::Success => "SUCCESS",
                AuditResult::Failure(msg) => &format!("FAILURE: {}", msg),
                AuditResult::Denied => "DENIED",
                AuditResult::Warning(msg) => &format!("WARNING: {}", msg),
            },
            event.details
        );

        match level {
            tracing::Level::DEBUG => tracing::debug!("{}", message),
            tracing::Level::INFO => tracing::info!("{}", message),
            tracing::Level::WARN => tracing::warn!("{}", message),
            tracing::Level::ERROR => tracing::error!("{}", message),
            _ => tracing::info!("{}", message),
        }
    }

    /// 刷新事件到文件
    async fn flush_events(&self) {
        let events = {
            let mut events_guard = self.events.lock().await;
            let events_to_flush = events_guard.clone();
            events_guard.clear();
            events_to_flush
        };

        if events.is_empty() {
            return;
        }

        if let Some(log_path) = &self.config.log_file_path {
            // 确保日志目录存在
            if let Some(parent) = std::path::Path::new(log_path).parent() {
                if let Err(e) = tokio::fs::create_dir_all(parent).await {
                    error!("Failed to create audit log directory: {}", e);
                    return;
                }
            }

            // 序列化事件
            let log_lines: Vec<String> = events.iter()
                .map(|event| {
                    match serde_json::to_string(event) {
                        Ok(json) => json,
                        Err(e) => {
                            error!("Failed to serialize audit event: {}", e);
                            format!("{{\"error\": \"serialization_failed\", \"id\": \"{}\"}}", event.id)
                        }
                    }
                })
                .collect();

            let content = log_lines.join("\n") + "\n";

            // 追加到日志文件
            match tokio::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(log_path)
                .await
            {
                Ok(mut file) => {
                    if let Err(e) = tokio::io::AsyncWriteExt::write_all(&mut file, content.as_bytes()).await {
                        error!("Failed to write audit log: {}", e);
                    }
                }
                Err(e) => {
                    error!("Failed to open audit log file: {}", e);
                }
            }
        }
    }

    /// 获取审计统计
    pub async fn get_stats(&self) -> AuditStats {
        let events = self.events.lock().await;

        let mut type_counts = HashMap::new();
        let mut severity_counts = HashMap::new();

        for event in events.iter() {
            *type_counts.entry(format!("{:?}", event.event_type)).or_insert(0) += 1;
            *severity_counts.entry(format!("{:?}", event.severity)).or_insert(0) += 1;
        }

        AuditStats {
            total_events: events.len(),
            type_counts,
            severity_counts,
            recent_events: events.iter().rev().take(10).cloned().collect(),
        }
    }

    /// 清理旧的审计事件
    pub async fn cleanup_old_events(&self, max_age_hours: i64) {
        let cutoff = chrono::Utc::now() - chrono::Duration::hours(max_age_hours);

        let mut events = self.events.lock().await;
        events.retain(|event| event.timestamp > cutoff);

        info!("Cleaned up old audit events, {} events remaining", events.len());
    }
}

impl AsRef<str> for AuditEventType {
    fn as_ref(&self) -> &str {
        match self {
            AuditEventType::Authentication => "authentication",
            AuditEventType::Authorization => "authorization",
            AuditEventType::AccessControl => "access_control",
            AuditEventType::DataAccess => "data_access",
            AuditEventType::ConfigurationChange => "config_change",
            AuditEventType::SystemEvent => "system_event",
            AuditEventType::SecurityEvent => "security_event",
            AuditEventType::ErrorEvent => "error_event",
        }
    }
}

impl Default for AuditLogger {
    fn default() -> Self {
        Self::new(AuditConfig::default())
    }
}

/// 审计统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditStats {
    /// 总事件数
    pub total_events: usize,
    /// 按类型统计
    pub type_counts: HashMap<String, usize>,
    /// 按严重程度统计
    pub severity_counts: HashMap<String, usize>,
    /// 最近事件
    pub recent_events: Vec<AuditEvent>,
}

/// 审计中间件
pub mod middleware {
    use axum::{
        extract::{ConnectInfo, Request},
        http::{header, StatusCode},
        middleware::Next,
        response::Response,
    };
    use std::net::SocketAddr;
    use std::sync::Arc;

    use super::AuditLogger;

    /// 审计中间件
    pub async fn audit_middleware(
        ConnectInfo(addr): ConnectInfo<SocketAddr>,
        audit_logger: Arc<AuditLogger>,
        request: Request,
        next: Next,
    ) -> Response {
        let start_time = std::time::Instant::now();
        let method = request.method().to_string();
        let uri = request.uri().to_string();

        // 提取用户信息（如果有的话）
        let user_id = request.headers()
            .get("X-User-ID")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());

        let response = next.run(request).await;
        let duration = start_time.elapsed();

        // 记录API访问
        audit_logger.log_api_access(
            user_id.as_deref(),
            &method,
            &uri,
            response.status().as_u16(),
            Some(&addr.ip().to_string()),
        ).await;

        // 添加审计相关的响应头
        let mut response = response;
        let headers = response.headers_mut();
        headers.insert(
            "X-Request-ID",
            uuid::Uuid::new_v4().to_string().parse().unwrap(),
        );
        headers.insert(
            "X-Response-Time",
            format!("{}ms", duration.as_millis()).parse().unwrap(),
        );

        response
    }
}
