//! 生产级垃圾回收系统性能测试
//!
//! 测试分代GC、引用计数、并行GC等核心功能的性能表现

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use rust_edge_compute::streaming::{
    garbage_collector::{GarbageCollector, GCConfig, GCStrategy, GCTrigger, ObjectId},
    edge_optimization::{EdgeOptimizationManager, EdgeOptimizationConfig},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("🚀 生产级垃圾回收系统性能测试");
    println!("==================================");

    // 1. GC系统基础功能测试
    println!("\n📊 1. GC系统基础功能测试");
    basic_gc_test().await?;

    // 2. 分代GC性能测试
    println!("\n🔄 2. 分代GC性能测试");
    generational_gc_test().await?;

    // 3. 并行GC性能测试
    println!("\n⚡ 3. 并行GC性能测试");
    parallel_gc_test().await?;

    // 4. 引用计数测试
    println!("\n🔢 4. 引用计数测试");
    reference_counting_test().await?;

    // 5. 内存分配优化测试
    println!("\n💾 5. 内存分配优化测试");
    memory_allocation_test().await?;

    // 6. GC监控指标测试
    println!("\n📈 6. GC监控指标测试");
    gc_metrics_test().await?;

    println!("\n🎉 所有GC性能测试完成！");
    Ok(())
}

/// GC系统基础功能测试
async fn basic_gc_test() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("测试GC系统基础功能...");

    let config = GCConfig {
        enabled: true,
        strategy: GCStrategy::Generational,
        heap_size_mb: 256, // 小堆测试
        young_generation_ratio: 0.3,
        old_generation_ratio: 0.6,
        perm_generation_ratio: 0.1,
        gc_threshold_percent: 75,
        max_pause_time_ms: 100,
        parallel_gc_threads: 2,
        enable_incremental_gc: true,
        enable_reference_counting: true,
        enable_compacting_gc: true,
        log_level: rust_edge_compute::streaming::garbage_collector::GCLogLevel::Basic,
    };

    let gc = GarbageCollector::new(config)?;
    println!("✅ GC系统初始化成功");

    // 分配一些对象
    let mut objects = Vec::new();
    for i in 0..100 {
        let obj_id = gc.allocate(
            format!("TestClass{}", i),
            1024,
            vec![], // 没有引用
        ).await?;
        objects.push(obj_id);
    }
    println!("✅ 分配了100个对象");

    // 添加一些根对象
    for &obj_id in &objects[0..10] {
        gc.add_root(obj_id).await?;
    }
    println!("✅ 设置了10个根对象");

    // 触发GC
    let start = Instant::now();
    gc.trigger_gc(GCTrigger::YoungGC).await?;
    let duration = start.elapsed();
    println!("✅ 新生代GC完成，耗时: {:.2}ms", duration.as_millis());

    // 获取指标
    let metrics = gc.get_metrics().await;
    println!("📊 GC指标 - 总回收: {}, 暂停时间: {}ms",
             metrics.total_collections, metrics.average_pause_time_ms);

    Ok(())
}

/// 分代GC性能测试
async fn generational_gc_test() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("测试分代GC性能...");

    let config = GCConfig {
        enabled: true,
        strategy: GCStrategy::Generational,
        heap_size_mb: 512,
        young_generation_ratio: 0.3,
        old_generation_ratio: 0.6,
        perm_generation_ratio: 0.1,
        gc_threshold_percent: 70,
        max_pause_time_ms: 200,
        parallel_gc_threads: 2,
        enable_incremental_gc: true,
        enable_reference_counting: true,
        enable_compacting_gc: true,
        log_level: rust_edge_compute::streaming::garbage_collector::GCLogLevel::Basic,
    };

    let gc = GarbageCollector::new(config)?;

    // 创建不同生命周期的对象
    println!("创建短生命周期对象...");
    let mut short_lived = Vec::new();
    for i in 0..1000 {
        let obj_id = gc.allocate(format!("ShortLived{}", i), 256, vec![]).await?;
        short_lived.push(obj_id);
    }

    println!("创建长生命周期对象...");
    let mut long_lived = Vec::new();
    for i in 0..100 {
        let obj_id = gc.allocate(format!("LongLived{}", i), 1024, vec![]).await?;
        gc.add_root(obj_id).await?; // 设为根对象，防止被回收
        long_lived.push(obj_id);
    }

    // 多次触发新生代GC
    println!("执行多次新生代GC...");
    let mut young_gc_times = Vec::new();
    for i in 0..5 {
        let start = Instant::now();
        gc.trigger_gc(GCTrigger::YoungGC).await?;
        let duration = start.elapsed();
        young_gc_times.push(duration.as_millis());
        println!("第{}次新生代GC: {}ms", i + 1, duration.as_millis());
    }

    // 触发老生代GC
    println!("执行老生代GC...");
    let start = Instant::now();
    gc.trigger_gc(GCTrigger::OldGC).await?;
    let old_gc_time = start.elapsed();
    println!("老生代GC: {}ms", old_gc_time.as_millis());

    // 计算平均性能
    let avg_young_gc: f64 = young_gc_times.iter().sum::<u128>() as f64 / young_gc_times.len() as f64;
    println!("📊 平均新生代GC时间: {:.2}ms", avg_young_gc);
    println!("📊 老生代GC时间: {}ms", old_gc_time.as_millis());

    Ok(())
}

