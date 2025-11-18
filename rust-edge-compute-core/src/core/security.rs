//! 安全模块
//!
//! 提供认证、授权、输入验证和访问控制功能

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use serde::{Deserialize, Serialize};

/// 安全配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// 启用认证
    pub enable_authentication: bool,
    /// JWT密钥
    pub jwt_secret: Option<String>,
    /// JWT过期时间（秒）
    pub jwt_expiration_seconds: u64,
    /// 启用授权
    pub enable_authorization: bool,
    /// 默认角色
    pub default_role: String,
    /// 启用速率限制
    pub enable_rate_limiting: bool,
    /// 每分钟请求限制
    pub requests_per_minute: u32,
    /// 突发请求限制
    pub burst_size: u32,
    /// 启用CORS
    pub enable_cors: bool,
    /// 允许的源
    pub allowed_origins: Vec<String>,
    /// 启用输入验证
    pub enable_input_validation: bool,
    /// 启用HTTPS重定向
    pub enable_https_redirect: bool,
    /// 启用安全头
    pub enable_security_headers: bool,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enable_authentication: false,
            jwt_secret: None,
            jwt_expiration_seconds: 3600, // 1小时
            enable_authorization: false,
            default_role: "user".to_string(),
            enable_rate_limiting: true,
            requests_per_minute: 60,
            burst_size: 10,
            enable_cors: true,
            allowed_origins: vec!["*".to_string()],
            enable_input_validation: true,
            enable_https_redirect: false,
            enable_security_headers: true,
        }
    }
}

/// 用户会话
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSession {
    /// 用户ID
    pub user_id: String,
    /// 用户名
    pub username: String,
    /// 角色列表
    pub roles: Vec<String>,
    /// 权限列表
    pub permissions: Vec<String>,
    /// 会话创建时间
    pub created_at: std::time::SystemTime,
    /// 会话过期时间
    pub expires_at: std::time::SystemTime,
}

/// 安全上下文
#[derive(Debug, Clone)]
pub struct SecurityContext {
    /// 当前用户会话
    pub session: Option<UserSession>,
    /// 请求IP地址
    pub client_ip: Option<String>,
    /// 用户代理
    pub user_agent: Option<String>,
    /// 请求路径
    pub path: String,
    /// 请求方法
    pub method: String,
}

/// 速率限制器
pub struct RateLimiter {
    /// 请求记录：IP -> (请求次数, 重置时间)
    records: Arc<Mutex<HashMap<String, (u32, std::time::Instant)>>>,
    /// 每分钟请求限制
    requests_per_minute: u32,
    /// 突发请求限制
    burst_size: u32,
}

impl RateLimiter {
    /// 创建新的速率限制器
    pub fn new(requests_per_minute: u32, burst_size: u32) -> Self {
        Self {
            records: Arc::new(Mutex::new(HashMap::new())),
            requests_per_minute,
            burst_size,
        }
    }

    /// 检查请求是否被允许
    pub async fn check_rate_limit(&self, client_ip: &str) -> Result<bool, SecurityError> {
        let mut records = self.records.lock().await;
        let now = std::time::Instant::now();

        let entry = records.entry(client_ip.to_string()).or_insert((0, now));

        // 检查是否需要重置计数器（每分钟重置）
        if now.duration_since(entry.1).as_secs() >= 60 {
            entry.0 = 0;
            entry.1 = now;
        }

        // 检查是否超过限制
        if entry.0 >= self.requests_per_minute {
            return Ok(false); // 超出限制
        }

        // 增加请求计数
        entry.0 += 1;

        Ok(true)
    }

    /// 获取客户端的当前请求统计
    pub async fn get_client_stats(&self, client_ip: &str) -> (u32, std::time::Duration) {
        let records = self.records.lock().await;

        if let Some((count, reset_time)) = records.get(client_ip) {
            let remaining_time = std::time::Instant::now().duration_since(*reset_time);
            let time_to_reset = if remaining_time.as_secs() < 60 {
                std::time::Duration::from_secs(60 - remaining_time.as_secs())
            } else {
                std::time::Duration::from_secs(0)
            };

            (*count, time_to_reset)
        } else {
            (0, std::time::Duration::from_secs(60))
        }
    }

    /// 清理过期的记录
    pub async fn cleanup_expired(&self) {
        let mut records = self.records.lock().await;
        let now = std::time::Instant::now();

        records.retain(|_, (_, reset_time)| {
            now.duration_since(*reset_time).as_secs() < 120 // 保留2分钟的记录
        });
    }
}

/// 输入验证器
pub struct InputValidator;

