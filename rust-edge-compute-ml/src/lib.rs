//! # Rust Edge Compute Framework - Candle ML Executor
//!
//! Candle ML算法执行器，提供机器学习模型推理能力

pub mod executor;
pub mod model_manager;
pub mod device_manager;

pub use executor::*;

