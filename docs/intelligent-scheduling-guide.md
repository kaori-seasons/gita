# 智能调度使用指南

## 📋 概述

智能调度是Rust Edge Compute框架的可选高级功能，提供基于机器学习的工作线程智能分配。通过分析历史性能数据，智能调度能够预测最优的工作线程选择，提高系统整体性能。

## ⚙️ 配置说明

### 基本配置

在 `config/default.toml` 中配置智能调度：

```toml
[scheduler]
# 启用/禁用智能调度
intelligent_scheduling_enabled = true
strategy = "Adaptive"
max_concurrent_tasks = 10
queue_size = 1000
task_timeout_seconds = 300
default_max_retries = 3

[scheduler.load_balancer]
# 负载均衡器配置
strategy = "Adaptive"
intelligent_scheduling_enabled = true
health_check_interval = 30
max_connections_per_worker = 10
update_interval_ms = 1000
adaptive_threshold = 0.8

[scheduler.learning]
# 机器学习参数
learning_rate = 0.01
history_window_size = 1000
min_training_samples = 100
prediction_window_seconds = 300
model_update_interval_seconds = 3600
```

### 环境变量

```bash
# 启用智能调度
export INTELLIGENT_SCHEDULING_ENABLED=true

# 设置学习参数
export LEARNING_RATE=0.01
export HISTORY_WINDOW_SIZE=1000
```

## 🚀 API接口

### 1. 启用智能调度

```http
POST /api/v1/scheduler/intelligent/enable
```

**响应：**
```json
{
  "message": "Intelligent scheduling enabled successfully",
  "note": "Please restart the service to apply changes"
}
```

### 2. 禁用智能调度

```http
POST /api/v1/scheduler/intelligent/disable
```

**响应：**
```json
{
  "message": "Intelligent scheduling disabled successfully",
  "note": "Please restart the service to apply changes"
}
```

### 3. 获取智能调度状态

```http
GET /api/v1/scheduler/intelligent/status
```

**响应：**
```json
{
  "enabled": true,
  "strategy": "Adaptive",
  "has_sufficient_data": true
}
```

### 4. 获取智能调度统计

```http
GET /api/v1/scheduler/intelligent/stats
```

**响应：**
```json
{
  "total_decisions": 1250,
  "successful_decisions": 1187,
  "success_rate": 0.9496,
  "avg_response_time": 45.2,
  "model_training_samples": 1250,
  "model_last_updated": "2024-01-15T10:30:00Z",
  "learning_config": {
    "learning_rate": 0.01,
    "history_window_size": 1000,
    "min_training_samples": 100,
    "prediction_window_seconds": 300,
    "model_update_interval_seconds": 3600
  }
}
```

## 💻 编程接口

### Rust代码示例

```rust
use rust_edge_compute::core::{TaskScheduler, SchedulerConfig, LoadBalancingStrategy};

// 创建启用智能调度的配置
let config = SchedulerConfig {
    intelligent_scheduling_enabled: true,
    load_balancer_config: LoadBalancerConfig {
        strategy: LoadBalancingStrategy::Adaptive,
        intelligent_scheduling_enabled: true,
        ..Default::default()
    },
    ..Default::default()
};

// 创建调度器
let scheduler = TaskScheduler::new(config);

// 启动调度器
scheduler.start().await?;

// 获取智能调度状态
let status = scheduler.get_intelligent_scheduling_status();
println!("智能调度已启用: {}", status.enabled);

// 动态启用智能调度（需要重新启动服务生效）
scheduler.enable_intelligent_scheduling().await?;

// 动态禁用智能调度
scheduler.disable_intelligent_scheduling().await?;
```

## 📊 调度策略详解

### 传统调度策略

1. **轮询调度 (RoundRobin)**
   - 依次分配任务到每个工作线程
   - 适用于CPU密集型任务
   - 优点：简单公平
   - 缺点：不考虑工作线程性能差异

2. **最少连接调度 (LeastConnections)**
   - 选择当前连接数最少的工作线程
   - 适用于I/O密集型任务
   - 优点：负载均衡
   - 缺点：忽略响应时间差异

3. **权重调度 (Weighted)**
   - 基于权重比例分配任务
   - 适用于异构环境
   - 优点：可控制资源分配
   - 缺点：需要手动配置权重

4. **随机调度 (Random)**
   - 随机选择工作线程
   - 适用于测试环境
   - 优点：简单快速
   - 缺点：可能导致负载不均

### 智能调度策略

5. **自适应调度 (Adaptive)**
   - 基于实时性能指标自动调整
   - 根据系统负载动态选择最优策略
   - 优点：自动优化，无需手动干预

6. **负载感知调度 (LoadAware)**
   - 基于负载预测选择工作线程
   - 考虑历史负载趋势
   - 优点：预测性调度，提前避免过载

7. **响应时间感知调度 (ResponseTimeAware)**
   - 选择历史响应时间最短的工作线程
   - 适用于对延迟敏感的应用
   - 优点：最小化响应时间

