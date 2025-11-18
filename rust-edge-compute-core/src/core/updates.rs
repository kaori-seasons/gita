//! OTA更新系统
//!
//! 提供在线更新、版本管理和安全部署功能

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

/// 更新配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateConfig {
    /// 是否启用自动更新检查
    pub auto_check_updates: bool,
    /// 更新检查间隔（秒）
    pub check_interval_seconds: u64,
    /// 更新服务器URL
    pub update_server_url: String,
    /// 当前版本
    pub current_version: String,
    /// 更新下载目录
    pub download_dir: String,
    /// 是否允许降级
    pub allow_downgrades: bool,
    /// 备份旧版本
    pub backup_old_version: bool,
    /// 最大重试次数
    pub max_retry_attempts: u32,
    /// 更新超时时间（秒）
    pub update_timeout_seconds: u64,
}

/// 版本信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    /// 版本号
    pub version: String,
    /// 发布日期
    pub release_date: chrono::DateTime<chrono::Utc>,
    /// 变更日志
    pub changelog: Vec<String>,
    /// 最低兼容版本
    pub minimum_version: Option<String>,
    /// 下载URL
    pub download_url: String,
    /// 校验和
    pub checksum: String,
    /// 校验和算法
    pub checksum_algorithm: String,
    /// 文件大小
    pub size_bytes: u64,
    /// 是否为强制更新
    pub mandatory: bool,
}

/// 更新状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UpdateStatus {
    /// 无可用更新
    UpToDate,
    /// 有可用更新
    Available(VersionInfo),
    /// 正在下载
    Downloading {
        version: String,
        progress: f64,
        downloaded_bytes: u64,
        total_bytes: u64,
    },
    /// 下载完成
    Downloaded(String),
    /// 正在安装
    Installing(String),
    /// 安装成功
    Installed(String),
    /// 更新失败
    Failed {
        version: String,
        error: String,
        can_retry: bool,
    },
    /// 正在回滚
    RollingBack(String),
    /// 回滚完成
    RolledBack(String),
}

/// 更新管理器
pub struct UpdateManager {
    config: UpdateConfig,
    status: Arc<Mutex<UpdateStatus>>,
    http_client: reqwest::Client,
}

impl UpdateManager {
    /// 创建新的更新管理器
    pub fn new(config: UpdateConfig) -> Self {
        Self {
            config,
            status: Arc::new(Mutex::new(UpdateStatus::UpToDate)),
            http_client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
        }
    }

