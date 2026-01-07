# Gita 指标监控系统文档

## 总体概述

Gita 提供了一套完整的指标监控系统，用于实时跟踪系统性能、诊断问题和优化资源利用。这份文档介绍了系统中记录的所有指标、如何访问这些指标，以及如何使用它们进行监控和告警。

---

## 指标体系架构

```
┌─────────────────────────────────────────┐
│    应用层指标 (Application Metrics)      │
│ • 任务调度 • API请求 • 算法执行         │
└──────────────┬──────────────────────────┘
               │
┌──────────────▼──────────────────────────┐
│    系统层指标 (System Metrics)           │
│ • CPU • 内存 • 连接 • 队列              │
└──────────────┬──────────────────────────┘
               │
┌──────────────▼──────────────────────────┐
│   指标收集器 (Metrics Collector)        │
│ • 原子操作 • 线程安全 • 高性能          │
└──────────────┬──────────────────────────┘
               │
┌──────────────▼──────────────────────────┐
│   导出器 (Exporters)                    │
│ • Prometheus • JSON • Grafana           │
└─────────────────────────────────────────┘
```

---

## 一、核心指标分类

### 1.1 任务调度指标

#### 活跃任务数 (Active Tasks)
- **指标名**: `scheduler_active_tasks`
- **类型**: Gauge（仪表）
- **单位**: 个
- **说明**: 当前正在执行的任务数
- **告警阈值**: > 50 时告警
- **Prometheus 查询**:
  ```promql
  scheduler_active_tasks
  ```

#### 排队任务数 (Queued Tasks)
- **指标名**: `scheduler_queued_tasks`
- **类型**: Gauge
- **单位**: 个
- **说明**: 等待执行的任务数
- **告警阈值**: > 1000 时告警（队列即将满）
- **Prometheus 查询**:
  ```promql
  scheduler_queued_tasks
  ```

#### 任务完成率 (Task Completion Rate)
- **指标名**: `scheduler_task_completion_rate`
- **类型**: Counter（计数器）
- **单位**: 任务/秒
- **说明**: 每秒完成的任务数
- **计算公式**: 
  ```
  rate(scheduler_tasks_completed_total[5m])
  ```

#### 任务超时数 (Task Timeouts)
- **指标名**: `scheduler_task_timeouts_total`
- **类型**: Counter
- **单位**: 个
- **说明**: 超时的任务总数（累计）
- **告警**: 5分钟内超时数 > 10 时告警

#### 任务重试次数 (Task Retries)
- **指标名**: `scheduler_task_retries_total`
- **类型**: Counter
- **单位**: 个
- **说明**: 重试的任务总数
- **标签**: `retry_reason` (timeout, error, etc.)

### 1.2 HTTP 请求指标

#### 请求延迟 (Request Latency)
- **指标名**: `http_request_duration_seconds`
- **类型**: Histogram（直方图）
- **单位**: 秒
- **说明**: HTTP请求处理时间分布
- **标签**: 
  - `method`: GET, POST, PUT, DELETE 等
  - `path`: 请求路径
  - `status`: HTTP状态码
- **Prometheus 查询**:
  ```promql
  # 平均延迟
  rate(http_request_duration_seconds_sum[5m]) / rate(http_request_duration_seconds_count[5m])
  
  # P95 延迟
  histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m]))
  
  # P99 延迟
  histogram_quantile(0.99, rate(http_request_duration_seconds_bucket[5m]))
  ```

#### HTTP 请求总数 (HTTP Requests)
- **指标名**: `http_requests_total`
- **类型**: Counter
- **单位**: 个
- **说明**: 处理的HTTP请求总数
- **标签**:
  - `method`: 请求方法
  - `path`: 请求路径
  - `status`: 状态码

#### 错误请求数 (Error Requests)
- **指标名**: `http_errors_total`
- **类型**: Counter
- **单位**: 个
- **说明**: 返回错误状态码的请求数（5xx）
- **标签**: `method`, `path`, `status`