/// 并行GC性能测试
async fn parallel_gc_test() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("测试并行GC性能...");

    let config = GCConfig {
        enabled: true,
        strategy: GCStrategy::Generational,
        heap_size_mb: 1024,
        young_generation_ratio: 0.3,
        old_generation_ratio: 0.6,
        perm_generation_ratio: 0.1,
        gc_threshold_percent: 60,
        max_pause_time_ms: 300,
        parallel_gc_threads: 4, // 使用4个线程
        enable_incremental_gc: false,
        enable_reference_counting: true,
        enable_compacting_gc: true,
        log_level: rust_edge_compute::streaming::garbage_collector::GCLogLevel::Basic,
    };

    let gc = GarbageCollector::new(config)?;

    // 创建大量对象来触发GC
    println!("创建大量测试对象...");
    let mut objects = Vec::new();
    for i in 0..5000 {
        let obj_id = gc.allocate(format!("ParallelTest{}", i), 512, vec![]).await?;
        objects.push(obj_id);
    }

    // 只保留一部分作为根对象
    for &obj_id in &objects[0..500] {
        gc.add_root(obj_id).await?;
    }

    // 触发并行GC
    println!("执行并行GC...");
    let start = Instant::now();
    gc.trigger_gc(GCTrigger::FullGC).await?;
    let duration = start.elapsed();

    println!("✅ 并行GC完成，耗时: {:.2}ms", duration.as_millis());

    let metrics = gc.get_metrics().await;
    println!("📊 并行GC指标:");
    println!("   - 总回收次数: {}", metrics.total_collections);
    println!("   - 平均暂停时间: {:.2}ms", metrics.average_pause_time_ms);
    println!("   - 最大暂停时间: {}ms", metrics.max_pause_time_ms);
    println!("   - 回收对象数: {}", metrics.collected_objects);

    Ok(())
}

/// 引用计数测试
async fn reference_counting_test() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("测试引用计数功能...");

    let config = GCConfig {
        enabled: true,
        strategy: GCStrategy::Generational,
        heap_size_mb: 256,
        young_generation_ratio: 0.3,
        old_generation_ratio: 0.6,
        perm_generation_ratio: 0.1,
        gc_threshold_percent: 80,
        max_pause_time_ms: 100,
        parallel_gc_threads: 2,
        enable_incremental_gc: true,
        enable_reference_counting: true,
        enable_compacting_gc: false,
        log_level: rust_edge_compute::streaming::garbage_collector::GCLogLevel::Basic,
    };

    let gc = GarbageCollector::new(config)?;

    // 创建对象引用链
    println!("创建对象引用链...");

    // 创建根对象
    let root_obj = gc.allocate("RootObject".to_string(), 256, vec![]).await?;
    gc.add_root(root_obj).await?;

    // 创建子对象，引用根对象
    let mut child_objects = Vec::new();
    for i in 0..10 {
        let child_obj = gc.allocate(
            format!("ChildObject{}", i),
            128,
            vec![root_obj], // 引用根对象
        ).await?;
        child_objects.push(child_obj);
    }

    // 创建孙子对象，引用子对象
    let mut grandchild_objects = Vec::new();
    for (i, &child_obj) in child_objects.iter().enumerate() {
        let grandchild_obj = gc.allocate(
            format!("GrandchildObject{}", i),
            64,
            vec![child_obj], // 引用子对象
        ).await?;
        grandchild_objects.push(grandchild_obj);
    }

    println!("✅ 创建了引用链: 根对象 -> 子对象 -> 孙子对象");

    // 触发GC，应该保留所有有引用关系的对象
    gc.trigger_gc(GCTrigger::FullGC).await?;

    // 释放一些引用
    println!("释放部分引用...");
    for &obj_id in &child_objects[5..10] {
        gc.release_reference(obj_id).await?;
    }

    // 再次触发GC
    gc.trigger_gc(GCTrigger::FullGC).await?;

    let metrics = gc.get_metrics().await;
    println!("📊 引用计数测试结果:");
    println!("   - 回收对象数: {}", metrics.collected_objects);
    println!("   - 晋升对象数: {}", metrics.promoted_objects);

    Ok(())
}