impl InputValidator {
    /// 验证算法名称
    pub fn validate_algorithm_name(name: &str) -> Result<(), SecurityError> {
        if name.is_empty() {
            return Err(SecurityError::ValidationError {
                field: "algorithm".to_string(),
                message: "Algorithm name cannot be empty".to_string(),
            });
        }

        if name.len() > 100 {
            return Err(SecurityError::ValidationError {
                field: "algorithm".to_string(),
                message: "Algorithm name too long".to_string(),
            });
        }

        // 只允许字母、数字、下划线和连字符
        if !name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
            return Err(SecurityError::ValidationError {
                field: "algorithm".to_string(),
                message: "Algorithm name contains invalid characters".to_string(),
            });
        }

        Ok(())
    }

    /// 验证JSON参数
    pub fn validate_json_params(params: &serde_json::Value) -> Result<(), SecurityError> {
        // 检查参数大小
        let params_str = serde_json::to_string(params)
            .map_err(|_| SecurityError::ValidationError {
                field: "parameters".to_string(),
                message: "Failed to serialize parameters".to_string(),
            })?;

        if params_str.len() > 1024 * 1024 { // 1MB限制
            return Err(SecurityError::ValidationError {
                field: "parameters".to_string(),
                message: "Parameters too large".to_string(),
            });
        }

        // 检查嵌套深度
        if Self::check_json_depth(params, 0) > 10 {
            return Err(SecurityError::ValidationError {
                field: "parameters".to_string(),
                message: "Parameters nesting too deep".to_string(),
            });
        }

        Ok(())
    }

    /// 检查JSON嵌套深度
    fn check_json_depth(value: &serde_json::Value, current_depth: usize) -> usize {
        match value {
            serde_json::Value::Object(obj) => {
                let mut max_depth = current_depth;
                for v in obj.values() {
                    max_depth = max_depth.max(Self::check_json_depth(v, current_depth + 1));
                }
                max_depth
            }
            serde_json::Value::Array(arr) => {
                let mut max_depth = current_depth;
                for v in arr {
                    max_depth = max_depth.max(Self::check_json_depth(v, current_depth + 1));
                }
                max_depth
            }
            _ => current_depth,
        }
    }

    /// 验证任务ID
    pub fn validate_task_id(task_id: &str) -> Result<(), SecurityError> {
        if task_id.is_empty() {
            return Err(SecurityError::ValidationError {
                field: "task_id".to_string(),
                message: "Task ID cannot be empty".to_string(),
            });
        }

        if task_id.len() > 256 {
            return Err(SecurityError::ValidationError {
                field: "task_id".to_string(),
                message: "Task ID too long".to_string(),
            });
        }

        // 检查是否包含危险字符
        if task_id.contains("..") || task_id.contains("/") || task_id.contains("\\") {
            return Err(SecurityError::ValidationError {
                field: "task_id".to_string(),
                message: "Task ID contains invalid characters".to_string(),
            });
        }

        Ok(())
    }
}

/// 权限检查器
pub struct PermissionChecker;

impl PermissionChecker {
    /// 检查用户是否有执行算法的权限
    pub fn check_algorithm_permission(
        session: &Option<UserSession>,
        algorithm: &str,
        config: &SecurityConfig,
    ) -> Result<(), SecurityError> {
        if !config.enable_authorization {
            return Ok(()); // 禁用授权时允许所有操作
        }

        let session = session.as_ref().ok_or_else(|| {
            SecurityError::AuthorizationError {
                message: "No active session".to_string(),
                permission: format!("execute:{}", algorithm),
                resource: algorithm.to_string(),
            }
        })?;

        // 检查用户是否有执行该算法的权限
        let required_permission = format!("execute:{}", algorithm);
        if !session.permissions.contains(&required_permission) {
            return Err(SecurityError::AuthorizationError {
                message: format!("User {} does not have permission to execute {}", session.username, algorithm),
                permission: required_permission,
                resource: algorithm.to_string(),
            });
        }

        Ok(())
    }

    /// 检查用户是否有管理权限
    pub fn check_admin_permission(session: &Option<UserSession>) -> Result<(), SecurityError> {
        let session = session.as_ref().ok_or_else(|| {
            SecurityError::AuthorizationError {
                message: "No active session".to_string(),
                permission: "admin:*".to_string(),
                resource: "*".to_string(),
            }
        })?;

        if !session.permissions.contains(&"admin:*".to_string()) {
            return Err(SecurityError::AuthorizationError {
                message: format!("User {} does not have admin permissions", session.username),
                permission: "admin:*".to_string(),
                resource: "*".to_string(),
            });
        }

        Ok(())
    }
}