#### 活跃连接数 (Active Connections)
- **指标名**: `http_connections_active`
- **类型**: Gauge
- **单位**: 个
- **说明**: 当前活跃的HTTP连接数
- **告警阈值**: > 500 时告警

### 1.3 系统资源指标

#### CPU 使用率 (CPU Usage)
- **指标名**: `system_cpu_usage_percent`
- **类型**: Gauge
- **单位**: % (0-100)
- **说明**: 进程的CPU使用百分比
- **数据来源**: 读取 `/proc/stat` 和 `/proc/<pid>/stat`
- **告警阈值**: > 80% 时告警
- **Prometheus 查询**:
  ```promql
  system_cpu_usage_percent
  ```

#### 内存使用量 (Memory Usage)
- **指标名**: `system_memory_bytes`
- **类型**: Gauge
- **单位**: 字节
- **说明**: 进程的物理内存（RSS）使用量
- **数据来源**: 读取 `/proc/<pid>/status` 中的 VmRSS
- **告警阈值**: > 1GB 时告警
- **Prometheus 查询**:
  ```promql
  system_memory_bytes / 1024 / 1024  # 转换为 MB
  ```

#### 内存使用率 (Memory Usage %)
- **指标名**: `system_memory_usage_percent`
- **类型**: Gauge
- **单位**: % (0-100)
- **说明**: 相对于系统总内存的使用百分比
- **数据来源**: (总内存 - 可用内存) / 总内存
- **告警阈值**: > 70% 时告警

### 1.3.1 精细化内存监控（Rust vs C++）

Gita 系统中包含 Rust 和 C++ 两部分代码，需要分别追踪它们的内存使用情况。本节提供详细的内存监控指标和方法。

#### Rust 堆内存 (Rust Heap Memory)
- **指标名**: `memory_rust_heap_bytes`
- **类型**: Gauge
- **单位**: 字节
- **说明**: Rust 运行时管理的堆内存总量
- **数据来源**: 通过 `GlobalAllocator` 追踪，或使用工具如 valgrind/heaptrack
- **包含内容**:
  - Vec、String 等数据结构的分配
  - 异步运行时（Tokio）的内存
  - 通过 Box、Arc 等智能指针的分配
- **告警阈值**: > 512MB 时告警
- **Prometheus 查询**:
  ```promql
  # Rust 堆内存使用趋势
  rate(memory_rust_heap_bytes[5m])
  ```

#### C++ 堆内存 (C++ Heap Memory)
- **指标名**: `memory_cpp_heap_bytes`
- **类型**: Gauge
- **单位**: 字节
- **说明**: FFI 调用的 C++ 代码分配的堆内存
- **数据来源**: 通过 `MemoryManager::allocate_cpp()` 和 `MemoryManager::deallocate_cpp()` 追踪
- **包含内容**:
  - C++ new/delete 分配的内存
  - std::vector、std::string 等标准容器
  - 第三方库（OpenCV、算法库等）分配的内存
- **告警阈值**: > 256MB 时告警
- **监控方法**:
  ```rust
  // 在 FFI 调用前后记录
  let cpp_allocator = CppAllocator::new();
  let before = cpp_allocator.get_allocator_stats().await.active_allocated_bytes;
  
  // 执行 C++ 代码
  ffi_call();
  
  let after = cpp_allocator.get_allocator_stats().await.active_allocated_bytes;
  let cpp_memory_delta = after - before;  // C++ 新分配的内存
  ```

#### 共享内存 (Shared Memory)
- **指标名**: `memory_shared_bytes`
- **类型**: Gauge
- **单位**: 字节
- **说明**: Rust 和 C++ 之间共享的内存（通过 FFI 传递的数据）
- **数据来源**: 在 `MemoryManager` 中标记为 `MemoryType::Shared` 的块
- **包含内容**:
  - 图像缓冲区（通常是共享的大块内存）
  - 跨语言边界的数据结构
  - 内存映射文件