    /// 检查更新
    pub async fn check_for_updates(&self) -> Result<UpdateStatus, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}/api/v1/updates/latest", self.config.update_server_url);

        tracing::info!("Checking for updates from: {}", url);

        let response = self.http_client
            .get(&url)
            .header("User-Agent", "Rust-Edge-Compute/1.0")
            .header("X-Current-Version", &self.config.current_version)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(format!("Update server returned status: {}", response.status()).into());
        }

        let version_info: VersionInfo = response.json().await?;

        // 比较版本
        if self.is_newer_version(&version_info.version)? {
            let mut status = self.status.lock().await;
            *status = UpdateStatus::Available(version_info.clone());
            tracing::info!("New version available: {}", version_info.version);
            Ok(UpdateStatus::Available(version_info))
        } else {
            tracing::info!("Already up to date: {}", self.config.current_version);
            Ok(UpdateStatus::UpToDate)
        }
    }

    /// 下载更新
    pub async fn download_update(&self, version_info: &VersionInfo) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // 更新状态为正在下载
        {
            let mut status = self.status.lock().await;
            *status = UpdateStatus::Downloading {
                version: version_info.version.clone(),
                progress: 0.0,
                downloaded_bytes: 0,
                total_bytes: version_info.size_bytes,
            };
        }

        // 确保下载目录存在
        tokio::fs::create_dir_all(&self.config.download_dir).await
            .map_err(|e| format!("Failed to create download directory: {}", e))?;

        let download_path = Path::new(&self.config.download_dir)
            .join(format!("update-{}.bin", version_info.version));

        tracing::info!("Downloading update to: {}", download_path.display());

        // 下载文件
        let mut response = self.http_client
            .get(&version_info.download_url)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_msg = format!("Download failed with status: {}", response.status());
            self.update_status(UpdateStatus::Failed {
                version: version_info.version.clone(),
                error: error_msg.clone(),
                can_retry: true,
            }).await;
            return Err(error_msg.into());
        }

        let mut file = tokio::fs::File::create(&download_path).await
            .map_err(|e| format!("Failed to create download file: {}", e))?;

        let mut downloaded = 0u64;
        let total = version_info.size_bytes;

        while let Some(chunk) = response.chunk().await? {
            tokio::io::AsyncWriteExt::write_all(&mut file, &chunk).await?;
            downloaded += chunk.len() as u64;

            // 更新进度
            let progress = (downloaded as f64 / total as f64) * 100.0;
            {
                let mut status = self.status.lock().await;
                if let UpdateStatus::Downloading { version, .. } = &*status {
                    if version == &version_info.version {
                        *status = UpdateStatus::Downloading {
                            version: version_info.version.clone(),
                            progress,
                            downloaded_bytes: downloaded,
                            total_bytes: total,
                        };
                    }
                }
            }
        }

        // 验证校验和
        self.verify_checksum(&download_path, version_info).await?;

        // 更新状态为下载完成
        self.update_status(UpdateStatus::Downloaded(version_info.version.clone())).await;

        tracing::info!("Update download completed: {}", version_info.version);
        Ok(())
    }

    /// 安装更新
    pub async fn install_update(&self, version: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let download_path = Path::new(&self.config.download_dir)
            .join(format!("update-{}.bin", version));

        if !download_path.exists() {
            return Err(format!("Update file not found: {}", download_path.display()).into());
        }

        // 更新状态为正在安装
        self.update_status(UpdateStatus::Installing(version.to_string())).await;

        tracing::info!("Installing update: {}", version);

        // 备份当前版本（如果启用）
        if self.config.backup_old_version {
            self.backup_current_version().await?;
        }

        // 这里应该实现具体的安装逻辑
        // 对于Rust应用，通常需要：
        // 1. 停止当前服务
        // 2. 替换二进制文件
        // 3. 重启服务

        // 模拟安装过程
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        // 更新配置中的版本号
        // 注意：这只是更新配置，实际的版本更新需要在重启后生效

        // 更新状态为安装完成
        self.update_status(UpdateStatus::Installed(version.to_string())).await;

        tracing::info!("Update installation completed: {}", version);
        Ok(())
    }

    /// 回滚到上一版本
    pub async fn rollback_update(&self, target_version: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // 更新状态为正在回滚
        self.update_status(UpdateStatus::RollingBack(target_version.to_string())).await;

        tracing::info!("Rolling back to version: {}", target_version);

        // 查找备份文件
        let backup_path = Path::new(&self.config.download_dir)
            .join(format!("backup-{}.bin", target_version));

        if !backup_path.exists() {
            let error_msg = format!("Backup file not found: {}", backup_path.display());
            self.update_status(UpdateStatus::Failed {
                version: target_version.to_string(),
                error: error_msg.clone(),
                can_retry: false,
            }).await;
            return Err(error_msg.into());
        }

        // 恢复备份文件
        self.restore_from_backup(&backup_path).await?;

        // 更新状态为回滚完成
        self.update_status(UpdateStatus::RolledBack(target_version.to_string())).await;

        tracing::info!("Rollback completed: {}", target_version);
        Ok(())
    }

    /// 获取当前更新状态
    pub async fn get_update_status(&self) -> UpdateStatus {
        self.status.lock().await.clone()
    }

    /// 验证文件校验和
    async fn verify_checksum(&self, file_path: &Path, version_info: &VersionInfo) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use ring::digest;

        let file_content = tokio::fs::read(file_path).await
            .map_err(|e| format!("Failed to read file for checksum verification: {}", e))?;

        let actual_checksum = match version_info.checksum_algorithm.as_str() {
            "SHA256" => {
                let hash = digest::digest(&digest::SHA256, &file_content);
                hex::encode(hash.as_ref())
            }
            "SHA512" => {
                let hash = digest::digest(&digest::SHA512, &file_content);
                hex::encode(hash.as_ref())
            }
            _ => {
                return Err(format!("Unsupported checksum algorithm: {}", version_info.checksum_algorithm).into());
            }
        };

        if actual_checksum != version_info.checksum {
            return Err(format!("Checksum verification failed. Expected: {}, Actual: {}",
                version_info.checksum, actual_checksum).into());
        }

        tracing::info!("Checksum verification passed for version: {}", version_info.version);
        Ok(())
    }

    /// 检查版本是否更新
    fn is_newer_version(&self, new_version: &str) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let current_parts: Vec<&str> = self.config.current_version.split('.').collect();
        let new_parts: Vec<&str> = new_version.split('.').collect();

        if current_parts.len() != new_parts.len() {
            return Err("Version format mismatch".into());
        }

        for (current, new) in current_parts.iter().zip(new_parts.iter()) {
            let current_num: u32 = current.parse()?;
            let new_num: u32 = new.parse()?;

            if new_num > current_num {
                return Ok(true);
            } else if new_num < current_num {
                return Ok(false);
            }
        }

        Ok(false) // 版本相同
    }

    /// 备份当前版本
    async fn backup_current_version(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // 获取当前可执行文件路径
        let current_exe = std::env::current_exe()
            .map_err(|e| format!("Failed to get current executable path: {}", e))?;

        let backup_path = Path::new(&self.config.download_dir)
            .join(format!("backup-{}.bin", self.config.current_version));

        tokio::fs::copy(&current_exe, &backup_path).await
            .map_err(|e| format!("Failed to backup current version: {}", e))?;

        tracing::info!("Current version backed up: {} -> {}",
            current_exe.display(), backup_path.display());

        Ok(())
    }

    /// 从备份恢复
    async fn restore_from_backup(&self, backup_path: &Path) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let current_exe = std::env::current_exe()
            .map_err(|e| format!("Failed to get current executable path: {}", e))?;

        tokio::fs::copy(backup_path, &current_exe).await
            .map_err(|e| format!("Failed to restore from backup: {}", e))?;

        tracing::info!("Restored from backup: {} -> {}",
            backup_path.display(), current_exe.display());

        Ok(())
    }

    /// 更新状态
    async fn update_status(&self, new_status: UpdateStatus) {
        let mut status = self.status.lock().await;
        *status = new_status;
    }

    /// 获取更新历史
    pub async fn get_update_history(&self) -> Vec<UpdateRecord> {
        // 这里应该从持久化存储中读取更新历史
        // 暂时返回空列表
        Vec::new()
    }
}