/// 安全中间件
pub struct SecurityMiddleware {
    config: SecurityConfig,
    rate_limiter: Option<Arc<RateLimiter>>,
}

impl SecurityMiddleware {
    /// 创建新的安全中间件
    pub fn new(config: SecurityConfig) -> Self {
        let rate_limiter = if config.enable_rate_limiting {
            Some(Arc::new(RateLimiter::new(
                config.requests_per_minute,
                config.burst_size,
            )))
        } else {
            None
        };

        Self {
            config,
            rate_limiter,
        }
    }

    /// 处理安全检查
    pub async fn process_request(
        &self,
        context: &SecurityContext,
    ) -> Result<(), SecurityError> {
        // 速率限制检查
        if let Some(ref limiter) = self.rate_limiter {
            if let Some(client_ip) = &context.client_ip {
                if !limiter.check_rate_limit(client_ip).await? {
                    return Err(SecurityError::RateLimitExceeded {
                        client_ip: client_ip.clone(),
                        limit: self.config.requests_per_minute,
                    });
                }
            }
        }

        // 输入验证
        if self.config.enable_input_validation {
            Self::validate_request(context)?;
        }

        Ok(())
    }

    /// 验证请求
    fn validate_request(context: &SecurityContext) -> Result<(), SecurityError> {
        // 验证路径安全性
        if context.path.contains("..") || context.path.contains("//") {
            return Err(SecurityError::ValidationError {
                field: "path".to_string(),
                message: "Invalid path format".to_string(),
            });
        }

        // 验证HTTP方法
        let allowed_methods = ["GET", "POST", "PUT", "DELETE"];
        if !allowed_methods.contains(&context.method.as_str()) {
            return Err(SecurityError::ValidationError {
                field: "method".to_string(),
                message: "Unsupported HTTP method".to_string(),
            });
        }

        Ok(())
    }

    /// 获取速率限制器统计
    pub async fn get_rate_limit_stats(&self, client_ip: &str) -> Option<(u32, std::time::Duration)> {
        if let Some(ref limiter) = self.rate_limiter {
            Some(limiter.get_client_stats(client_ip).await)
        } else {
            None
        }
    }
}

/// 安全错误类型
#[derive(Debug, thiserror::Error)]
pub enum SecurityError {
    #[error("Authentication error: {message}")]
    AuthenticationError { message: String },

    #[error("Authorization error: permission '{permission}' denied for resource '{resource}' - {message}")]
    AuthorizationError {
        message: String,
        permission: String,
        resource: String,
    },

    #[error("Validation error in field '{field}': {message}")]
    ValidationError {
        field: String,
        message: String,
    },

    #[error("Rate limit exceeded for client {client_ip}, limit: {limit} requests/minute")]
    RateLimitExceeded {
        client_ip: String,
        limit: u32,
    },

    #[error("Security violation: {message}")]
    SecurityViolation { message: String },

    #[error("Session error: {message}")]
    SessionError { message: String },
}

/// 安全工具函数
pub mod utils {
    use ring::rand::{SecureRandom, SystemRandom};
    use ring::pbkdf2;
    use std::num::NonZeroU32;

    /// 生成安全的随机字符串
    pub fn generate_secure_token(length: usize) -> String {
        let rng = SystemRandom::new();
        let mut bytes = vec![0u8; length];
        rng.fill(&mut bytes).expect("Failed to generate random bytes");

        base64::encode(bytes)
    }

    /// 哈希密码
    pub fn hash_password(password: &str, salt: &[u8]) -> Vec<u8> {
        let mut hash = vec![0u8; 32]; // SHA256输出长度
        let iterations = NonZeroU32::new(100_000).unwrap();

        pbkdf2::derive(
            pbkdf2::PBKDF2_HMAC_SHA256,
            iterations,
            salt,
            password.as_bytes(),
            &mut hash,
        );

        hash
    }

    /// 验证密码哈希
    pub fn verify_password(password: &str, salt: &[u8], expected_hash: &[u8]) -> bool {
        let iterations = NonZeroU32::new(100_000).unwrap();

        pbkdf2::verify(
            pbkdf2::PBKDF2_HMAC_SHA256,
            iterations,
            salt,
            password.as_bytes(),
            expected_hash,
        ).is_ok()
    }

    /// 清理敏感信息
    pub fn sanitize_input(input: &str) -> String {
        // 移除或转义危险字符
        input
            .replace("<", "&lt;")
            .replace(">", "&gt;")
            .replace("\"", "&quot;")
            .replace("'", "&#x27;")
            .replace("&", "&amp;")
    }
}
