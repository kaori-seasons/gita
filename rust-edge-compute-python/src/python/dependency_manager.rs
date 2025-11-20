//! Python依赖管理器
//!
//! 提供Python包依赖的安装、缓存和管理功能

use rust_edge_compute_core::core::error::{Result, EdgeComputeError};
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

impl DependencyManager {
    /// 创建新的依赖管理器
    pub fn new(config: DependencyManagerConfig) -> Result<Self> {
        // 创建缓存目录
        if let Some(parent) = config.cache_dir.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| EdgeComputeError::Config {
                    message: format!("Failed to create cache directory: {}", e),
                    source: Some("dependency-manager".to_string()),
                })?;
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
            .map_err(|e| EdgeComputeError::Config {
                message: format!("Failed to create virtual environment: {}", e),
                source: Some("dependency-manager".to_string()),
            })?;
        
        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Err(EdgeComputeError::Config {
                message: format!("Failed to create virtual environment: {}", error_msg),
                source: Some("dependency-manager".to_string()),
            });
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
        let mut cmd = if self.config.use_venv {
            if let Some(ref venv_dir) = self.config.venv_dir {
                let pip_path = venv_dir.join("bin").join("pip");
                if cfg!(windows) {
                    venv_dir.join("Scripts").join("pip.exe")
                } else {
                    pip_path
                }
            } else {
                PathBuf::from("pip")
            }
        } else {
            PathBuf::from("pip")
        };
        
        // 构建包规格
        let package_spec = if let Some(version) = version {
            format!("{}=={}", package, version)
        } else {
            package.to_string()
        };
        
        // 执行安装
        let install_result = tokio::time::timeout(
            Duration::from_secs(self.config.install_timeout_seconds),
            self.run_pip_install(&cmd, &package_spec),
        ).await;
        
        match install_result {
            Ok(Ok(())) => {
                // 记录已安装的依赖
                let mut deps = self.installed_dependencies.write().await;
                deps.insert(package.to_string(), InstalledDependency {
                    name: package.to_string(),
                    version: version.unwrap_or("latest").to_string(),
                    installed_at: SystemTime::now(),
                    usage_count: 1,
                });
                
                // 更新统计
                let mut stats = self.install_stats.write().await;
                stats.successful_installs += 1;
                
                tracing::info!("Successfully installed dependency: {}", package);
                Ok(())
            }
            Ok(Err(e)) => {
                // 更新统计
                let mut stats = self.install_stats.write().await;
                stats.failed_installs += 1;
                
                tracing::error!("Failed to install dependency {}: {}", package, e);
                Err(e)
            }
            Err(_) => {
                // 超时
                let mut stats = self.install_stats.write().await;
                stats.failed_installs += 1;
                
                let error = EdgeComputeError::AlgorithmExecution {
                    message: format!("Dependency installation timeout: {}", package),
                    algorithm: Some("pip_install".to_string()),
                    input_size: Some(package.len()),
                };
                tracing::error!("{}", error);
                Err(error)
            }
        }
    }
    
    /// 运行pip安装命令
    async fn run_pip_install(&self, pip_path: &Path, package_spec: &str) -> Result<()> {
        let pip_str = pip_path.to_string_lossy().to_string();
        
        tokio::task::spawn_blocking(move || {
            let output = Command::new(&pip_str)
                .arg("install")
                .arg("--cache-dir")
                .arg("./python_cache")
                .arg(package_spec)
                .output()
                .map_err(|e| EdgeComputeError::AlgorithmExecution {
                    message: format!("Failed to execute pip install: {}", e),
                    algorithm: Some("pip_install".to_string()),
                    input_size: Some(package_spec.len()),
                })?;
            
            if !output.status.success() {
                let error_msg = String::from_utf8_lossy(&output.stderr);
                return Err(EdgeComputeError::AlgorithmExecution {
                    message: format!("pip install failed: {}", error_msg),
                    algorithm: Some("pip_install".to_string()),
                    input_size: Some(package_spec.len()),
                });
            }
            
            Ok(())
        })
        .await
        .map_err(|e| EdgeComputeError::AlgorithmExecution {
            message: format!("Task join error: {}", e),
            algorithm: Some("pip_install".to_string()),
            input_size: Some(package_spec.len()),
        })?
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
        let package = package.to_string();
        
        tokio::task::spawn_blocking(move || {
            let output = Command::new(&pip_str)
                .arg("uninstall")
                .arg("-y")
                .arg(&package)
                .output()
                .map_err(|e| EdgeComputeError::AlgorithmExecution {
                    message: format!("Failed to execute pip uninstall: {}", e),
                    algorithm: Some("pip_uninstall".to_string()),
                    input_size: Some(package.len()),
                })?;
            
            if !output.status.success() {
                let error_msg = String::from_utf8_lossy(&output.stderr);
                return Err(EdgeComputeError::AlgorithmExecution {
                    message: format!("pip uninstall failed: {}", error_msg),
                    algorithm: Some("pip_uninstall".to_string()),
                    input_size: Some(package.len()),
                });
            }
            
            Ok(())
        })
        .await
        .map_err(|e| EdgeComputeError::AlgorithmExecution {
            message: format!("Task join error: {}", e),
            algorithm: Some("pip_uninstall".to_string()),
            input_size: Some(package.len()),
        })?;
        
        // 从缓存中移除
        let mut deps = self.installed_dependencies.write().await;
        deps.remove(&package);
        
        tracing::info!("Successfully uninstalled dependency: {}", package);
        Ok(())
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
                .map_err(|e| EdgeComputeError::Config {
                    message: format!("Failed to clear cache directory: {}", e),
                    source: Some("dependency-manager".to_string()),
                })?;
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