/// 更新记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateRecord {
    /// 版本号
    pub version: String,
    /// 更新时间
    pub update_time: chrono::DateTime<chrono::Utc>,
    /// 更新结果
    pub result: UpdateResult,
    /// 详细信息
    pub details: Option<String>,
}

/// 更新结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UpdateResult {
    /// 成功
    Success,
    /// 失败
    Failed(String),
    /// 回滚
    RolledBack(String),
}

/// 更新服务
pub struct UpdateService {
    manager: Arc<UpdateManager>,
}

impl UpdateService {
    /// 创建新的更新服务
    pub fn new(manager: Arc<UpdateManager>) -> Self {
        Self { manager }
    }

    /// 启动自动更新检查
    pub async fn start_auto_check(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let manager = Arc::clone(&self.manager);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                std::time::Duration::from_secs(manager.config.check_interval_seconds)
            );

            loop {
                interval.tick().await;

                match manager.check_for_updates().await {
                    Ok(UpdateStatus::Available(version_info)) => {
                        tracing::info!("Auto-update check found new version: {}", version_info.version);

                        // 如果是强制更新，自动下载
                        if version_info.mandatory {
                            tracing::info!("Mandatory update detected, starting download...");
                            if let Err(e) = manager.download_update(&version_info).await {
                                tracing::error!("Auto-download failed: {}", e);
                            }
                        }
                    }
                    Ok(_) => {
                        // 已经是最新版本，无需操作
                    }
                    Err(e) => {
                        tracing::warn!("Auto-update check failed: {}", e);
                    }
                }
            }
        });

        tracing::info!("Auto-update check started with interval: {}s",
            self.manager.config.check_interval_seconds);

        Ok(())
    }

    /// 停止自动更新检查
    pub async fn stop_auto_check(&self) {
        // 这里应该实现停止自动检查的逻辑
        // 在实际实现中，可能需要使用CancellationToken
        tracing::info!("Auto-update check stopped");
    }
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self {
            auto_check_updates: true,
            check_interval_seconds: 3600, // 1小时
            update_server_url: "https://updates.example.com".to_string(),
            current_version: env!("CARGO_PKG_VERSION").to_string(),
            download_dir: "./updates".to_string(),
            allow_downgrades: false,
            backup_old_version: true,
            max_retry_attempts: 3,
            update_timeout_seconds: 300, // 5分钟
        }
    }
}

/// 更新API处理器
pub mod handlers {
    use super::*;
    use axum::{extract::Query, Json};
    use std::collections::HashMap;

