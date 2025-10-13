//! 配置结构体定义

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 应用程序配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// 服务器配置
    pub server: ServerSettings,
    /// 容器配置
    pub container: ContainerSettings,
    /// FFI配置
    pub ffi: FfiSettings,
    /// 日志配置
    pub logging: LoggingSettings,
    /// 安全配置
    pub security: SecuritySettings,
}

/// 服务器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerSettings {
    /// 监听主机
    pub host: String,
    /// 监听端口
    pub port: u16,
    /// 工作线程数
    pub workers: Option<usize>,
    /// 请求超时时间（秒）
    pub request_timeout_seconds: u64,
    /// 任务队列大小
    pub task_queue_size: usize,
}

/// 容器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerSettings {
    /// Youki可执行文件路径
    pub youki_path: PathBuf,
    /// 容器镜像目录
    pub image_dir: PathBuf,
    /// 容器运行时目录
    pub runtime_dir: PathBuf,
    /// 默认容器配置
    pub default_config: ContainerConfig,
    /// 容器资源限制
    pub resource_limits: ResourceLimits,
}

/// 容器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerConfig {
    /// 是否使用rootless模式
    pub rootless: bool,
    /// Seccomp配置文件路径
    pub seccomp_profile: Option<PathBuf>,
    /// AppArmor配置文件路径
    pub apparmor_profile: Option<PathBuf>,
    /// 网络隔离
    pub network_isolation: bool,
}

/// 资源限制
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// 默认CPU核心数
    pub default_cpu_cores: f64,
    /// 默认内存大小（MB）
    pub default_memory_mb: u64,
    /// 最大CPU核心数
    pub max_cpu_cores: f64,
    /// 最大内存大小（MB）
    pub max_memory_mb: u64,
    /// 容器超时时间（秒）
    pub container_timeout_seconds: u64,
}

/// FFI配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FfiSettings {
    /// C++库路径
    pub cpp_library_path: PathBuf,
    /// 桥接头文件路径
    pub bridge_header_path: PathBuf,
    /// 桥接实现文件路径
    pub bridge_impl_path: PathBuf,
    /// 内存池大小
    pub memory_pool_size: usize,
}

/// 日志配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingSettings {
    /// 日志级别
    pub level: String,
    /// 日志输出格式
    pub format: LogFormat,
    /// 日志文件路径
    pub file_path: Option<PathBuf>,
    /// 是否输出到控制台
    pub console_output: bool,
}

/// 日志格式
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    /// JSON格式
    Json,
    /// 文本格式
    Text,
}

/// 安全配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecuritySettings {
    /// TLS证书路径
    pub tls_cert_path: Option<PathBuf>,
    /// TLS私钥路径
    pub tls_key_path: Option<PathBuf>,
    /// API密钥
    pub api_key: Option<String>,
    /// 允许的CORS源
    pub allowed_origins: Vec<String>,
    /// 请求频率限制
    pub rate_limit: RateLimit,
}

/// 频率限制
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimit {
    /// 每分钟请求数
    pub requests_per_minute: u32,
    /// 突发请求数
    pub burst_size: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            server: ServerSettings::default(),
            container: ContainerSettings::default(),
            ffi: FfiSettings::default(),
            logging: LoggingSettings::default(),
            security: SecuritySettings::default(),
        }
    }
}

impl Default for ServerSettings {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 3000,
            workers: None,
            request_timeout_seconds: 300,
            task_queue_size: 1000,
        }
    }
}

impl Default for ContainerSettings {
    fn default() -> Self {
        Self {
            youki_path: PathBuf::from("youki"),
            image_dir: PathBuf::from("./images"),
            runtime_dir: PathBuf::from("./runtime"),
            default_config: ContainerConfig::default(),
            resource_limits: ResourceLimits::default(),
        }
    }
}

impl Default for ContainerConfig {
    fn default() -> Self {
        Self {
            rootless: true,
            seccomp_profile: None,
            apparmor_profile: None,
            network_isolation: true,
        }
    }
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            default_cpu_cores: 1.0,
            default_memory_mb: 512,
            max_cpu_cores: 4.0,
            max_memory_mb: 2048,
            container_timeout_seconds: 300,
        }
    }
}

impl Default for FfiSettings {
    fn default() -> Self {
        Self {
            cpp_library_path: PathBuf::from("./lib"),
            bridge_header_path: PathBuf::from("./src/ffi/cpp/bridge.h"),
            bridge_impl_path: PathBuf::from("./src/ffi/cpp/bridge.cc"),
            memory_pool_size: 1024 * 1024, // 1MB
        }
    }
}

impl Default for LoggingSettings {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            format: LogFormat::Text,
            file_path: None,
            console_output: true,
        }
    }
}

impl Default for SecuritySettings {
    fn default() -> Self {
        Self {
            tls_cert_path: None,
            tls_key_path: None,
            api_key: None,
            allowed_origins: vec!["*".to_string()],
            rate_limit: RateLimit::default(),
        }
    }
}

impl Default for RateLimit {
    fn default() -> Self {
        Self {
            requests_per_minute: 60,
            burst_size: 10,
        }
    }
}