- **告警阈值**: > 100MB 时告警
- **特点**:
  - 需要特别关注生命周期管理
  - 避免重复计算（既不算 Rust 也不算 C++）
  - 记录所有权边界

#### 虚拟内存 (Virtual Memory)
- **指标名**: `memory_vm_total_bytes`
- **类型**: Gauge
- **单位**: 字节
- **说明**: 进程的总虚拟内存，包括尚未分配的内存
- **数据来源**: 读取 `/proc/<pid>/status` 中的 VmSize
- **重要性**:
  - 检测内存泄漏（VmSize 持续增长但 VmRSS 不变）
  - 监控内存碎片化
  - 容器环境中的资源限制管理

#### 常驻内存 (Resident Set Size, RSS)
- **指标名**: `memory_rss_bytes`
- **类型**: Gauge
- **单位**: 字节
- **说明**: 进程实际使用的物理内存
- **数据来源**: `/proc/<pid>/status` 中的 VmRSS
- **分解方法**:
  ```
  VmRSS = Rust堆 + C++堆 + 共享内存 + 代码段 + 栈 + 库文件等
  ```

#### 内存映射 (Memory Mapping)
- **指标名**: `memory_mapped_bytes`
- **类型**: Gauge
- **单位**: 字节
- **说明**: 通过内存映射文件或 mmap 分配的内存
- **数据来源**: `/proc/<pid>/maps` 中的 [heap]、[stack]、文件映射等
- **用途**:
  - 大文件处理（如模型文件）
  - 进程间通信
  - 高效的缓冲区管理

#### 队列长度 (Queue Length)
- **指标名**: `system_queue_length`
- **类型**: Gauge
- **单位**: 个
- **说明**: 当前等待处理的任务队列长度
- **告警阈值**: > 5000 时告警

### 1.4 性能指标

#### 平均响应时间 (Average Response Time)
- **指标名**: `performance_avg_response_time_ms`
- **类型**: Gauge
- **单位**: 毫秒
- **说明**: 所有请求的平均响应时间
- **告警阈值**: > 500ms 时告警
- **计算方式**: 
  ```
  Σ(响应时间) / 请求总数
  ```

#### P95 响应时间 (P95 Response Time)
- **指标名**: `performance_p95_response_time_ms`
- **类型**: Gauge
- **单位**: 毫秒
- **说明**: 95% 的请求响应时间
- **告警阈值**: > 1000ms 时告警

#### P99 响应时间 (P99 Response Time)
- **指标名**: `performance_p99_response_time_ms`
- **类型**: Gauge
- **单位**: 毫秒
- **说明**: 99% 的请求响应时间
- **告警阈值**: > 2000ms 时告警

#### 吞吐量 (Throughput)
- **指标名**: `performance_throughput_rps`
- **类型**: Gauge
- **单位**: 请求/秒
- **说明**: 每秒处理的请求数
- **计算方式**: 
  ```
  总请求数 / 运行时间(秒)
  ```

#### 错误率 (Error Rate)
- **指标名**: `performance_error_rate_percent`
- **类型**: Gauge
- **单位**: % (0-100)
- **说明**: 失败请求的百分比
- **告警阈值**: > 1% 时告警
- **计算方式**: 
  ```
  (错误总数 / 请求总数) × 100
  ```

### 1.5 FFI（跨语言互操作）指标

#### FFI 调用次数 (FFI Calls)
- **指标名**: `ffi_calls_total`
- **类型**: Counter
- **单位**: 个
- **说明**: 从 Rust 调用 C++ 的总次数
- **标签**: 
  - `function`: 函数名称
  - `status`: success, error

#### FFI 调用延迟 (FFI Call Duration)
- **指标名**: `ffi_call_duration_milliseconds`
- **类型**: Histogram
- **单位**: 毫秒
- **说明**: C++ 函数执行时间
- **告警阈值**: P95 > 1000ms 时告警

