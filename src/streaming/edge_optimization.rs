//! 边缘端资源优化
//!
//! 针对4核8G内存、HDD硬盘的工控机环境进行专项优化
//! 包括内存管理、磁盘I/O优化、CPU调度优化等

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use crate::streaming::garbage_collector::{GarbageCollector, GCConfig, ObjectId};

/// 边缘优化配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeOptimizationConfig {
    /// 内存优化配置
    pub memory_optimization: MemoryOptimizationConfig,
    /// 磁盘优化配置
    pub disk_optimization: DiskOptimizationConfig,
    /// CPU优化配置
    pub cpu_optimization: CpuOptimizationConfig,
    /// 缓存优化配置
    pub cache_optimization: CacheOptimizationConfig,
}

/// 内存优化配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryOptimizationConfig {
    /// 启用内存池
    pub enable_memory_pool: bool,
    /// 内存池大小(MB)
    pub memory_pool_size_mb: usize,
    /// 对象重用池大小
    pub object_pool_size: usize,
    /// 启用内存映射文件
    pub enable_memory_mapped_files: bool,
    /// 内存映射文件大小(MB)
    pub memory_mapped_file_size_mb: usize,
    /// 垃圾回收阈值
    pub gc_threshold_mb: usize,
}

/// 磁盘优化配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskOptimizationConfig {
    /// 启用磁盘缓存
    pub enable_disk_cache: bool,
    /// 缓存目录
    pub cache_directory: String,
    /// 缓存大小(MB)
    pub cache_size_mb: usize,
    /// 启用异步I/O
    pub enable_async_io: bool,
    /// I/O线程池大小
    pub io_thread_pool_size: usize,
    /// 启用预读
    pub enable_read_ahead: bool,
    /// 预读大小(KB)
    pub read_ahead_size_kb: usize,
}

/// CPU优化配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuOptimizationConfig {
    /// 启用CPU亲和性
    pub enable_cpu_affinity: bool,
    /// CPU核心绑定列表
    pub cpu_cores: Vec<usize>,
    /// 线程池大小
    pub thread_pool_size: usize,
    /// 启用工作窃取
    pub enable_work_stealing: bool,
    /// 任务优先级队列
    pub enable_priority_queue: bool,
}

/// 缓存优化配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheOptimizationConfig {
    /// 启用多级缓存
    pub enable_multi_level_cache: bool,
    /// L1缓存大小
    pub l1_cache_size: usize,
    /// L2缓存大小
    pub l2_cache_size: usize,
    /// 缓存过期时间(秒)
    pub cache_ttl_seconds: u64,
    /// 缓存压缩启用
    pub enable_cache_compression: bool,
}

/// 边缘优化管理器
pub struct EdgeOptimizationManager {
    config: EdgeOptimizationConfig,
    memory_manager: Arc<RwLock<MemoryManager>>,
    disk_manager: Arc<RwLock<DiskManager>>,
    cache_manager: Arc<RwLock<CacheManager>>,
    gc_manager: Arc<RwLock<GarbageCollector>>,
    metrics: Arc<RwLock<OptimizationMetrics>>,
}

/// 内存管理器
pub struct MemoryManager {
    pool: Vec<Vec<u8>>,
    allocated: usize,
    pool_size: usize,
}

/// 磁盘管理器
pub struct DiskManager {
    cache: HashMap<String, CachedData>,
    cache_size: usize,
    current_size: usize,
}

/// 缓存管理器
pub struct CacheManager {
    l1_cache: HashMap<String, CacheEntry>,
    l2_cache: HashMap<String, CacheEntry>,
    l1_size: usize,
    l2_size: usize,
}

/// 缓存条目
#[derive(Debug, Clone)]
pub struct CacheEntry {
    data: Vec<u8>,
    timestamp: std::time::Instant,
    ttl: u64,
    access_count: u64,
    last_access: std::time::Instant,
}

