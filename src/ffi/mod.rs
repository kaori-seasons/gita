//! FFI桥接模块
//!
//! 提供Rust与C++的安全互操作接口

pub mod bridge;
pub mod memory_manager;
pub mod exception_handler;
pub mod type_converter;
pub mod performance_monitor;
pub mod integration_example;

pub use bridge::*;
pub use memory_manager::*;
pub use exception_handler::*;
pub use type_converter::*;
pub use performance_monitor::*;
pub use integration_example::*;

// TODO: 包含CXX生成的代码
// #[cxx::bridge]
// mod ffi { ... }