#### FFI 错误数 (FFI Errors)
- **指标名**: `ffi_errors_total`
- **类型**: Counter
- **单位**: 个
- **说明**: FFI 调用失败总数
- **标签**: 
  - `function`: 函数名称
  - `error_type`: 错误类型

### 1.6 容器指标

#### 活跃容器数 (Active Containers)
- **指标名**: `container_instances_active`
- **类型**: Gauge
- **单位**: 个
- **说明**: 当前运行的容器数
- **告警阈值**: > 100 时告警

#### 容器创建总数 (Container Creations)
- **指标名**: `container_instances_created_total`
- **类型**: Counter
- **单位**: 个
- **说明**: 创建的容器总数（生命周期）

#### 容器执行时间 (Container Execution)
- **指标名**: `container_execution_duration_seconds`
- **类型**: Histogram
- **单位**: 秒
- **说明**: 容器中任务的执行时间

---

## 二、访问指标的方式

### 2.1 通过 HTTP API

#### 获取最新指标
```bash
curl -H "Authorization: Bearer <token>" \
  http://localhost:3000/api/v1/metrics
```

响应示例:
```json
{
  "timestamp": "2024-12-08T10:30:00Z",
  "scheduler": {
    "active_tasks": 8,
    "queued_tasks": 45,
    "completion_rate": 12.3
  },
  "system": {
    "cpu_usage_percent": 25.3,
    "memory_usage_bytes": 524288000,
    "memory_usage_percent": 31.2
  },
  "http": {
    "requests_total": 10542,
    "errors_total": 45,
    "avg_response_time_ms": 125.6
  },
  "performance": {
    "p95_response_time_ms": 450.2,
    "p99_response_time_ms": 890.1,
    "throughput_rps": 25.5,
    "error_rate_percent": 0.42
  }
}
```

#### 获取性能统计
```bash
curl -H "Authorization: Bearer <token>" \
  http://localhost:3000/api/v1/performance/stats
```

#### 获取错误统计
```bash
curl -H "Authorization: Bearer <token>" \
  http://localhost:3000/api/v1/errors/stats
```

### 2.2 通过 Prometheus

#### 查询端点
```
http://localhost:9090/
```

#### 常用查询示例

**1. 查看当前 CPU 使用率**
```promql
system_cpu_usage_percent
```

**2. 查看内存使用趋势（最近1小时）**
```promql
system_memory_bytes{job="gita"} / 1024 / 1024
```

**3. 查看任务完成速率（5分钟平均）**
```promql
rate(scheduler_tasks_completed_total[5m])
```

**4. 查看错误率**
```promql
rate(http_errors_total[5m]) / rate(http_requests_total[5m])
```

**5. 查看 P95 请求延迟**
```promql
histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m]))
```

**6. 查看活跃任务数变化**
```promql
scheduler_active_tasks
```

**7. 查看每个 endpoint 的错误率**
```promql
rate(http_errors_total{job="gita"}[5m]) by (path)
```

### 2.3 通过 Grafana

#### 导入仪表板
1. 访问 Grafana: http://localhost:3001
2. 登录 (admin/admin)
3. 创建新仪表板或导入预制仪表板
4. 添加 Prometheus 数据源

#### 推荐的仪表板配置

**仪表板 1: 系统概览**
- 活跃任务数
- CPU 使用率
- 内存使用率
- 错误率趋势

**仪表板 2: API 性能**
- 请求吞吐量（RPS）
- P95/P99 延迟
- 错误分布（按状态码）
- 慢查询（>1秒的请求）

**仪表板 3: 深度诊断**
- CPU 使用率历史
- 内存泄漏检测（内存持续增长）
- 任务超时趋势
- FFI 调用失败率

---

## 三、关键告警规则

### 3.1 高优先级告警

#### 1. 高错误率
```yaml
alert: HighErrorRate
expr: rate(http_errors_total[5m]) / rate(http_requests_total[5m]) > 0.05
for: 5m
annotations:
  summary: "错误率超过 5%"
  description: "当前错误率: {{ $value | humanizePercentage }}"
```