/// 缓存数据
#[derive(Debug, Clone)]
pub struct CachedData {
    data: Vec<u8>,
    path: PathBuf,
    size: usize,
    last_access: std::time::Instant,
}

/// 优化指标
#[derive(Debug, Clone, Default)]
pub struct OptimizationMetrics {
    pub memory_usage_mb: f64,
    pub cache_hit_rate: f64,
    pub disk_io_operations: u64,
    pub cpu_utilization: f64,
    pub average_latency_ms: f64,
}

impl EdgeOptimizationManager {
    /// 创建边缘优化管理器
    pub async fn new(config: EdgeOptimizationConfig) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let memory_manager = Arc::new(RwLock::new(MemoryManager::new(
            config.memory_optimization.memory_pool_size_mb * 1024 * 1024
        )));

        let disk_manager = Arc::new(RwLock::new(DiskManager::new(
            config.disk_optimization.cache_size_mb * 1024 * 1024
        )));

        let cache_manager = Arc::new(RwLock::new(CacheManager::new(
            config.cache_optimization.l1_cache_size,
            config.cache_optimization.l2_cache_size,
        )));

        // 创建GC配置
        let gc_config = GCConfig {
            enabled: true,
            strategy: crate::streaming::garbage_collector::GCStrategy::Generational,
            heap_size_mb: config.memory_optimization.memory_pool_size_mb,
            young_generation_ratio: 0.3,
            old_generation_ratio: 0.6,
            perm_generation_ratio: 0.1,
            gc_threshold_percent: 75,
            max_pause_time_ms: 200,
            parallel_gc_threads: 2, // 4核CPU，保留2核用于业务
            enable_incremental_gc: true,
            enable_reference_counting: true,
            enable_compacting_gc: true,
            log_level: crate::streaming::garbage_collector::GCLogLevel::Basic,
        };

        let gc_manager = Arc::new(RwLock::new(GarbageCollector::new(gc_config)?));

