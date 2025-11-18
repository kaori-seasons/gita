//! # Rust Edge Compute Framework - Python WASM Executor
//!
//! Python算法执行器，支持WASM沙箱和PyO3集成

pub mod executor;
pub mod wasm;
pub mod python;

pub use executor::*;