8. **资源感知调度 (ResourceAware)**
   - 基于CPU和内存使用率选择工作线程
   - 适用于资源受限环境
   - 优点：最大化资源利用率

## 🔧 最佳实践

### 1. 何时启用智能调度

**推荐启用场景：**
- 高并发环境（>100并发请求）
- 任务执行时间差异较大
- 需要优化响应时间
- 系统负载经常变化
- 有足够的训练数据（>100个任务）

**不推荐启用场景：**
- 低并发环境（<10并发请求）
- 任务执行时间非常一致
- 系统资源充足
- 对延迟不敏感的应用

### 2. 配置调优

```toml
# 高性能配置
[scheduler.learning]
learning_rate = 0.05          # 较高的学习率
history_window_size = 2000     # 更大的历史窗口
min_training_samples = 200     # 更多的训练样本

# 低延迟配置
[scheduler.learning]
prediction_window_seconds = 60  # 更短的预测窗口
model_update_interval_seconds = 1800  # 更频繁的模型更新

# 资源受限配置
[scheduler.learning]
learning_rate = 0.005          # 较低的学习率
history_window_size = 500       # 较小的历史窗口
```

### 3. 监控和维护

```bash
# 查看智能调度状态
curl http://localhost:3000/api/v1/scheduler/intelligent/status

# 查看性能统计
curl http://localhost:3000/api/v1/scheduler/intelligent/stats

# 查看负载均衡器状态
curl http://localhost:3000/api/v1/scheduler/status
```

## 📈 性能优化

### 学习参数调优

1. **学习率 (learning_rate)**
   - 太大：收敛快但可能震荡
   - 太小：收敛慢但更稳定
   - 建议：0.001 ~ 0.1

2. **历史窗口大小 (history_window_size)**
   - 太大：占用更多内存，训练慢
   - 太小：学习效果差
   - 建议：500 ~ 5000

3. **最小训练样本 (min_training_samples)**
   - 太少：模型不准确
   - 太多：启用智能调度延迟
   - 建议：50 ~ 200

### 系统资源优化

```toml
# 内存优化
[scheduler.learning]
history_window_size = 1000     # 控制内存使用

# CPU优化
[scheduler.learning]
model_update_interval_seconds = 7200  # 减少模型更新频率

# 存储优化
# 定期清理历史数据
```

## 🔍 故障排除

### 常见问题

1. **智能调度效果不明显**
   ```bash
   # 检查训练数据量
   curl http://localhost:3000/api/v1/scheduler/intelligent/stats

   # 增加历史窗口
   history_window_size = 2000
   ```

2. **内存使用过高**
   ```bash
   # 减少历史窗口大小
   history_window_size = 500

   # 增加模型更新间隔
   model_update_interval_seconds = 7200
   ```

3. **响应时间增加**
   ```bash
   # 降低学习率
   learning_rate = 0.005

   # 或者暂时禁用智能调度
   intelligent_scheduling_enabled = false
   ```

### 日志分析

```bash
# 查看智能调度相关日志
tail -f logs/application.log | grep "intelligent\|scheduler"

# 查看性能监控日志
tail -f logs/application.log | grep "LoadBalancer\|Worker"
```

## 🎯 使用建议

### 渐进式启用

1. **第一阶段**：观察期
   ```toml
   intelligent_scheduling_enabled = false
   # 先运行一段时间收集基准数据
   ```

2. **第二阶段**：测试期
   ```toml
   intelligent_scheduling_enabled = true
   learning_rate = 0.001  # 保守的学习率
   history_window_size = 500  # 较小的窗口
   ```

3. **第三阶段**：优化期
   ```toml
   learning_rate = 0.01
   history_window_size = 1000
   # 根据实际表现调整参数
   ```

### 监控指标

- **调度成功率**：>95%
- **响应时间变化**：<10%增加
- **CPU使用率**：<5%增加
- **内存使用率**：<10%增加

### 回滚策略

如果智能调度效果不佳，可以快速回滚：

```toml
# 临时禁用智能调度
intelligent_scheduling_enabled = false

# 或切换到传统调度策略
strategy = "RoundRobin"
```

## 📚 深入阅读

- [负载均衡器架构详解](load-balancer-sequence.md)
- [调度器时序图详解](scheduler-sequence.md)
- [机器学习算法说明](intelligent-scheduler.md)
- [性能监控指南](performance-monitoring.md)

---

## 🎊 总结

智能调度是Rust Edge Compute框架的高级功能，为用户提供了：

✅ **可选启用**：默认禁用，用户可按需开启
✅ **自适应学习**：基于历史数据自动优化
✅ **多策略支持**：8种不同的调度策略
✅ **实时监控**：完整的性能指标和统计
✅ **动态调整**：根据系统状态自动调整策略
✅ **易于配置**：丰富的配置选项和API接口

通过合理配置和使用，智能调度能够显著提升系统的性能和资源利用率！🚀