        Ok(Self {
            config,
            memory_manager,
            disk_manager,
            cache_manager,
            gc_manager,
            metrics: Arc::new(RwLock::new(OptimizationMetrics::default())),
        })
    }

    /// 优化内存分配
    pub async fn optimize_memory_allocation(&self, size: usize) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        let mut memory_manager = self.memory_manager.write().await;

        // 尝试从内存池分配
        if self.config.memory_optimization.enable_memory_pool {
            if let Some(buffer) = memory_manager.allocate_from_pool(size) {
                return Ok(buffer);
            }
        }

        // 检查是否需要触发GC
        let should_gc = {
            let gc_manager = self.gc_manager.read().await;
            let metrics = gc_manager.get_metrics().await;
            let heap_used_ratio = metrics.heap_used_mb / metrics.heap_total_mb;
            heap_used_ratio > (self.config.memory_optimization.gc_threshold_mb as f64 / 100.0)
        };

        if should_gc {
            let gc_manager = self.gc_manager.write().await;
            gc_manager.trigger_gc(crate::streaming::garbage_collector::GCTrigger::YoungGC).await?;
        }

        // 从系统分配
        let buffer = vec![0u8; size];
        memory_manager.allocated += size;

        // 注册到GC系统中
        if self.config.memory_optimization.enable_memory_pool {
            // 这里可以注册对象到GC系统，但为了简化，我们暂时不做
        }

        Ok(buffer)
    }

    /// 优化磁盘I/O
    pub async fn optimize_disk_io(&self, operation: DiskOperation) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut disk_manager = self.disk_manager.write().await;

        match operation {
            DiskOperation::Read { key, path } => {
                // 检查缓存
                if let Some(data) = disk_manager.get_cached(&key) {
                    tracing::debug!("Cache hit for key: {}", key);
                    return Ok(());
                }

                // 异步读取
                if self.config.disk_optimization.enable_async_io {
                    self.async_read_file(path, key).await?;
                } else {
                    self.sync_read_file(path, key).await?;
                }
            }
            DiskOperation::Write { key, data, path } => {
                // 写入缓存
                disk_manager.put_cached(key.clone(), data, path.clone());

                // 异步写入
                if self.config.disk_optimization.enable_async_io {
                    self.async_write_file(path, data).await?;
                } else {
                    tokio::fs::write(&path, data).await?;
                }
            }
        }

        Ok(())
    }

    /// 优化缓存访问
    pub async fn optimize_cache_access(&self, key: &str) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error + Send + Sync>> {
        let mut cache_manager = self.cache_manager.write().await;

        // 首先检查L1缓存
        if let Some(entry) = cache_manager.l1_cache.get_mut(key) {
            if !entry.is_expired() {
                entry.access_count += 1;
                entry.last_access = std::time::Instant::now();
                return Ok(Some(entry.data.clone()));
            } else {
                cache_manager.l1_cache.remove(key);
            }
        }

        // 检查L2缓存
        if let Some(entry) = cache_manager.l2_cache.get_mut(key) {
            if !entry.is_expired() {
                entry.access_count += 1;
                entry.last_access = std::time::Instant::now();

                // 提升到L1缓存
                cache_manager.l1_cache.insert(key.to_string(), entry.clone());
                return Ok(Some(entry.data.clone()));
            } else {
                cache_manager.l2_cache.remove(key);
            }
        }

        Ok(None)
    }

    /// 存储到缓存
    pub async fn store_in_cache(&self, key: String, data: Vec<u8>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut cache_manager = self.cache_manager.write().await;

        let entry = CacheEntry {
            data: data.clone(),
            timestamp: std::time::Instant::now(),
            ttl: self.config.cache_optimization.cache_ttl_seconds,
            access_count: 0,
            last_access: std::time::Instant::now(),
        };

        // 压缩数据（如果启用）
        let final_data = if self.config.cache_optimization.enable_cache_compression {
            self.compress_data(data)?
        } else {
            data
        };

        let entry = CacheEntry {
            data: final_data,
            ..entry
        };

        // 存储到L1缓存
        if cache_manager.l1_cache.len() < cache_manager.l1_size {
            cache_manager.l1_cache.insert(key, entry);
        } else {
            // L1缓存满了，存储到L2缓存
            if cache_manager.l2_cache.len() < cache_manager.l2_size {
                cache_manager.l2_cache.insert(key, entry);
            } else {
                // L2缓存也满了，使用LRU策略
                cache_manager.evict_lru_l2();
                cache_manager.l2_cache.insert(key, entry);
            }
        }

        Ok(())
    }

    /// 获取优化指标
    pub async fn get_optimization_metrics(&self) -> OptimizationMetrics {
        let mut metrics = self.metrics.write().await;

        // 更新内存使用情况
        let memory_manager = self.memory_manager.read().await;
        metrics.memory_usage_mb = memory_manager.allocated as f64 / (1024.0 * 1024.0);

        // 更新GC指标
        let gc_manager = self.gc_manager.read().await;
        let gc_metrics = gc_manager.get_metrics().await;
        metrics.gc_efficiency = gc_metrics.gc_efficiency;

        // 更新缓存命中率
        let cache_manager = self.cache_manager.read().await;
        let total_requests = cache_manager.get_total_requests();
        let cache_hits = cache_manager.get_cache_hits();
        metrics.cache_hit_rate = if total_requests > 0 {
            cache_hits as f64 / total_requests as f64
        } else {
            0.0
        };

        metrics.clone()
    }

    /// 获取GC指标
    pub async fn get_gc_metrics(&self) -> crate::streaming::garbage_collector::GCMetrics {
        let gc_manager = self.gc_manager.read().await;
        gc_manager.get_metrics().await
    }

    /// 手动触发GC
    pub async fn trigger_gc(&self, trigger: crate::streaming::garbage_collector::GCTrigger) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let gc_manager = self.gc_manager.write().await;
        gc_manager.trigger_gc(trigger).await
    }

    /// 分配GC管理的对象
    pub async fn allocate_gc_object(&self, class_name: String, size: usize, references: Vec<ObjectId>) -> Result<ObjectId, Box<dyn std::error::Error + Send + Sync>> {
        let gc_manager = self.gc_manager.write().await;
        gc_manager.allocate(class_name, size, references).await
    }

    /// 释放GC对象引用
    pub async fn release_gc_object(&self, object_id: ObjectId) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let gc_manager = self.gc_manager.write().await;
        gc_manager.release_reference(object_id).await
    }

    /// 添加GC根对象
    pub async fn add_gc_root(&self, object_id: ObjectId) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let gc_manager = self.gc_manager.write().await;
        gc_manager.add_root(object_id).await
    }

    /// 移除GC根对象
    pub async fn remove_gc_root(&self, object_id: ObjectId) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let gc_manager = self.gc_manager.write().await;
        gc_manager.remove_root(object_id).await
    }

    /// 异步读取文件
    async fn async_read_file(&self, path: PathBuf, key: String) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let data = tokio::fs::read(&path).await?;
        let mut disk_manager = self.disk_manager.write().await;
        disk_manager.put_cached(key, data, path);
        Ok(())
    }

    /// 同步读取文件
    async fn sync_read_file(&self, path: PathBuf, key: String) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let data = std::fs::read(&path)?;
        let mut disk_manager = self.disk_manager.write().await;
        disk_manager.put_cached(key, data, path);
        Ok(())
    }

    /// 异步写入文件
    async fn async_write_file(&self, path: PathBuf, data: Vec<u8>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        tokio::fs::write(&path, data).await?;
        Ok(())
    }

    /// 压缩数据
    fn compress_data(&self, data: Vec<u8>) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        // 简化的压缩实现
        // 实际应该使用专业的压缩算法
        Ok(data)
    }
}

