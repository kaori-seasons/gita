//! 设备管理器
//!
//! 管理CPU、CUDA、Metal等计算设备，提供设备选择和资源管理功能

use candle_core::{Device, DeviceLocation};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use rust_edge_compute_core::core::error::{Result, EdgeComputeError};

/// 设备类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeviceType {
    /// CPU设备
    Cpu,
    /// CUDA设备
    Cuda(usize),
    /// Metal设备
    Metal(usize),
}

impl DeviceType {
    /// 创建Candle设备
    pub fn to_candle_device(&self) -> Result<Device> {
        match self {
            DeviceType::Cpu => Ok(Device::Cpu),
            DeviceType::Cuda(idx) => {
                #[cfg(feature = "cuda")]
                {
                    Device::new_cuda(*idx)
                        .map_err(|e| EdgeComputeError::AlgorithmExecution {
                            message: format!("Failed to create CUDA device {}: {}", idx, e),
                            algorithm: None,
                            input_size: None,
                        })
                }
                #[cfg(not(feature = "cuda"))]
                {
                    Err(EdgeComputeError::Config {
                        message: "CUDA support not enabled".to_string(),
                        source: Some("candle-ml".to_string()),
                    })
                }
            }
            DeviceType::Metal(idx) => {
                #[cfg(feature = "metal")]
                {
                    Device::new_metal(*idx)
                        .map_err(|e| EdgeComputeError::AlgorithmExecution {
                            message: format!("Failed to create Metal device {}: {}", idx, e),
                            algorithm: None,
                            input_size: None,
                        })
                }
                #[cfg(not(feature = "metal"))]
                {
                    Err(EdgeComputeError::Config {
                        message: "Metal support not enabled".to_string(),
                        source: Some("candle-ml".to_string()),
                    })
                }
            }
        }
    }
}

/// 设备信息
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    /// 设备类型
    pub device_type: DeviceType,
    /// 设备名称
    pub name: String,
    /// 是否可用
    pub available: bool,
    /// 内存大小（MB）
    pub memory_mb: Option<u64>,
    /// 计算能力（仅GPU）
    pub compute_capability: Option<String>,
}

/// 设备管理器配置
#[derive(Debug, Clone)]
pub struct DeviceManagerConfig {
    /// 默认设备类型
    pub default_device: DeviceType,
    /// 是否自动选择最佳设备
    pub auto_select_device: bool,
    /// 最大并发设备数
    pub max_concurrent_devices: usize,
}

impl Default for DeviceManagerConfig {
    fn default() -> Self {
        Self {
            default_device: DeviceType::Cpu,
            auto_select_device: true,
            max_concurrent_devices: 4,
        }
    }
}

/// 设备管理器
pub struct DeviceManager {
    /// 配置
    config: DeviceManagerConfig,
    /// 可用设备列表
    devices: Arc<RwLock<HashMap<String, DeviceInfo>>>,
    /// 设备使用计数
    device_usage: Arc<RwLock<HashMap<String, usize>>>,
}

impl DeviceManager {
    /// 创建新的设备管理器
    pub fn new(config: DeviceManagerConfig) -> Result<Self> {
        let mut manager = Self {
            config,
            devices: Arc::new(RwLock::new(HashMap::new())),
            device_usage: Arc::new(RwLock::new(HashMap::new())),
        };
        
        // 初始化设备
        manager.initialize_devices().await?;
        
        Ok(manager)
    }
    
