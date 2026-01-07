//! 类型转换器模块
//!
//! 提供跨语言类型转换功能

use std::sync::Arc;

/// 类型转换器
pub struct TypeConverter;

impl TypeConverter {
    /// 创建带内存管理器的类型转换器
    pub fn with_memory_manager(_memory_manager: Arc<super::MemoryManager>) -> Self {
        Self
    }
}

/// 转换类型枚举
pub enum ConversionType {
    /// 自动转换
    Auto,
    /// 零拷贝转换
    ZeroCopy,
    /// 安全转换
    Safe,
}