#### 2. 高 CPU 使用率
```yaml
alert: HighCPUUsage
expr: system_cpu_usage_percent > 80
for: 5m
annotations:
  summary: "CPU 使用率过高"
  description: "CPU 使用率: {{ $value }}%"
```

#### 3. 高内存使用率
```yaml
alert: HighMemoryUsage
expr: system_memory_usage_percent > 80
for: 5m
annotations:
  summary: "内存使用率过高"
  description: "内存使用率: {{ $value }}%"
```

#### 4. 队列堆积
```yaml
alert: QueueBacklog
expr: scheduler_queued_tasks > 5000
for: 10m
annotations:
  summary: "任务队列堆积"
  description: "队列长度: {{ $value }} 个任务"
```

### 3.2 中等优先级告警

#### 1. 响应时间过长
```yaml
alert: HighLatency
expr: histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m])) > 1
for: 10m
annotations:
  summary: "P95 响应时间 > 1 秒"
  description: "P95 延迟: {{ $value | humanizeDuration }}"
```

#### 2. 任务超时频繁
```yaml
alert: FrequentTimeouts
expr: rate(scheduler_task_timeouts_total[5m]) > 1
for: 5m
annotations:
  summary: "任务频繁超时"
  description: "超时速率: {{ $value }}/s"
```

#### 3. FFI 调用失败
```yaml
alert: FFICallFailure
expr: rate(ffi_errors_total[5m]) > 0.1
for: 5m
annotations:
  summary: "FFI 调用失败率高"
  description: "失败速率: {{ $value }}/s"
```

---

## 四、实战示例

### 4.1 性能基准测试

```bash
# 发送基准测试请求
for i in {1..1000}; do
  curl -X POST http://localhost:3000/api/v1/compute \
    -H "Authorization: Bearer <token>" \
    -H "Content-Type: application/json" \
    -d '{"algorithm": "add", "parameters": {"a": 1, "b": 2}}' \
    &
done
wait

# 查看结果
curl -H "Authorization: Bearer <token>" \
  http://localhost:3000/api/v1/metrics
```

### 4.2 负载测试脚本

```bash
#!/bin/bash

DURATION=300  # 5分钟
CONCURRENT=50
TARGET="http://localhost:3000"

# 使用 Apache Bench
ab -n 10000 -c $CONCURRENT -H "Authorization: Bearer <token>" \
  "$TARGET/api/v1/health"

# 检查指标
curl -H "Authorization: Bearer <token>" \
  "$TARGET/api/v1/metrics"
```

### 4.3 内存泄漏检测

在 Grafana 中添加查询:
```promql
# 计算内存增长趋势（每小时增加多少）
rate(system_memory_bytes[1h])

# 如果值持续为正，表示可能有内存泄漏
```

### 4.4 错误分析

```bash
# 按错误类型分组统计
curl -H "Authorization: Bearer <token>" \
  http://localhost:3000/api/v1/errors/stats | \
  jq '.error_breakdown | sort_by(.count) | reverse'
```

---

## 五、最佳实践

### 5.1 指标采集

1. **采样间隔**: 系统指标每30秒采集一次，足以捕捉异常
2. **数据保留**: Prometheus 默认保留 15 天数据，可根据需要调整
3. **高基数标签**: 避免在标签中使用高基数值（如用户ID），会增加存储压力

### 5.2 告警策略

1. **分级告警**: 
   - P0（立即处理）: 服务不可用、数据丢失
   - P1（30分钟内）: 错误率 > 5%、响应时间 > 2s
   - P2（1小时内）: CPU > 85%、内存 > 80%

2. **告警窗口**: 避免瞬间告警，使用至少 5 分钟的窗口

3. **去噪**: 设置合理的阈值，减少误告警

### 5.3 内存监控最佳实践

#### 5.3.1 Rust 内存监控