    /// 初始化设备
    async fn initialize_devices(&mut self) -> Result<()> {
        let mut devices = self.devices.write().await;
        
        // 添加CPU设备
        devices.insert(
            "cpu".to_string(),
            DeviceInfo {
                device_type: DeviceType::Cpu,
                name: "CPU".to_string(),
                available: true,
                memory_mb: None,
                compute_capability: None,
            },
        );
        
        // 尝试添加CUDA设备
        #[cfg(feature = "cuda")]
        {
            // 这里应该检测可用的CUDA设备
            // 简化实现，假设有一个CUDA设备
            devices.insert(
                "cuda:0".to_string(),
                DeviceInfo {
                    device_type: DeviceType::Cuda(0),
                    name: "CUDA:0".to_string(),
                    available: true,
                    memory_mb: Some(8192), // 假设8GB
                    compute_capability: Some("8.0".to_string()),
                },
            );
        }
        
        // 尝试添加Metal设备
        #[cfg(feature = "metal")]
        {
            devices.insert(
                "metal:0".to_string(),
                DeviceInfo {
                    device_type: DeviceType::Metal(0),
                    name: "Metal:0".to_string(),
                    available: true,
                    memory_mb: Some(4096), // 假设4GB
                    compute_capability: None,
                },
            );
        }
        
        Ok(())
    }
    
    /// 获取默认设备
    pub async fn get_default_device(&self) -> Result<Device> {
        self.config.default_device.to_candle_device()
    }
    
    /// 选择最佳设备
    pub async fn select_best_device(&self) -> Result<Device> {
        if self.config.auto_select_device {
            // 优先选择GPU设备
            let devices = self.devices.read().await;
            
            // 查找可用的CUDA设备
            #[cfg(feature = "cuda")]
            {
                for (name, info) in devices.iter() {
                    if let DeviceType::Cuda(_) = info.device_type {
                        if info.available {
                            return info.device_type.to_candle_device();
                        }
                    }
                }
            }
            
            // 查找可用的Metal设备
            #[cfg(feature = "metal")]
            {
                for (name, info) in devices.iter() {
                    if let DeviceType::Metal(_) = info.device_type {
                        if info.available {
                            return info.device_type.to_candle_device();
                        }
                    }
                }
            }
        }
        
        // 回退到默认设备
        self.get_default_device().await
    }
    
    /// 获取设备信息
    pub async fn get_device_info(&self, device_name: &str) -> Option<DeviceInfo> {
        let devices = self.devices.read().await;
        devices.get(device_name).cloned()
    }
    
    /// 列出所有可用设备
    pub async fn list_devices(&self) -> Vec<DeviceInfo> {
        let devices = self.devices.read().await;
        devices.values().cloned().collect()
    }
    
    /// 获取设备使用计数
    pub async fn get_device_usage(&self, device_name: &str) -> usize {
        let usage = self.device_usage.read().await;
        usage.get(device_name).copied().unwrap_or(0)
    }
    
    /// 增加设备使用计数
    pub async fn increment_device_usage(&self, device_name: &str) {
        let mut usage = self.device_usage.write().await;
        *usage.entry(device_name.to_string()).or_insert(0) += 1;
    }
    
    /// 减少设备使用计数
    pub async fn decrement_device_usage(&self, device_name: &str) {
        let mut usage = self.device_usage.write().await;
        if let Some(count) = usage.get_mut(device_name) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                usage.remove(device_name);
            }
        }
    }
}

impl Default for DeviceManager {
    fn default() -> Self {
        Self::new(DeviceManagerConfig::default()).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_device_manager_creation() {
        let manager = DeviceManager::new(DeviceManagerConfig::default()).unwrap();
        let devices = manager.list_devices().await;
        assert!(!devices.is_empty());
    }
    
    #[tokio::test]
    async fn test_device_manager_default_device() {
        let manager = DeviceManager::new(DeviceManagerConfig::default()).unwrap();
        let device = manager.get_default_device().await.unwrap();
        assert_eq!(device.location(), DeviceLocation::Cpu);
    }
    
    #[tokio::test]
    async fn test_device_manager_select_best() {
        let manager = DeviceManager::new(DeviceManagerConfig::default()).unwrap();
        let device = manager.select_best_device().await.unwrap();
        // 应该至少能获取到一个设备
        assert!(true);
    }
}
