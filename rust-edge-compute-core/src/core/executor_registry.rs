//! Executor注册表
//!
//! 管理所有注册的executor，提供executor查找和选择功能

use crate::core::executor_trait::{Executor, ResourceRequirements};
use crate::core::{ComputeRequest, ComputeResponse};
use crate::core::error::Result;
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::RwLock;

/// Executor注册表
///
/// 管理所有注册的executor，提供executor查找和选择功能
pub struct ExecutorRegistry {
    /// Executor映射：名称 -> Executor
    executors: Arc<RwLock<HashMap<String, Arc<dyn Executor>>>>,
    /// 算法到executor的映射：算法名 -> Executor名称
    algorithm_map: Arc<RwLock<HashMap<String, String>>>,
}

impl ExecutorRegistry {
    /// 创建新的Executor注册表
    pub fn new() -> Self {
        Self {
            executors: Arc::new(RwLock::new(HashMap::new())),
            algorithm_map: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// 注册executor
    ///
    /// # Arguments
    /// * `executor` - 要注册的executor
    ///
    /// # Returns
    /// * `Result<()>` - 注册结果
    pub async fn register(&self, executor: Arc<dyn Executor>) -> Result<()> {
        let name = executor.name().to_string();
        let algorithms = executor.supported_algorithms();
        
        // 注册executor
        {
            let mut executors = self.executors.write().await;
            executors.insert(name.clone(), executor);
        }
        
        // 更新算法映射
        {
            let mut algorithm_map = self.algorithm_map.write().await;
            for algorithm in algorithms {
                algorithm_map.insert(algorithm, name.clone());
            }
        }
        
        Ok(())
    }
    
    /// 获取executor
    ///
    /// # Arguments
    /// * `name` - Executor名称
    ///
    /// # Returns
    /// * `Option<Arc<dyn Executor>>` - Executor实例
    pub async fn get(&self, name: &str) -> Option<Arc<dyn Executor>> {
        let executors = self.executors.read().await;
        executors.get(name).cloned()
    }
    
    /// 根据算法选择executor
    ///
    /// # Arguments
    /// * `algorithm` - 算法名称
    ///
    /// # Returns
    /// * `Option<Arc<dyn Executor>>` - 支持该算法的executor
    pub async fn select_executor(&self, algorithm: &str) -> Option<Arc<dyn Executor>> {
        // 首先从算法映射中查找
        let executor_name = {
            let algorithm_map = self.algorithm_map.read().await;
            algorithm_map.get(algorithm).cloned()
        };
        
        if let Some(name) = executor_name {
            return self.get(&name).await;
        }
        
        // 如果没有找到，遍历所有executor查找
        let executors = self.executors.read().await;
        for executor in executors.values() {
            if executor.supported_algorithms().contains(&algorithm.to_string()) {
                return Some(executor.clone());
            }
        }
        
        None
    }
    
    /// 列出所有注册的executor
    ///
    /// # Returns
    /// * `Vec<String>` - Executor名称列表
    pub async fn list_executors(&self) -> Vec<String> {
        let executors = self.executors.read().await;
        executors.keys().cloned().collect()
    }
    
    /// 获取executor统计信息
    ///
    /// # Returns
    /// * `HashMap<String, ExecutorStats>` - Executor统计信息
    pub async fn get_stats(&self) -> HashMap<String, ExecutorStats> {
        let executors = self.executors.read().await;
        let mut stats = HashMap::new();
        
        for (name, executor) in executors.iter() {
            stats.insert(
                name.clone(),
                ExecutorStats {
                    name: executor.name().to_string(),
                    version: executor.version().to_string(),
                    supported_algorithms: executor.supported_algorithms(),
                    resource_requirements: executor.resource_requirements(),
                },
            );
        }
        
        stats
    }
    
    /// 取消注册executor
    ///
    /// # Arguments
    /// * `name` - Executor名称
    ///
    /// # Returns
    /// * `Result<()>` - 取消注册结果
    pub async fn unregister(&self, name: &str) -> Result<()> {
        // 获取executor支持的算法
        let algorithms = {
            let executors = self.executors.read().await;
            if let Some(executor) = executors.get(name) {
                executor.supported_algorithms()
            } else {
                return Ok(()); // 如果不存在，直接返回
            }
        };
        
        // 移除executor
        {
            let mut executors = self.executors.write().await;
            executors.remove(name);
        }
        
        // 移除算法映射
        {
            let mut algorithm_map = self.algorithm_map.write().await;
            for algorithm in algorithms {
                algorithm_map.remove(&algorithm);
            }
        }
        
        Ok(())
    }
}

impl Default for ExecutorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Executor统计信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExecutorStats {
    /// Executor名称
    pub name: String,
    /// Executor版本
    pub version: String,
    /// 支持的算法列表
    pub supported_algorithms: Vec<String>,
    /// 资源需求
    pub resource_requirements: ResourceRequirements,
}

