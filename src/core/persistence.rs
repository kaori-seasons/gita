//! 数据持久化模块
//!
//! 提供任务状态、错误统计和配置数据的持久化存储

use sled::{Db, IVec};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

use super::{EdgeComputeError, ErrorRecord, ErrorStats, ScheduledTask, TaskPriority};
use crate::Result;

/// 持久化存储
pub struct PersistenceStore {
    /// Sled数据库实例
    db: Arc<Db>,
    /// 任务存储前缀
    task_prefix: String,
    /// 错误统计存储前缀
    error_prefix: String,
    /// 配置存储前缀
    config_prefix: String,
}

/// 存储键常量
const TASK_PREFIX: &str = "task:";
const ERROR_PREFIX: &str = "error:";
const CONFIG_PREFIX: &str = "config:";
const TASK_QUEUE_KEY: &str = "task_queue";
const ERROR_STATS_KEY: &str = "error_stats";

impl PersistenceStore {
    /// 创建新的持久化存储
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let db = sled::open(path)
            .map_err(|e| format!("Failed to open database: {}", e))?;

        Ok(Self {
            db: Arc::new(db),
            task_prefix: TASK_PREFIX.to_string(),
            error_prefix: ERROR_PREFIX.to_string(),
            config_prefix: CONFIG_PREFIX.to_string(),
        })
    }

    /// 存储任务
    pub async fn store_task(&self, task: &ScheduledTask) -> Result<()> {
        let key = format!("{}{}", self.task_prefix, task.id);
        let value = bincode::serialize(task)
            .map_err(|e| format!("Failed to serialize task: {}", e))?;

        self.db.insert(key.as_bytes(), value)
            .map_err(|e| format!("Failed to store task: {}", e))?;

        // 确保数据写入磁盘
        self.db.flush_async().await
            .map_err(|e| format!("Failed to flush database: {}", e))?;

        Ok(())
    }

    /// 加载任务
    pub async fn load_task(&self, task_id: &str) -> Result<Option<ScheduledTask>> {
        let key = format!("{}{}", self.task_prefix, task_id);

        match self.db.get(key.as_bytes()) {
            Ok(Some(value)) => {
                let task: ScheduledTask = bincode::deserialize(&value)
                    .map_err(|e| format!("Failed to deserialize task: {}", e))?;
                Ok(Some(task))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(format!("Failed to load task: {}", e).into()),
        }
    }

    /// 删除任务
    pub async fn delete_task(&self, task_id: &str) -> Result<()> {
        let key = format!("{}{}", self.task_prefix, task_id);

        self.db.remove(key.as_bytes())
            .map_err(|e| format!("Failed to delete task: {}", e))?;

        // 确保数据写入磁盘
        self.db.flush_async().await
            .map_err(|e| format!("Failed to flush database: {}", e))?;

        Ok(())
    }

    /// 存储任务队列状态
    pub async fn store_task_queue(&self, tasks: &[ScheduledTask]) -> Result<()> {
        let key = format!("{}{}", self.task_prefix, TASK_QUEUE_KEY);
        let value = bincode::serialize(tasks)
            .map_err(|e| format!("Failed to serialize task queue: {}", e))?;

        self.db.insert(key.as_bytes(), value)
            .map_err(|e| format!("Failed to store task queue: {}", e))?;

        self.db.flush_async().await
            .map_err(|e| format!("Failed to flush database: {}", e))?;

        Ok(())
    }

    /// 加载任务队列状态
    pub async fn load_task_queue(&self) -> Result<Vec<ScheduledTask>> {
        let key = format!("{}{}", self.task_prefix, TASK_QUEUE_KEY);

        match self.db.get(key.as_bytes()) {
            Ok(Some(value)) => {
                let tasks: Vec<ScheduledTask> = bincode::deserialize(&value)
                    .map_err(|e| format!("Failed to deserialize task queue: {}", e))?;
                Ok(tasks)
            }
            Ok(None) => Ok(Vec::new()),
            Err(e) => Err(format!("Failed to load task queue: {}", e).into()),
        }
    }

    /// 存储错误统计
    pub async fn store_error_stats(&self, stats: &ErrorStats) -> Result<()> {
        let key = format!("{}{}", self.error_prefix, ERROR_STATS_KEY);
        let value = bincode::serialize(stats)
            .map_err(|e| format!("Failed to serialize error stats: {}", e))?;

        self.db.insert(key.as_bytes(), value)
            .map_err(|e| format!("Failed to store error stats: {}", e))?;

        self.db.flush_async().await
            .map_err(|e| format!("Failed to flush database: {}", e))?;

        Ok(())
    }

    /// 加载错误统计
    pub async fn load_error_stats(&self) -> Result<Option<ErrorStats>> {
        let key = format!("{}{}", self.error_prefix, ERROR_STATS_KEY);

        match self.db.get(key.as_bytes()) {
            Ok(Some(value)) => {
                let stats: ErrorStats = bincode::deserialize(&value)
                    .map_err(|e| format!("Failed to deserialize error stats: {}", e))?;
                Ok(Some(stats))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(format!("Failed to load error stats: {}", e).into()),
        }
    }

    /// 存储错误记录
    pub async fn store_error_record(&self, record: &ErrorRecord) -> Result<()> {
        let key = format!("{}record:{}", self.error_prefix, record.id);
        let value = bincode::serialize(record)
            .map_err(|e| format!("Failed to serialize error record: {}", e))?;

        self.db.insert(key.as_bytes(), value)
            .map_err(|e| format!("Failed to store error record: {}", e))?;

        Ok(())
    }

    /// 加载所有错误记录
    pub async fn load_error_records(&self, limit: usize) -> Result<Vec<ErrorRecord>> {
        let mut records = Vec::new();
        let prefix = format!("{}record:", self.error_prefix);

        for item in self.db.scan_prefix(prefix.as_bytes()) {
            match item {
                Ok((_, value)) => {
                    let record: ErrorRecord = bincode::deserialize(&value)
                        .map_err(|e| format!("Failed to deserialize error record: {}", e))?;
                    records.push(record);

                    if records.len() >= limit {
                        break;
                    }
                }
                Err(e) => return Err(format!("Failed to scan error records: {}", e).into()),
            }
        }

        // 按时间戳倒序排列
        records.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        Ok(records)
    }

    /// 存储配置
    pub async fn store_config(&self, key: &str, value: &str) -> Result<()> {
        let full_key = format!("{}{}", self.config_prefix, key);

        self.db.insert(full_key.as_bytes(), value.as_bytes())
            .map_err(|e| format!("Failed to store config: {}", e))?;

        self.db.flush_async().await
            .map_err(|e| format!("Failed to flush database: {}", e))?;

        Ok(())
    }

    /// 加载配置
    pub async fn load_config(&self, key: &str) -> Result<Option<String>> {
        let full_key = format!("{}{}", self.config_prefix, key);

        match self.db.get(full_key.as_bytes()) {
            Ok(Some(value)) => {
                let config_value = String::from_utf8(value.to_vec())
                    .map_err(|e| format!("Failed to decode config value: {}", e))?;
                Ok(Some(config_value))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(format!("Failed to load config: {}", e).into()),
        }
    }

    /// 获取数据库统计信息
    pub async fn get_stats(&self) -> Result<DatabaseStats> {
        let size = self.db.size_on_disk()
            .map_err(|e| format!("Failed to get database size: {}", e))?;

        // 计算任务数量
        let task_count = self.db.scan_prefix(self.task_prefix.as_bytes()).count();

        // 计算错误记录数量
        let error_count = self.db.scan_prefix(self.error_prefix.as_bytes()).count();

        Ok(DatabaseStats {
            total_size_bytes: size,
            task_count,
            error_count,
            config_count: self.db.scan_prefix(self.config_prefix.as_bytes()).count(),
        })
    }

    /// 清理过期数据
    pub async fn cleanup_expired_data(&self, max_age_days: u64) -> Result<CleanupStats> {
        let cutoff_time = std::time::SystemTime::now()
            - std::time::Duration::from_secs(max_age_days * 24 * 60 * 60);

        let mut removed_tasks = 0;
        let mut removed_errors = 0;

        // 清理过期的任务（简化实现，实际应该基于任务状态和时间）
        // 这里只是示例，实际实现需要更复杂的逻辑

        // 清理过期的错误记录
        let error_prefix = format!("{}record:", self.error_prefix);
        let mut keys_to_remove = Vec::new();

        for item in self.db.scan_prefix(error_prefix.as_bytes()) {
            match item {
                Ok((key, value)) => {
                    if let Ok(record) = bincode::deserialize::<ErrorRecord>(&value) {
                        if record.timestamp < cutoff_time {
                            keys_to_remove.push(key);
                            removed_errors += 1;
                        }
                    }
                }
                Err(_) => continue,
            }
        }

        // 批量删除
        for key in keys_to_remove {
            let _ = self.db.remove(key);
        }

        // 刷新数据库
        self.db.flush_async().await
            .map_err(|e| format!("Failed to flush after cleanup: {}", e))?;

        Ok(CleanupStats {
            removed_tasks,
            removed_errors,
            freed_space_bytes: 0, // 简化实现
        })
    }

    /// 关闭数据库
    pub async fn close(self) -> Result<()> {
        self.db.flush_async().await
            .map_err(|e| format!("Failed to flush before close: {}", e))?;

        // 注意：sled的drop会自动处理关闭
        Ok(())
    }
}

/// 数据库统计信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DatabaseStats {
    /// 数据库总大小（字节）
    pub total_size_bytes: u64,
    /// 任务数量
    pub task_count: usize,
    /// 错误记录数量
    pub error_count: usize,
    /// 配置项数量
    pub config_count: usize,
}

/// 清理统计信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CleanupStats {
    /// 移除的任务数量
    pub removed_tasks: usize,
    /// 移除的错误记录数量
    pub removed_errors: usize,
    /// 释放的空间（字节）
    pub freed_space_bytes: u64,
}

/// 持久化管理器
pub struct PersistenceManager {
    store: Arc<PersistenceStore>,
}

impl PersistenceManager {
    /// 创建新的持久化管理器
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let store = PersistenceStore::new(path)?;
        Ok(Self {
            store: Arc::new(store),
        })
    }

    /// 获取存储实例
    pub fn store(&self) -> Arc<PersistenceStore> {
        Arc::clone(&self.store)
    }

    /// 备份数据库
    pub async fn backup<P: AsRef<Path>>(&self, backup_path: P) -> Result<()> {
        // 简化实现：复制数据库文件
        // 实际应该使用sled的导出功能
        tokio::fs::copy("./data/db", backup_path)
            .await
            .map_err(|e| format!("Failed to backup database: {}", e))?;

        Ok(())
    }

    /// 从备份恢复
    pub async fn restore<P: AsRef<Path>>(&self, backup_path: P) -> Result<()> {
        // 简化实现：复制备份文件
        tokio::fs::copy(backup_path, "./data/db")
            .await
            .map_err(|e| format!("Failed to restore database: {}", e))?;

        Ok(())
    }
}

impl Default for PersistenceManager {
    fn default() -> Self {
        Self::new("./data/db").expect("Failed to create default persistence manager")
    }
}
