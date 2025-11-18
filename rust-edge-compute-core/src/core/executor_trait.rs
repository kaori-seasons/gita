//! 统一Executor接口定义
//!
//! 定义所有executor必须实现的统一接口，确保不同executor可以无缝集成

use crate::core::{ComputeRequest, ComputeResponse};
use crate::core::error::Result;
use std::collections::HashMap;
use async_trait::async_trait;

/// 健康状态
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HealthStatus {
    /// 是否健康
    pub healthy: bool,
    /// 状态消息
    pub message: String,
    /// 详细信息
    pub details: HashMap<String, String>,
}

/// 统一的Executor trait
///
/// 所有executor（C++、Candle ML、Python WASM）都必须实现此trait
#[async_trait]
pub trait Executor: Send + Sync {
    /// 执行计算任务
    ///
    /// # Arguments
    /// * `request` - 计算任务请求
    ///
    /// # Returns
    /// * `Result<ComputeResponse>` - 计算任务响应
    async fn execute(&self, request: ComputeRequest) -> Result<ComputeResponse>;
    
    /// Executor名称
    ///
    /// # Returns
    /// * `&str` - Executor名称，如 "cpp", "candle-ml", "python-wasm"
    fn name(&self) -> &str;
    
    /// Executor版本
    ///
    /// # Returns
    /// * `&str` - Executor版本，如 "1.0.0"
    fn version(&self) -> &str;
    
    /// 支持的算法类型列表
    ///
    /// # Returns
    /// * `Vec<String>` - 支持的算法名称列表
    fn supported_algorithms(&self) -> Vec<String>;
    
    /// 资源需求
    ///
    /// # Returns
    /// * `ResourceRequirements` - 默认资源需求
    fn resource_requirements(&self) -> ResourceRequirements;
    
    /// 健康检查
    ///
    /// # Returns
    /// * `Result<HealthStatus>` - 健康状态
    async fn health_check(&self) -> Result<HealthStatus>;
}

/// 资源需求
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResourceRequirements {
    /// CPU核心数
    pub cpu_cores: f64,
    /// 内存需求（MB）
    pub memory_mb: u64,
    /// 磁盘空间需求（MB）
    pub disk_mb: Option<u64>,
    /// GPU内存需求（MB，可选）
    pub gpu_memory_mb: Option<u64>,
    /// 网络带宽需求（Mbps，可选）
    pub network_mbps: Option<u64>,
}

impl Default for ResourceRequirements {
    fn default() -> Self {
        Self {
            cpu_cores: 1.0,
            memory_mb: 512,
            disk_mb: Some(1024),
            gpu_memory_mb: None,
            network_mbps: None,
        }
    }
}