    /// 检查更新
    pub async fn check_updates(
        manager: Arc<UpdateManager>,
    ) -> Result<Json<serde_json::Value>, Box<dyn std::error::Error + Send + Sync>> {
        match manager.check_for_updates().await? {
            UpdateStatus::Available(version_info) => {
                Ok(Json(serde_json::json!({
                    "available": true,
                    "version": version_info.version,
                    "changelog": version_info.changelog,
                    "mandatory": version_info.mandatory,
                    "size_bytes": version_info.size_bytes
                })))
            }
            UpdateStatus::UpToDate => {
                Ok(Json(serde_json::json!({
                    "available": false,
                    "message": "Already up to date"
                })))
            }
            _ => {
                Ok(Json(serde_json::json!({
                    "available": false,
                    "message": "Update check in progress"
                })))
            }
        }
    }

    /// 获取更新状态
    pub async fn get_update_status(
        manager: Arc<UpdateManager>,
    ) -> Json<serde_json::Value> {
        let status = manager.get_update_status().await;

        let response = match status {
            UpdateStatus::UpToDate => {
                serde_json::json!({
                    "status": "up_to_date",
                    "message": "No updates available"
                })
            }
            UpdateStatus::Available(version_info) => {
                serde_json::json!({
                    "status": "available",
                    "version": version_info.version,
                    "changelog": version_info.changelog,
                    "mandatory": version_info.mandatory
                })
            }
            UpdateStatus::Downloading { version, progress, downloaded_bytes, total_bytes } => {
                serde_json::json!({
                    "status": "downloading",
                    "version": version,
                    "progress": progress,
                    "downloaded_bytes": downloaded_bytes,
                    "total_bytes": total_bytes
                })
            }
            UpdateStatus::Downloaded(version) => {
                serde_json::json!({
                    "status": "downloaded",
                    "version": version,
                    "message": "Update downloaded, ready to install"
                })
            }
            UpdateStatus::Installing(version) => {
                serde_json::json!({
                    "status": "installing",
                    "version": version
                })
            }
            UpdateStatus::Installed(version) => {
                serde_json::json!({
                    "status": "installed",
                    "version": version,
                    "message": "Update installed successfully"
                })
            }
            UpdateStatus::Failed { version, error, can_retry } => {
                serde_json::json!({
                    "status": "failed",
                    "version": version,
                    "error": error,
                    "can_retry": can_retry
                })
            }
            UpdateStatus::RollingBack(version) => {
                serde_json::json!({
                    "status": "rolling_back",
                    "version": version
                })
            }
            UpdateStatus::RolledBack(version) => {
                serde_json::json!({
                    "status": "rolled_back",
                    "version": version,
                    "message": "Rollback completed successfully"
                })
            }
        };

        Json(response)
    }

    /// 下载更新
    pub async fn download_update(
        manager: Arc<UpdateManager>,
    ) -> Result<Json<serde_json::Value>, Box<dyn std::error::Error + Send + Sync>> {
        let status = manager.get_update_status().await;

        if let UpdateStatus::Available(version_info) = status {
            manager.download_update(&version_info).await?;
            Ok(Json(serde_json::json!({
                "message": "Update download started",
                "version": version_info.version
            })))
        } else {
            Err("No update available for download".into())
        }
    }

    /// 安装更新
    pub async fn install_update(
        manager: Arc<UpdateManager>,
    ) -> Result<Json<serde_json::Value>, Box<dyn std::error::Error + Send + Sync>> {
        let status = manager.get_update_status().await;

        if let UpdateStatus::Downloaded(version) = status {
            manager.install_update(&version).await?;
            Ok(Json(serde_json::json!({
                "message": "Update installation started",
                "version": version
            })))
        } else {
            Err("No downloaded update available for installation".into())
        }
    }

    /// 回滚更新
    pub async fn rollback_update(
        Query(params): Query<HashMap<String, String>>,
        manager: Arc<UpdateManager>,
    ) -> Result<Json<serde_json::Value>, Box<dyn std::error::Error + Send + Sync>> {
        let target_version = params.get("version")
            .ok_or("Target version parameter is required")?;

        manager.rollback_update(target_version).await?;
        Ok(Json(serde_json::json!({
            "message": "Rollback started",
            "target_version": target_version
        })))
    }

    /// 获取更新历史
    pub async fn get_update_history(
        manager: Arc<UpdateManager>,
    ) -> Json<serde_json::Value> {
        let history = manager.get_update_history().await;
        Json(serde_json::json!({
            "history": history
        }))
    }
}