**收集策略**:
```rust
// 方式1: 使用 GlobalAllocator 追踪（推荐用于生产环境）
use std::alloc::{GlobalAlloc, Layout};
use std::sync::atomic::{AtomicUsize, Ordering};

static ALLOCATED: AtomicUsize = AtomicUsize::new(0);

struct TrackingAllocator;

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATED.fetch_add(layout.size(), Ordering::SeqCst);
        std::alloc::System.alloc(layout)
    }
    
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        ALLOCATED.fetch_sub(layout.size(), Ordering::SeqCst);
        std::alloc::System.dealloc(ptr, layout)
    }
}

#[global_allocator]
static GLOBAL: TrackingAllocator = TrackingAllocator;

pub fn get_rust_memory_usage() -> usize {
    ALLOCATED.load(Ordering::Relaxed)
}
```

**关键指标**:
- **堆内存分配速度**: 监控 `memory_rust_heap_bytes` 的变化率
- **峰值内存**: 记录运行期间的最大值
- **内存碎片**: VmSize / VmRSS 的比值（> 1.5 表示碎片化）

**告警规则**:
```yaml
# Rust 内存增长过快
alert: RustMemoryGrowth
expr: rate(memory_rust_heap_bytes[5m]) > 10485760  # > 10MB/5m
for: 10m

# Rust 内存峰值超过阈值
alert: RustMemoryPeak
expr: memory_rust_heap_bytes > 536870912  # > 512MB
for: 5m
```

#### 5.3.2 C++ 内存监控

**收集策略**:
```rust
// 方式1: 通过 MemoryManager 追踪 FFI 分配
let cpp_allocator = CppAllocator::new();

// FFI 调用前后
let before = cpp_allocator.get_allocator_stats().await.active_allocated_bytes;
// ... 执行 FFI 调用 ...
let after = cpp_allocator.get_allocator_stats().await.active_allocated_bytes;

let cpp_delta = after as i64 - before as i64;

// 方式2: 在 C++ 代码中用自定义分配器
// 在 C++ 侧实现 new/delete hook 或使用 jemalloc
```

**关键指标**:
- **活跃分配数**: `cpp_allocator.active_allocations`
- **总分配字节数**: `cpp_allocator.total_allocated_bytes`
- **平均分配时间**: `cpp_allocator.avg_allocation_time_ms`
- **分配失败率**: `cpp_allocator.allocation_failures / total_allocations`

**告警规则**:
```yaml
# C++ 内存超出阈值
alert: CppMemoryOverThreshold
expr: memory_cpp_heap_bytes > 268435456  # > 256MB
for: 5m

# 频繁的内存分配/释放（可能导致性能问题）
alert: CppAllocationThrashing
expr: rate(cpp_allocator_total_allocations[1m]) > 10000
for: 5m
```

#### 5.3.3 Rust vs C++ 内存对比

**同时监控两部分**:
```promql
# Rust 占总内存的比例
ratio_rust = memory_rust_heap_bytes / (memory_rust_heap_bytes + memory_cpp_heap_bytes)

# C++ 占总内存的比例
ratio_cpp = memory_cpp_heap_bytes / (memory_rust_heap_bytes + memory_cpp_heap_bytes)

# 内存分配偏向性
# 值接近 0.5 表示均衡
# 值 < 0.3 表示 C++ 占用过多
# 值 > 0.7 表示 Rust 占用过多
allocation_bias = ratio_cpp
```

**Grafana 仪表板配置**:
```json
{
  "panels": [
    {
      "title": "Rust vs C++ 内存使用",
      "targets": [
        {
          "legendFormat": "Rust Heap",
          "expr": "memory_rust_heap_bytes / 1024 / 1024"
        },
        {
          "legendFormat": "C++ Heap",
          "expr": "memory_cpp_heap_bytes / 1024 / 1024"
        },
        {
          "legendFormat": "Shared",
          "expr": "memory_shared_bytes / 1024 / 1024"
        }
      ],
      "type": "graph"
    }
  ]
}
```

#### 5.3.4 共享内存管理

