//! # Rust Edge Compute Framework - Core Library
//!
//! 核心库，包含框架的核心类型、错误定义、任务调度和统一Executor接口

pub mod core;
pub mod api;
pub mod config;
pub mod container;

/// 重新导出核心类型
pub use core::*;

/// 框架的主要错误类型
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

