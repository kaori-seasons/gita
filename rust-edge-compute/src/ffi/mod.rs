//! FFI桥接模块
//!
//! 提供Rust与C++的安全互操作接口

pub mod bridge;
pub mod memory_manager;
pub mod exception_handler;
pub mod type_converter;
pub mod performance_monitor;
pub mod integration_example;

// Cap'n Proto RPC 支持 (可选)
#[cfg(feature = "capnproto")]
pub mod capnp_service;

pub use bridge::*;
pub use memory_manager::*;
pub use exception_handler::*;
pub use type_converter::*;
pub use performance_monitor::*;
pub use integration_example::*;

#[cfg(feature = "capnproto")]
pub use capnp_service::*;

// TODO: 包含CXX生成的代码
// #[cxx::bridge]
// mod ffi { ... }
