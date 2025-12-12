//! # Rust Edge Compute Framework - Main Application Library
//!
//! 导出 FFI 模块供外部使用

pub mod core {
    pub use rust_edge_compute_core::core::*;
}

pub mod ffi;
