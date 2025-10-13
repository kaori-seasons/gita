# 调度器模块 - 交互时序图详解

## 🎯 调度器架构图

```mermaid
graph TB
    subgraph "调度器组件"
        SM[调度器管理器<br/>SchedulerManager]
        TQ[任务队列<br/>TaskQueue]
        WS[工作线程池<br/>WorkerPool]
        PS[优先级调度器<br/>PriorityScheduler]
        RM[重试管理器<br/>RetryManager]
        LM[负载均衡器<br/>LoadBalancer]
    end

    subgraph "任务生命周期"
        SUBMIT[任务提交]
        QUEUE[队列等待]
        SCHEDULE[任务调度]
        EXECUTE[任务执行]
        COMPLETE[任务完成]
        RETRY[重试处理]
        FAIL[任务失败]
    end

    subgraph "监控集成"
        METRICS[性能指标]
        LOGS[操作日志]
        ALERTS[告警通知]
    end

    SUBMIT --> TQ
    TQ --> PS
    PS --> LM
    LM --> WS
    WS --> EXECUTE
    EXECUTE --> COMPLETE
    EXECUTE --> RETRY
    RETRY --> TQ
    EXECUTE --> FAIL

    SM --> TQ
    SM --> PS
    SM --> RM
    SM --> LM

    TQ --> METRICS
    WS --> METRICS
    PS --> LOGS
    RM --> ALERTS
```

## 🔄 任务调度完整时序图

```mermaid
sequenceDiagram
    participant Client
    participant API
    participant Auth
    participant Scheduler
    participant Queue
    participant PriorityScheduler
    participant LoadBalancer
    participant WorkerPool
    participant Worker
    participant TaskExecutor
    participant FFI
    participant Container
    participant Storage
    participant Metrics

    %% 任务提交阶段
    rect rgb(240, 248, 255)
        Client->>API: POST /api/v1/compute
        API->>Auth: 验证用户权限
        Auth-->>API: 权限验证通过
        API->>Scheduler: submit_task(request)

        Scheduler->>Queue: add_task(task)
        Queue-->>Scheduler: 任务入队成功
        Scheduler-->>API: 返回任务ID
        API-->>Client: HTTP 202 Accepted
    end

    %% 任务调度阶段
    rect rgb(255, 250, 240)
        Scheduler->>PriorityScheduler: 选择下一个任务
        PriorityScheduler->>Queue: get_next_task()
        Queue-->>PriorityScheduler: 返回最高优先级任务
        PriorityScheduler-->>Scheduler: 返回调度任务

        Scheduler->>LoadBalancer: 选择最优工作线程
        LoadBalancer->>WorkerPool: get_available_worker()
        WorkerPool-->>LoadBalancer: 返回可用工作线程
        LoadBalancer-->>Scheduler: 返回目标工作线程

        Scheduler->>WorkerPool: assign_task(worker_id, task)
        WorkerPool->>Worker: execute_task(task)
    end

    %% 任务执行阶段
    rect rgb(240, 255, 240)
        Worker->>TaskExecutor: 执行任务逻辑
        TaskExecutor->>FFI: 调用C++算法
        FFI->>Container: 在容器中执行
        Container-->>FFI: 返回执行结果
        FFI-->>TaskExecutor: 返回算法结果
        TaskExecutor-->>Worker: 任务执行完成
    end

    %% 结果处理阶段
    rect rgb(255, 240, 245)
        Worker->>Storage: 保存执行结果
        Storage-->>Worker: 保存成功
        Worker->>Scheduler: notify_task_complete(task_id)
        Scheduler->>Queue: remove_completed_task(task_id)
        Queue-->>Scheduler: 任务清理完成
        Scheduler-->>API: 异步通知结果（可选）
    end

    %% 监控和日志
    Worker->>Metrics: record_execution_time(duration)
    Scheduler->>Metrics: record_queue_size(size)
    Worker->>Metrics: record_success_rate(success)
    Metrics-->>Metrics: 更新性能指标
```

## 📋 详细时序分析

### 1. 任务提交流程

```mermaid
sequenceDiagram
    participant Client
    participant API
    participant Validator
    participant Auth
    participant RateLimiter
    participant Scheduler
    participant TaskBuilder

    Client->>API: POST /compute {"algorithm": "add", "params": {...}}
    API->>Validator: validate_request(request)
    Validator-->>API: 验证通过

    API->>Auth: authenticate(token)
    Auth-->>API: 用户认证成功

    API->>RateLimiter: check_rate_limit(user_id)
    RateLimiter-->>API: 速率检查通过

    API->>TaskBuilder: build_task(request, user_context)
    TaskBuilder-->>API: 任务对象创建完成

    API->>Scheduler: submit_task(task)
    Scheduler-->>API: 任务ID返回
    API-->>Client: 202 Accepted {"task_id": "..."}
```

### 2. 优先级调度流程