impl MemoryManager {
    /// 创建内存管理器
    fn new(pool_size: usize) -> Self {
        Self {
            pool: Vec::new(),
            allocated: 0,
            pool_size,
        }
    }

    /// 从池中分配内存
    fn allocate_from_pool(&mut self, size: usize) -> Option<Vec<u8>> {
        // 查找合适大小的缓冲区
        for (i, buffer) in self.pool.iter().enumerate() {
            if buffer.len() >= size {
                let mut buffer = self.pool.remove(i);
                buffer.resize(size, 0);
                return Some(buffer);
            }
        }

        // 如果池中没有合适的缓冲区，创建新的
        if self.allocated + size <= self.pool_size {
            self.allocated += size;
            Some(vec![0u8; size])
        } else {
            None
        }
    }

    /// 垃圾回收（已废弃，使用生产级GC系统）
    #[deprecated(note = "使用生产级的垃圾回收系统代替")]
    fn gc(&mut self) {
        tracing::warn!("使用废弃的GC方法，建议使用生产级GC系统");
        // 保留原有逻辑以保证兼容性
        let keep_size = self.pool.len() / 2;
        self.pool.truncate(keep_size);
    }
}

impl DiskManager {
    /// 创建磁盘管理器
    fn new(cache_size: usize) -> Self {
        Self {
            cache: HashMap::new(),
            cache_size,
            current_size: 0,
        }
    }

    /// 获取缓存数据
    fn get_cached(&self, key: &str) -> Option<Vec<u8>> {
        self.cache.get(key).map(|cached| {
            // 更新最后访问时间
            let mut cached = cached.clone();
            cached.last_access = std::time::Instant::now();
            cached.data.clone()
        })
    }

    /// 存储缓存数据
    fn put_cached(&mut self, key: String, data: Vec<u8>, path: PathBuf) {
        let size = data.len();

        // 检查是否需要清理缓存
        if self.current_size + size > self.cache_size {
            self.evict_lru();
        }

        let cached_data = CachedData {
            data,
            path,
            size,
            last_access: std::time::Instant::now(),
        };

        self.cache.insert(key, cached_data);
        self.current_size += size;
    }

