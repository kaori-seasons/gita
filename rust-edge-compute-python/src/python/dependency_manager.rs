//! Python依赖管理器
//!
//! 提供Python包依赖的安装、缓存和管理功能

use rust_edge_compute_core::core::error::EdgeComputeError;
use rust_edge_compute_core::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::process::Command;
use std::time::{Duration, SystemTime};

/// 依赖管理器配置
#[derive(Debug, Clone)]
pub struct DependencyManagerConfig {
    /// 依赖管理器类型（pip, conda等）
    pub manager: String,
    /// 依赖缓存目录
    pub cache_dir: PathBuf,
    /// 是否自动安装依赖
    pub auto_install: bool,
    /// 安装超时时间（秒）
    pub install_timeout_seconds: u64,
    /// 是否使用虚拟环境
    pub use_venv: bool,
    /// 虚拟环境目录
    pub venv_dir: Option<PathBuf>,
}

impl Default for DependencyManagerConfig {
    fn default() -> Self {
        Self {
            manager: "pip".to_string(),
            cache_dir: PathBuf::from("./python_cache"),
            auto_install: true,
            install_timeout_seconds: 300,
            use_venv: true,
            venv_dir: Some(PathBuf::from("./python_venv")),
        }
    }
}

/// 已安装的依赖信息
#[derive(Debug, Clone)]
struct InstalledDependency {
    /// 包名
    name: String,
    /// 版本
    version: String,
    /// 安装时间
    installed_at: SystemTime,
    /// 使用次数
    usage_count: u64,
}

/// 依赖管理器
pub struct DependencyManager {
    /// 配置
    config: DependencyManagerConfig,
    /// 已安装的依赖缓存
    installed_dependencies: Arc<RwLock<HashMap<String, InstalledDependency>>>,
    /// 安装统计
    install_stats: Arc<RwLock<InstallStats>>,
}

/// 安装统计
#[derive(Debug, Clone, Default)]
pub struct InstallStats {
    /// 总安装次数
    pub total_installs: u64,
    /// 成功安装次数
    pub successful_installs: u64,
    /// 失败安装次数
    pub failed_installs: u64,
    /// 缓存命中次数
    pub cache_hits: u64,
}

/// 创建配置错误
fn config_error(message: String, _source: Option<String>) -> Box<dyn std::error::Error + Send + Sync> {
    Box::new(EdgeComputeError(format!("Configuration error: {}", message)))
}

/// 创建算法执行错误
fn algorithm_execution_error(message: String, _algorithm: Option<String>, _input_size: Option<usize>) -> Box<dyn std::error::Error + Send + Sync> {
    Box::new(EdgeComputeError(format!("Algorithm execution error: {}", message)))
}

/// 创建验证错误
fn validation_error(message: String, _field: Option<String>, _value: Option<String>) -> Box<dyn std::error::Error + Send + Sync> {
    Box::new(EdgeComputeError(format!("Validation error: {}", message)))
}