```mermaid
sequenceDiagram
    participant Scheduler
    participant PriorityQueue
    participant TaskSelector
    participant PriorityCalculator
    participant DeadlineManager

    Scheduler->>PriorityQueue: get_next_task()
    PriorityQueue->>TaskSelector: select_candidate_tasks()

    loop 候选任务评估
        TaskSelector->>PriorityCalculator: calculate_priority(task)
        PriorityCalculator-->>TaskSelector: 优先级分数

        TaskSelector->>DeadlineManager: check_deadline(task)
        DeadlineManager-->>TaskSelector: 截止时间状态
    end

    TaskSelector-->>PriorityQueue: 返回最高优先级任务
    PriorityQueue-->>Scheduler: 调度任务
```

### 3. 工作线程分配流程

```mermaid
sequenceDiagram
    participant Scheduler
    participant LoadBalancer
    participant WorkerPool
    participant WorkerMonitor
    participant ResourceManager

    Scheduler->>LoadBalancer: select_worker(task)
    LoadBalancer->>WorkerPool: get_worker_status()

    loop 工作线程评估
        WorkerPool->>WorkerMonitor: get_worker_metrics(worker_id)
        WorkerMonitor-->>WorkerPool: CPU/内存/队列长度

        WorkerPool->>ResourceManager: check_resource_availability(worker_id)
        ResourceManager-->>WorkerPool: 资源可用性
    end

    WorkerPool-->>LoadBalancer: 返回最优工作线程
    LoadBalancer-->>Scheduler: 分配结果
```

### 4. 任务执行流程

```mermaid
sequenceDiagram
    participant Worker
    participant TaskExecutor
    participant FFI
    participant Container
    participant TimeoutManager
    participant RetryHandler
    participant MetricsCollector

    Worker->>TaskExecutor: execute(task)
    TaskExecutor->>TimeoutManager: start_timeout_monitor(timeout)
    TimeoutManager-->>TaskExecutor: 监控启动

    TaskExecutor->>FFI: call_algorithm(algorithm, params)
    FFI->>Container: execute_in_container(command)
    Container-->>FFI: 执行结果
    FFI-->>TaskExecutor: 算法结果

    TaskExecutor->>MetricsCollector: record_execution_metrics()
    MetricsCollector-->>TaskExecutor: 指标记录完成

    TaskExecutor->>TimeoutManager: cancel_timeout()
    TimeoutManager-->>TaskExecutor: 监控取消

    alt 执行成功
        TaskExecutor-->>Worker: TaskResult::Success
    else 执行失败
        TaskExecutor->>RetryHandler: should_retry(error)
        RetryHandler-->>TaskExecutor: 是否重试
        TaskExecutor-->>Worker: TaskResult::Retry 或 TaskResult::Failed
    end
```

### 5. 重试机制流程

```mermaid
sequenceDiagram
    participant Worker
    participant RetryHandler
    participant BackoffCalculator
    participant RetryQueue
    participant Scheduler
    participant AlertManager

    Worker->>RetryHandler: handle_failure(task, error)

    RetryHandler->>RetryHandler: check_retry_policy(task)
    alt 可以重试
        RetryHandler->>BackoffCalculator: calculate_delay(attempt_count)
        BackoffCalculator-->>RetryHandler: 重试延迟时间

        RetryHandler->>RetryQueue: enqueue_with_delay(task, delay)
        RetryQueue-->>RetryHandler: 入队成功

        RetryHandler-->>Worker: 重试已调度
    else 达到最大重试次数
        RetryHandler->>AlertManager: send_failure_alert(task, error)
        AlertManager-->>RetryHandler: 告警发送完成

        RetryHandler-->>Worker: 任务失败
    end
```

### 6. 负载均衡流程

```mermaid
sequenceDiagram
    participant LoadBalancer
    participant WorkerRegistry
    participant HealthChecker
    participant MetricsAggregator
    participant DecisionEngine

    LoadBalancer->>WorkerRegistry: get_active_workers()
    WorkerRegistry-->>LoadBalancer: 活跃工作线程列表

    loop 工作线程健康检查
        LoadBalancer->>HealthChecker: check_worker_health(worker_id)
        HealthChecker-->>LoadBalancer: 健康状态
    end

    LoadBalancer->>MetricsAggregator: get_worker_metrics()
    MetricsAggregator-->>LoadBalancer: 性能指标

    LoadBalancer->>DecisionEngine: select_optimal_worker(task, metrics)
    DecisionEngine-->>LoadBalancer: 最优工作线程选择

    alt 找到合适的工作线程
        LoadBalancer-->>Scheduler: 返回选中工作线程
    else 无可用工作线程
        LoadBalancer-->>Scheduler: 返回等待或拒绝
    end
```

## 📊 调度器性能指标

