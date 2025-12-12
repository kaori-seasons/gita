//! 全局内存分配器 - 追踪 Rust 堆内存使用
//!
//! 根据 docs/metrics.md 1.3.1 节实现，支持：
//! - Rust 堆内存追踪（包括 Vec、String、Box、Arc 等）
//! - Tokio 异步运行时内存追踪
//! - 实时内存统计导出到指标系统

use std::alloc::{GlobalAlloc, Layout};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::ptr::NonNull;

/// 全局 Rust 堆内存统计
static ALLOCATED_BYTES: AtomicUsize = AtomicUsize::new(0);
static ALLOCATION_COUNT: AtomicUsize = AtomicUsize::new(0);

/// 自定义全局分配器
pub struct MetricsAllocator;

unsafe impl GlobalAlloc for MetricsAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // 调用系统分配器
        let ret = std::alloc::System.alloc(layout);
        
        if !ret.is_null() {
            // 成功分配，更新统计
            ALLOCATED_BYTES.fetch_add(layout.size(), Ordering::Relaxed);
            ALLOCATION_COUNT.fetch_add(1, Ordering::Relaxed);
            
            // 每 1000 次分配记录一次日志（避免日志过多）
            let count = ALLOCATION_COUNT.load(Ordering::Relaxed);
            if count % 1000 == 0 {
                let total_bytes = ALLOCATED_BYTES.load(Ordering::Relaxed);
                tracing::debug!(
                    "Rust heap allocation: count={}, total_bytes={} MB",
                    count,
                    total_bytes / 1024 / 1024
                );
            }
        }
        
        ret
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // 调用系统分配器释放
        std::alloc::System.dealloc(ptr, layout);
        
        // 更新统计（减去释放的内存）
        ALLOCATED_BYTES.fetch_sub(layout.size(), Ordering::Relaxed);
        ALLOCATION_COUNT.fetch_sub(1, Ordering::Relaxed);
    }

    unsafe fn realloc(&self, ptr: *mut u8, old_layout: Layout, new_size: usize) -> *mut u8 {
        // 调用系统分配器重新分配
        let ret = std::alloc::System.realloc(ptr, old_layout, new_size);
        
        if !ret.is_null() {
            // 重新分配成功，更新统计
            let size_diff = new_size as isize - old_layout.size() as isize;
            
            if size_diff > 0 {
                // 增加内存
                ALLOCATED_BYTES.fetch_add(size_diff as usize, Ordering::Relaxed);
            } else if size_diff < 0 {
                // 减少内存
                ALLOCATED_BYTES.fetch_sub((-size_diff) as usize, Ordering::Relaxed);
            }
        }
        
        ret
    }
}

/// 获取当前 Rust 堆内存使用量（字节）
pub fn get_allocated_bytes() -> u64 {
    ALLOCATED_BYTES.load(Ordering::Relaxed) as u64
}

/// 获取当前分配计数
pub fn get_allocation_count() -> u64 {
    ALLOCATION_COUNT.load(Ordering::Relaxed) as u64
}

/// 重置统计（仅用于测试）
#[cfg(test)]
pub fn reset_stats() {
    ALLOCATED_BYTES.store(0, Ordering::Relaxed);
    ALLOCATION_COUNT.store(0, Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allocation_tracking() {
        reset_stats();
        
        // 分配一个 Vec
        let _v: Vec<u32> = (0..1000).collect();
        
        let allocated = get_allocated_bytes();
        assert!(allocated >= 4000, "Expected at least 4000 bytes allocated, got {}", allocated);
    }

    #[test]
    fn test_allocation_count() {
        reset_stats();
        
        let _v = Box::new(vec![1, 2, 3, 4, 5]);
        
        let count = get_allocation_count();
        assert!(count > 0, "Expected allocation count > 0, got {}", count);
    }
}