    /// 清理最少使用的缓存
    fn evict_lru(&mut self) {
        if let Some((key, cached)) = self.cache.iter()
            .min_by_key(|(_, cached)| cached.last_access) {
            let key = key.clone();
            let cached = self.cache.remove(&key).unwrap();
            self.current_size -= cached.size;
        }
    }
}

impl CacheManager {
    /// 创建缓存管理器
    fn new(l1_size: usize, l2_size: usize) -> Self {
        Self {
            l1_cache: HashMap::new(),
            l2_cache: HashMap::new(),
            l1_size,
            l2_size,
        }
    }

    /// 清理L2缓存中的LRU条目
    fn evict_lru_l2(&mut self) {
        if let Some((key, _)) = self.l2_cache.iter()
            .min_by_key(|(_, entry)| entry.last_access) {
            let key = key.clone();
            self.l2_cache.remove(&key);
        }
    }

    /// 获取总请求数
    fn get_total_requests(&self) -> u64 {
        self.l1_cache.values().map(|entry| entry.access_count).sum::<u64>() +
        self.l2_cache.values().map(|entry| entry.access_count).sum::<u64>()
    }

    /// 获取缓存命中数
    fn get_cache_hits(&self) -> u64 {
        self.l1_cache.values().map(|entry| entry.access_count).sum::<u64>() +
        self.l2_cache.values().map(|entry| entry.access_count).sum::<u64>()
    }
}

impl CacheEntry {
    /// 检查是否过期
    fn is_expired(&self) -> bool {
        self.timestamp.elapsed().as_secs() > self.ttl
    }
}

/// 磁盘操作
#[derive(Debug)]
pub enum DiskOperation {
    Read { key: String, path: PathBuf },
    Write { key: String, data: Vec<u8>, path: PathBuf },
}

impl Default for EdgeOptimizationConfig {
    fn default() -> Self {
        Self {
            memory_optimization: MemoryOptimizationConfig {
                enable_memory_pool: true,
                memory_pool_size_mb: 512,
                object_pool_size: 1000,
                enable_memory_mapped_files: true,
                memory_mapped_file_size_mb: 256,
                gc_threshold_mb: 1024,
            },
            disk_optimization: DiskOptimizationConfig {
                enable_disk_cache: true,
                cache_directory: "/tmp/edge_cache".to_string(),
                cache_size_mb: 1024,
                enable_async_io: true,
                io_thread_pool_size: 4,
                enable_read_ahead: true,
                read_ahead_size_kb: 64,
            },
            cpu_optimization: CpuOptimizationConfig {
                enable_cpu_affinity: true,
                cpu_cores: vec![0, 1, 2, 3],
                thread_pool_size: 4,
                enable_work_stealing: true,
                enable_priority_queue: true,
            },
            cache_optimization: CacheOptimizationConfig {
                enable_multi_level_cache: true,
                l1_cache_size: 1000,
                l2_cache_size: 10000,
                cache_ttl_seconds: 300,
                enable_cache_compression: true,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edge_optimization_config_default() {
        let config = EdgeOptimizationConfig::default();
        assert!(config.memory_optimization.enable_memory_pool);
        assert!(config.disk_optimization.enable_disk_cache);
        assert!(config.cpu_optimization.enable_cpu_affinity);
        assert!(config.cache_optimization.enable_multi_level_cache);
    }

    #[tokio::test]
    async fn test_memory_manager() {
        let mut manager = MemoryManager::new(1024 * 1024); // 1MB

        // 测试内存分配
        let buffer = manager.allocate_from_pool(1024);
        assert!(buffer.is_some());
        assert_eq!(buffer.unwrap().len(), 1024);
    }

    #[test]
    fn test_cache_entry_expiration() {
        let entry = CacheEntry {
            data: vec![1, 2, 3],
            timestamp: std::time::Instant::now(),
            ttl: 0, // 立即过期
            access_count: 0,
            last_access: std::time::Instant::now(),
        };

        // 由于TTL为0，应该立即过期
        assert!(entry.is_expired());
    }
}
