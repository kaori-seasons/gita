//! # Rust Edge Compute Framework
//!
//! 基于Tokio、CXX与Youki的边缘计算框架
//!
//! ## 架构概述
//!
//! 本框架采用三层架构设计：
//! - **控制平面**: 基于Tokio的异步HTTP服务，负责任务调度和API接口
//! - **FFI层**: CXX桥接层，实现Rust与C++的安全互操作
//! - **执行平面**: Youki容器运行时，提供安全的算法执行环境

pub mod api;
pub mod config;
pub mod container;
pub mod core;
pub mod ffi;

// 条件导出cpp executor
#[cfg(feature = "cpp")]
pub use rust_edge_compute_cpp;

// 导出 core 模块中的子模块
pub use core::error;

/// 框架的核心类型和特性的重新导出
pub use core::*;

/// 框架的主要错误类型
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