**最佳实践**:
1. **明确所有权**: 在代码中清楚标记哪部分内存由谁拥有
   ```rust
   // 示例：图像缓冲区的所有权管理
   pub struct ImageBuffer {
       owner: MemoryOwner,  // Rust, Cpp, or Shared
       data_ptr: *mut u8,
       size: usize,
   }
   
   enum MemoryOwner {
       Rust,
       Cpp,
       Shared { ref_count: Arc<AtomicUsize> },
   }
   ```

2. **避免双重计算**: 在计算总内存时排除共享内存
   ```
   Total = Rust + Cpp + Shared
   Total != (RSS - 共享部分)
   ```

3. **追踪共享数据生命周期**:
   ```rust
   // 记录共享内存的创建和销毁
   fn track_shared_allocation(size: usize) {
       SHARED_MEMORY_TRACKER.allocate(size);
   }
   
   fn track_shared_deallocation(size: usize) {
       SHARED_MEMORY_TRACKER.deallocate(size);
   }
   ```

### 5.4 监控清单

定期检查以下指标:

- [ ] **Rust 内存**:
  - [ ] 堆内存使用 < 512MB
  - [ ] 内存分配速率稳定
  - [ ] 无内存泄漏（持续增长）
  
- [ ] **C++ 内存**:
  - [ ] 堆内存使用 < 256MB
  - [ ] 分配/释放配对
  - [ ] FFI 调用不产生内存泄漏
  
- [ ] **共享内存**:
  - [ ] 使用量 < 100MB
  - [ ] 所有权清晰
  - [ ] 无悬空指针
  
- [ ] **整体内存**:
  - [ ] RSS 增长率 < 10MB/小时
  - [ ] VmSize 稳定
  - [ ] 内存碎片化 < 50%
  - [ ] 错误率 < 1%
  - [ ] P95 响应时间 < 500ms
  - [ ] CPU 使用率 < 60%
  - [ ] 任务队列 < 100
  - [ ] FFI 调用成功率 > 99%
  - [ ] 容器创建稳定（无频繁创建/销毁

---

## 六、故障排查指南

### 问题 1: 高错误率

**症状**: `http_errors_total` 增长迅速

**排查步骤**:
1. 查看错误日志: `curl http://localhost:3000/api/v1/errors/stats`
2. 按 endpoint 查看错误分布
3. 检查依赖服务是否正常
4. 查看 FFI 调用是否有问题

### 问题 2: 内存持续增长

**症状**: `system_memory_bytes` 和 `memory_rust_heap_bytes` 或 `memory_cpp_heap_bytes` 之一持续增长

**分签飘移排查**:

#### 方案 A: Rust 侧泄漏（memory_rust_heap_bytes 持续增长）
1. 检查是否有不阀用的 Vec 积累
   ```rust
   // 问题代码
   let mut large_vec = Vec::new();
   loop {
       large_vec.push(data);  // 没有 truncate
   }
   
   // 修复
   large_vec.truncate(max_capacity);
   ```

2. 检查异步任务是否未完成
   ```rust
   // 查齐等待之前没有 drop
   let handle = tokio::spawn(async { ... });
   // 需要等待或 drop
   handle.await?;
   ```

3. 使用 heaptrack 或 valgrind 检测
   ```bash
   # 庅选（效率高）
   heaptrack ./target/release/gita
   heaptrack gita_result.heaptrack.gz
   
   # valgrind （功能完整）
   valgrind --leak-check=full ./target/debug/gita
   ```

#### 方案 B: C++ 侧泄漏（memory_cpp_heap_bytes 持续增长）
1. 检查 C++ new/delete 是否配对
   ```cpp
   // 问题代码：new 了没有 delete
   void* ptr = new char[1024];
   // 丢失了 ptr
   
   // 修复：使用 unique_ptr 或 shared_ptr
   std::unique_ptr<char[]> ptr(new char[1024]);
   // 自动 delete
   ```

