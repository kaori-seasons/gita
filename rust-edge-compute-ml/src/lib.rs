//! # Rust Edge Compute Framework - Candle ML Executor
//!
//! Candle ML算法执行器，提供机器学习模型推理能力

pub mod device_manager;
pub mod executor;
pub mod inference;
pub mod model_manager;
pub mod postprocessing;
pub mod preprocessing;

pub use executor::*;
pub use inference::*;
pub use postprocessing::*;
pub use preprocessing::*;
