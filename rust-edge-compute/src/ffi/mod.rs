//! FFI模块 - 外部函数接口

pub mod bridge;
pub mod cpp_allocator;
pub mod exception_handler;
pub mod integration_example;
pub mod memory_manager;
pub mod memory_mapper;
pub mod performance_monitor;
pub mod type_converter;

// 重新导出常用的类型
pub use cpp_allocator::CppAllocator;
pub use exception_handler::ExceptionHandler;
pub use memory_manager::MemoryManager;
pub use memory_mapper::MemoryMapper;
pub use performance_monitor::PerformanceMonitor;
pub use type_converter::{ConversionType, TypeConverter};