2. 检查第三方库是否不释放
   ```bash
   # 使用 Valgrind 检测 C++ 内存泄漏
   valgrind --leak-check=full ./cpp_plugin
   
   # 或使用 AddressSanitizer
   export ASAN_OPTIONS=detect_leaks=1
   ./cpp_plugin
   ```

3. 检查 FFI 參数是否正确释放
   ```rust
   // 在 FFI 调用后，構会自动释放
   let allocator = CppAllocator::new();
   let before = allocator.get_stats().await.active_allocated_bytes;
   
   ffi_call();
   
   let after = allocator.get_stats().await.active_allocated_bytes;
   if after > before {
       eprintln!("FFI 没有释放: {} bytes", after - before);
   }
   ```

#### 方案 C: 共享内存泄漏（memory_shared_bytes 增长）
1. 检查图像缓冲区是否正常清理
   ```rust
   // 问题：转换后的图像不释放
   let image = load_image();
   convert_to_shared_format(&image);  // 创建映射
   // 没有 drop image
   
   // 修复：显式 drop
   drop(image);
   ```

2. 检查引用计数是否正常渐低
   ```rust
   let shared_ptr = Arc::clone(&image);
   // 提供给 FFI
   ffi_process(shared_ptr.clone());
   // strong_count 应该逐步渐低
   ```

### 问题 3: 响应时间突然升高

**症状**: `http_request_duration_seconds_bucket` 中高延迟的计数增加

**Rust 侧排查**:
1. 检查是否采用了效率低的算法
   ```promql
   # 查询 C++ FFI 调用的缓存
   histogram_quantile(0.95, ffi_call_duration_milliseconds_bucket)
   ```

2. 检查是否有 Tokio 任务阻塞
   ```rust
   // 打印任务阻塞的警告
   tokio::time::timeout(Duration::from_secs(5), ffi_call()).await
   ```

3. 检查是否不必要的内存拷贝
   ```rust
   // 问题: 大量不必要的拷贝
   let data = large_buffer.clone();  // 不需要
   
   // 修复: 使用引用
   let data = &large_buffer;
   ```

**C++ 侧排查**:
1. 检查是否频繁内存分配/释放。应询指标: cpp_allocator.total_allocations
   ```bash
   # 查看分配统计
   curl http://localhost:3000/api/v1/metrics | jq .cpp_allocator
   # 如果 total_allocations 很高但每次分配量比较小，表示提供帣字节的批量分配
   ```

2. 使用 perf 检测 CPU 热点
   ```bash
   perf top
   # 检查是否有 malloc/free 作为热点
   # 或 memcpy 耗时过高
   ```

3. 检查 FFI 调用是否不效
   ```promql
   # 查看 FFI 调用时间
   histogram_quantile(0.95, ffi_call_duration_milliseconds_bucket)
   # 如果 > 1000ms，需要优化 C++ 代码
   ```

---

## 七、集成示例

### 7.1 与 Slack 集成

使用 Prometheus AlertManager:

```yaml
# alertmanager.yml
global:
  slack_api_url: 'https://hooks.slack.com/services/YOUR/WEBHOOK'

route:
  receiver: 'slack-notifications'

receivers:
- name: 'slack-notifications'
  slack_configs:
  - channel: '#alerts'
    title: '{{ .GroupLabels.alertname }}'
    text: '{{ range .Alerts }}{{ .Annotations.description }}{{ end }}'
```

### 7.2 与 PagerDuty 集成

```yaml
receivers:
- name: 'pagerduty'
  pagerduty_configs:
  - service_key: 'YOUR_SERVICE_KEY'
```

---

## 八、参考资源

- [Prometheus 官方文档](https://prometheus.io/docs/)
- [Grafana 官方文档](https://grafana.com/docs/)
- [四个黄金指标](https://sre.google/books/) (Google SRE Book)
- [RED 方法论](https://www.weave.works/blog/the-red-method-key-metrics-for-microservices-architecture/)
- [USE 方法论](http://www.brendangregg.com/usemethod.html)