impl DependencyManager {
    /// 创建新的依赖管理器
    pub fn new(config: DependencyManagerConfig) -> Result<Self> {
        // 创建缓存目录
        if let Some(parent) = config.cache_dir.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| config_error(
                    format!("Failed to create cache directory: {}", e),
                    Some("dependency-manager".to_string()),
                ))?;
        }
        
        // 创建虚拟环境（如果启用）
        if config.use_venv {
            if let Some(ref venv_dir) = config.venv_dir {
                Self::create_venv(venv_dir)?;
            }
        }
        
        Ok(Self {
            config,
            installed_dependencies: Arc::new(RwLock::new(HashMap::new())),
            install_stats: Arc::new(RwLock::new(InstallStats::default())),
        })
    }
    
    /// 创建Python虚拟环境
    fn create_venv(venv_dir: &Path) -> Result<()> {
        if venv_dir.exists() {
            return Ok(());
        }
        
        tracing::info!("Creating Python virtual environment at: {:?}", venv_dir);
        
        // 使用python -m venv创建虚拟环境
        let output = Command::new("python")
            .arg("-m")
            .arg("venv")
            .arg(venv_dir)
            .output()
            .map_err(|e| config_error(
                format!("Failed to create virtual environment: {}", e),
                Some("dependency-manager".to_string()),
            ))?;
        
        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Err(config_error(
                format!("Failed to create virtual environment: {}", error_msg),
                Some("dependency-manager".to_string()),
            ));
        }
        
        tracing::info!("Python virtual environment created successfully");
        Ok(())
    }
    
    /// 安装依赖
    pub async fn install_dependency(&self, package: &str, version: Option<&str>) -> Result<()> {
        // 检查是否已安装
        {
            let deps = self.installed_dependencies.read().await;
            if deps.contains_key(package) {
                // 更新使用计数
                let mut stats = self.install_stats.write().await;
                stats.cache_hits += 1;
                tracing::debug!("Dependency {} already installed, using cache", package);
                return Ok(());
            }
        }
        
        tracing::info!("Installing Python dependency: {} {}", package, 
            version.map(|v| format!("=={}", v)).unwrap_or_default());
        
        // 更新统计
        {
            let mut stats = self.install_stats.write().await;
            stats.total_installs += 1;
        }
        
        // 构建pip安装命令
        let package_spec = if let Some(v) = version {
            format!("{}=={}", package, v)
        } else {
            package.to_string()
        };
        
        // 确定pip路径
        let pip_path = if self.config.use_venv {
            if let Some(ref venv_dir) = self.config.venv_dir {
                if cfg!(windows) {
                    venv_dir.join("Scripts").join("pip.exe")
                } else {
                    venv_dir.join("bin").join("pip")
                }
            } else {
                PathBuf::from("pip")
            }
        } else {
            PathBuf::from("pip")
        };
        
        // 执行安装
        self.run_pip_install(&pip_path, &package_spec).await?;
        
        // 更新缓存
        let mut deps = self.installed_dependencies.write().await;
        deps.insert(package.to_string(), InstalledDependency {
            name: package.to_string(),
            version: version.unwrap_or("latest").to_string(),
            installed_at: SystemTime::now(),
            usage_count: 1,
        });

        tracing::info!("Successfully installed dependency: {}", package_spec);
        Ok(())
    }
    
    /// 运行pip安装命令
    async fn run_pip_install(&self, pip_path: &Path, package_spec: &str) -> Result<()> {
        let pip_str = pip_path.to_string_lossy().to_string();
        let package_spec_owned = package_spec.to_string(); // 克隆字符串以避免生命周期问题
        
        let result = tokio::task::spawn_blocking(move || {
            let output = Command::new(&pip_str)
                .arg("install")
                .arg("--cache-dir")
                .arg("./python_cache")
                .arg(&package_spec_owned) // 使用克隆的字符串
                .output()
                .map_err(|e| Box::new(EdgeComputeError(format!("Algorithm execution error: Failed to execute pip install: {}", e))))?;
            
            if !output.status.success() {
                let error_msg = String::from_utf8_lossy(&output.stderr);
                return Err(Box::new(EdgeComputeError(format!("Algorithm execution error: pip install failed: {}", error_msg))));
            }
            
            Ok(())
        })
        .await;
        
        // 处理任务执行结果
        match result {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(e) => Err(Box::new(EdgeComputeError(format!("Algorithm execution error: Task join error: {}", e))))
        }
    }
    
    /// 安装多个依赖
    pub async fn install_dependencies(&self, packages: &[String]) -> Result<()> {
        for package in packages {
            // 解析包名和版本（格式：package==version 或 package）
            let (name, version) = if let Some(pos) = package.find("==") {
                let (name, version) = package.split_at(pos);
                (name, Some(&version[2..]))
            } else {
                (package.as_str(), None)
            };
            
            self.install_dependency(name, version).await?;
        }
        
        Ok(())
    }
    
    /// 卸载依赖
    pub async fn uninstall_dependency(&self, package: &str) -> Result<()> {
        tracing::info!("Uninstalling Python dependency: {}", package);
        
        let pip_path = if self.config.use_venv {
            if let Some(ref venv_dir) = self.config.venv_dir {
                if cfg!(windows) {
                    venv_dir.join("Scripts").join("pip.exe")
                } else {
                    venv_dir.join("bin").join("pip")
                }
            } else {
                PathBuf::from("pip")
            }
        } else {
            PathBuf::from("pip")
        };
        
        let pip_str = pip_path.to_string_lossy().to_string();
        let package_name = package.to_string();  // 克隆package以避免移动问题
        
        let result = tokio::task::spawn_blocking(move || {
            let output = Command::new(&pip_str)
                .arg("uninstall")
                .arg("-y")
                .arg(&package_name)  // 使用克隆的变量
                .output()
                .map_err(|e| Box::new(EdgeComputeError(format!("Algorithm execution error: Failed to execute pip uninstall: {}", e))))?;
            
            if !output.status.success() {
                let error_msg = String::from_utf8_lossy(&output.stderr);
                return Err(Box::new(EdgeComputeError(format!("Algorithm execution error: pip uninstall failed: {}", error_msg))));
            }
            
            Ok(())
        })
        .await;
        
        // 处理任务执行结果
        match result {
            Ok(Ok(())) => {
                // 从缓存中移除
                let mut deps = self.installed_dependencies.write().await;
                deps.remove(package);
                
                tracing::info!("Successfully uninstalled dependency: {}", package);
                Ok(())
            },
            Ok(Err(e)) => Err(e),
            Err(e) => Err(Box::new(EdgeComputeError(format!("Algorithm execution error: Task join error: {}", e))))
        }
    }
    
    /// 列出已安装的依赖
    pub async fn list_installed(&self) -> Vec<String> {
        let deps = self.installed_dependencies.read().await;
        deps.keys().cloned().collect()
    }
    
    /// 获取安装统计
    pub async fn get_stats(&self) -> InstallStats {
        self.install_stats.read().await.clone()
    }
    
    /// 清理缓存
    pub async fn clear_cache(&self) -> Result<()> {
        tracing::info!("Clearing dependency cache");
        
        let mut deps = self.installed_dependencies.write().await;
        deps.clear();
        
        // 清理缓存目录
        if self.config.cache_dir.exists() {
            std::fs::remove_dir_all(&self.config.cache_dir)
                .map_err(|e| Box::new(EdgeComputeError(format!("Configuration error: Failed to clear cache directory: {}", e))))?;
        }
        
        tracing::info!("Cache cleared successfully");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_dependency_manager_creation() {
        let config = DependencyManagerConfig {
            cache_dir: PathBuf::from("./test_cache"),
            use_venv: false, // 测试时不创建虚拟环境
            venv_dir: None,
            ..Default::default()
        };
        
        let manager = DependencyManager::new(config);
        assert!(manager.is_ok());
    }
    
    #[tokio::test]
    async fn test_dependency_manager_list() {
        let config = DependencyManagerConfig {
            cache_dir: PathBuf::from("./test_cache"),
            use_venv: false,
            venv_dir: None,
            ..Default::default()
        };
        
        let manager = DependencyManager::new(config).unwrap();
        let installed = manager.list_installed().await;
        assert_eq!(installed.len(), 0);
    }
}