### 核心指标
```mermaid
graph LR
    subgraph "队列指标"
        QI1[队列长度]
        QI2[入队速率]
        QI3[出队速率]
        QI4[队列等待时间]
    end

    subgraph "调度指标"
        SI1[调度延迟]
        SI2[调度成功率]
        SI3[负载均衡效率]
        SI4[优先级准确性]
    end

    subgraph "执行指标"
        EI1[任务执行时间]
        EI2[并发执行数]
        EI3[资源利用率]
        EI4[失败重试率]
    end

    subgraph "系统指标"
        SYI1[内存使用]
        SYI2[CPU使用]
        SYI3[线程数]
        SYI4[锁竞争]
    end

    QI1 --> MONITOR[监控系统]
    QI2 --> MONITOR
    QI3 --> MONITOR
    QI4 --> MONITOR
    SI1 --> MONITOR
    SI2 --> MONITOR
    SI3 --> MONITOR
    SI4 --> MONITOR
    EI1 --> MONITOR
    EI2 --> MONITOR
    EI3 --> MONITOR
    EI4 --> MONITOR
    SYI1 --> MONITOR
    SYI2 --> MONITOR
    SYI3 --> MONITOR
    SYI4 --> MONITOR
```

### 性能阈值监控
```mermaid
graph TD
    A[性能指标收集] --> B{检查阈值}
    B -->|正常| C[继续监控]
    B -->|警告| D[发送警告]
    B -->|严重| E[触发告警]
    B -->|紧急| F[自动降级]

    D --> G[记录日志]
    E --> H[通知管理员]
    F --> I[启用降级模式]

    G --> C
    H --> C
    I --> J[降级监控]
    J --> C
```

## 🔧 配置参数

### 调度器配置
```toml
[scheduler]
max_concurrent_tasks = 10
queue_size = 1000
task_timeout_seconds = 300
default_max_retries = 3
worker_pool_size = 4
priority_levels = 4
load_balance_strategy = "round_robin"

[scheduler.retry]
max_attempts = 5
initial_delay_ms = 1000
backoff_multiplier = 2.0
max_delay_ms = 30000

[scheduler.priority]
high_threshold = 0.8
medium_threshold = 0.6
low_threshold = 0.3

[scheduler.monitoring]
metrics_interval_ms = 5000
health_check_interval_ms = 30000
alert_threshold_percent = 80
```

### 工作线程配置
```toml
[worker]
max_tasks_per_worker = 100
task_execution_timeout_ms = 300000
resource_check_interval_ms = 1000
health_report_interval_ms = 5000

[worker.resource_limits]
max_memory_mb = 512
max_cpu_percent = 80
max_concurrent_tasks = 5

[worker.metrics]
collect_execution_time = true
collect_resource_usage = true
collect_error_rates = true
export_prometheus = true
```

## 🚨 故障处理

### 常见故障场景
```mermaid
graph TD
    A[故障检测] --> B{故障类型}
    B -->|队列积压| C[增加工作线程]
    B -->|执行超时| D[调整超时设置]
    B -->|资源不足| E[扩展系统资源]
    B -->|网络故障| F[启用重试机制]
    B -->|依赖失败| G[降级处理]

    C --> H[监控效果]
    D --> H
    E --> H
    F --> H
    G --> H

    H -->|问题解决| I[恢复正常]
    H -->|问题持续| J[升级告警]
```

### 自动恢复机制
```mermaid
sequenceDiagram
    participant Monitor
    participant Scheduler
    participant WorkerPool
    participant AlertManager
    participant RecoveryManager

    Monitor->>Monitor: 检测系统异常
    Monitor->>Scheduler: report_anomaly(type, severity)
    Scheduler->>WorkerPool: check_worker_health()

    alt 工作线程异常
        WorkerPool->>RecoveryManager: restart_worker(worker_id)
        RecoveryManager-->>WorkerPool: 重启完成
    else 队列积压
        Scheduler->>WorkerPool: scale_up_workers(count)
        WorkerPool-->>Scheduler: 扩容完成
    else 资源不足
        Scheduler->>AlertManager: send_resource_alert()
        AlertManager-->>Scheduler: 告警发送
    end

    Scheduler->>Monitor: recovery_complete()
    Monitor-->>Scheduler: 监控恢复
```

## 📈 优化建议

### 性能优化
1. **并发优化**: 调整工作线程池大小
2. **队列优化**: 实现多级队列和优先级调度
3. **缓存优化**: 添加任务结果缓存
4. **资源优化**: 动态调整资源分配

### 可扩展性优化
1. **分布式调度**: 支持跨节点任务调度
2. **负载均衡**: 实现智能负载均衡算法
3. **故障转移**: 添加故障自动转移机制
4. **配置管理**: 实现动态配置更新

### 可观测性优化
1. **详细指标**: 添加更多性能指标
2. **链路追踪**: 实现分布式链路追踪
3. **日志聚合**: 集中化日志管理
4. **告警规则**: 配置智能告警规则

这个调度器模块提供了完整的任务调度功能，包括优先级调度、负载均衡、重试机制等，是整个系统的核心组件。