/// 内存分配优化测试
async fn memory_allocation_test() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("测试内存分配优化...");

    let edge_config = EdgeOptimizationConfig::default();
    let edge_manager = EdgeOptimizationManager::new(edge_config).await?;

    println!("测试不同大小的内存分配...");

    let sizes = vec![64, 128, 256, 512, 1024, 2048, 4096, 8192];

    for &size in &sizes {
        let start = Instant::now();

        // 分配内存
        let _buffer = edge_manager.optimize_memory_allocation(size).await?;

        let duration = start.elapsed();
        println!("分配 {} 字节: {:.3}ms", size, duration.as_micros() as f64 / 1000.0);
    }

    // 测试连续分配
    println!("测试连续内存分配...");
    let mut allocations = Vec::new();
    let start = Instant::now();

    for i in 0..100 {
        let size = 1024 + (i % 10) * 128; // 变化的分配大小
        let buffer = edge_manager.optimize_memory_allocation(size).await?;
        allocations.push(buffer);
    }

    let total_time = start.elapsed();
    let avg_time = total_time.as_micros() as f64 / 100.0 / 1000.0;

    println!("✅ 连续分配100次完成");
    println!("📊 平均分配时间: {:.3}ms", avg_time);
    println!("📊 总分配时间: {:.2}ms", total_time.as_millis());

    Ok(())
}

/// GC监控指标测试
async fn gc_metrics_test() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("测试GC监控指标...");

    let config = GCConfig {
        enabled: true,
        strategy: GCStrategy::Generational,
        heap_size_mb: 512,
        young_generation_ratio: 0.3,
        old_generation_ratio: 0.6,
        perm_generation_ratio: 0.1,
        gc_threshold_percent: 70,
        max_pause_time_ms: 200,
        parallel_gc_threads: 2,
        enable_incremental_gc: true,
        enable_reference_counting: true,
        enable_compacting_gc: true,
        log_level: rust_edge_compute::streaming::garbage_collector::GCLogLevel::Basic,
    };

    let gc = GarbageCollector::new(config)?;

    // 执行一系列GC操作来产生指标数据
    println!("执行一系列GC操作...");

    for i in 0..10 {
        // 分配一些对象
        for j in 0..50 {
            let obj_id = gc.allocate(format!("MetricTest{}{}", i, j), 256, vec![]).await?;
            if j < 5 {
                gc.add_root(obj_id).await?; // 只保留少数对象
            }
        }

        // 触发GC
        gc.trigger_gc(GCTrigger::YoungGC).await?;
    }

    // 获取完整指标
    let metrics = gc.get_metrics().await;

    println!("📊 完整的GC监控指标:");
    println!("=========================================");
    println!("基础指标:");
    println!("  总回收次数: {}", metrics.total_collections);
    println!("  新生代回收: {}", metrics.young_collections);
    println!("  老生代回收: {}", metrics.old_collections);
    println!("  全堆回收: {}", metrics.full_collections);

    println!("\n性能指标:");
    println!("  总暂停时间: {}ms", metrics.total_pause_time_ms);
    println!("  平均暂停时间: {:.2}ms", metrics.average_pause_time_ms);
    println!("  最大暂停时间: {}ms", metrics.max_pause_time_ms);

    println!("\n内存指标:");
    println!("  堆使用量: {:.2}MB", metrics.heap_used_mb);
    println!("  堆总量: {:.2}MB", metrics.heap_total_mb);
    println!("  堆使用率: {:.2}%", (metrics.heap_used_mb / metrics.heap_total_mb) * 100.0);

    println!("\n回收指标:");
    println!("  GC效率: {:.2}%", metrics.gc_efficiency);
    println!("  晋升对象数: {}", metrics.promoted_objects);
    println!("  回收对象数: {}", metrics.collected_objects);
    println!("  内存碎片率: {:.2}%", metrics.fragmentation_ratio);

    // 计算一些派生指标
    let objects_per_collection = if metrics.total_collections > 0 {
        metrics.collected_objects as f64 / metrics.total_collections as f64
    } else {
        0.0
    };

    let pause_time_per_collection = if metrics.total_collections > 0 {
        metrics.total_pause_time_ms as f64 / metrics.total_collections as f64
    } else {
        0.0
    };

    println!("\n派生指标:");
    println!("  每次回收对象数: {:.1}", objects_per_collection);
    println!("  每次回收暂停时间: {:.2}ms", pause_time_per_collection);
    println!("  内存利用率: {:.2}%", (1.0 - metrics.fragmentation_ratio) * 100.0);

    // 性能评估
    println!("\n🎯 性能评估:");
    if metrics.average_pause_time_ms < 50.0 {
        println!("  ✅ GC暂停时间优秀 (<50ms)");
    } else if metrics.average_pause_time_ms < 100.0 {
        println!("  ✅ GC暂停时间良好 (<100ms)");
    } else {
        println!("  ⚠️  GC暂停时间较长 (>100ms)");
    }

    if metrics.gc_efficiency > 80.0 {
        println!("  ✅ GC效率优秀 (>80%)");
    } else if metrics.gc_efficiency > 60.0 {
        println!("  ✅ GC效率良好 (>60%)");
    } else {
        println!("  ⚠️  GC效率需要改进 (<60%)");
    }

    Ok(())
}
