# ZeroMQ 消费隔离完整指南

## 📌 核心问题

**ZeroMQ 没有消费组概念**，这意味着：

1. **PUB/SUB 模式**：所有订阅者都接收相同消息（广播）
   - 无法实现消费隔离
   - 无法追踪消费进度
   - 新加入的订阅者会丢失历史消息

2. **缺少消费者管理**：
   - 没有偏移量管理
   - 没有消费组概念
   - 没有自动故障恢复

## 🏗️ 三层解决方案

### 方案1️⃣：PUSH/PULL 模式（自动负载均衡）

**适用场景**：任务分发、工作队列

#### 特点
- ✅ **自动消费隔离**：每条消息只送给一个消费者
- ✅ **自动负载均衡**：轮转分配消息
- ✅ **简单可靠**：无需额外管理
- ❌ **无消费者分组**：不支持逻辑分组
- ❌ **无偏移量管理**：无法重放

#### 消息流
```
发布者 (PUSH)
   ↓
[消息队列]
   ├→ 消费者1
   ├→ 消费者2
   └→ 消费者3

特点：消息被平均分配，每条消息只有一个消费者收到
```

#### 运行方式

**启动发布者**：
```bash
cargo run --features cpp --example zeromq_push_pull_pattern -- \
  --role publisher \
  --host 127.0.0.1 \
  --port 5555 \
  --count 30 \
  --interval 1000
```

**启动多个消费者**（在不同终端）：
```bash
# 消费者1
cargo run --features cpp --example zeromq_push_pull_pattern -- \
  --role subscriber \
  --consumer-id consumer-1 \
  --port 5555

# 消费者2
cargo run --features cpp --example zeromq_push_pull_pattern -- \
  --role subscriber \
  --consumer-id consumer-2 \
  --port 5555

# 消费者3
cargo run --features cpp --example zeromq_push_pull_pattern -- \
  --role subscriber \
  --consumer-id consumer-3 \
  --port 5555
```

**预期结果**：
```
发布者：发送30条消息，轮转分配给3个消费者
消费者1：接收消息 1, 4, 7, 10, ...（每个消费10条）
消费者2：接收消息 2, 5, 8, 11, ...（每个消费10条）
消费者3：接收消息 3, 6, 9, 12, ...（每个消费10条）
```

### 方案2️⃣：消费组管理（应用层实现）

**适用场景**：事件处理、日志聚合、消息广播

#### 特点
- ✅ **消费组隔离**：不同组接收相同消息
- ✅ **偏移量追踪**：记录消费进度
- ✅ **故障恢复**：支持消费重放
- ✅ **灵活分组**：支持业务逻辑分组
- ⚠️ **需要管理**：应用层维护状态

#### 消息流
```
发布者 (PUB) - 广播模式
   ↓
[所有消息]
   ├→ 消费组1 [成员A, 成员B] → 消费者都收到
   ├→ 消费组2 [成员C] → 消费者都收到
   └→ 消费组3 [成员D, 成员E] → 消费者都收到

特点：每个组内的所有成员都接收相同消息，不同组可独立追踪进度
```

#### 运行方式

**启动发布者**：
```bash
cargo run --features cpp --example zeromq_consumer_group -- \
  --role publisher \
  --host 127.0.0.1 \
  --port 5556 \
  --count 50 \
  --interval 1000
```

**启动不同消费组的订阅者**：
```bash
# 消费组1 - 成员1
cargo run --features cpp --example zeromq_consumer_group -- \
  --role subscriber \
  --group group-1 \
  --consumer-id member-1 \
  --port 5556

# 消费组1 - 成员2
cargo run --features cpp --example zeromq_consumer_group -- \
  --role subscriber \
  --group group-1 \
  --consumer-id member-2 \
  --port 5556

# 消费组2 - 成员1（独立接收相同消息）
cargo run --features cpp --example zeromq_consumer_group -- \
  --role subscriber \
  --group group-2 \
  --consumer-id member-1 \
  --port 5556
```

**预期结果**：
```
group-1（成员1、成员2）：
  ✓ 成员1：接收所有消息，偏移量独立追踪
  ✓ 成员2：接收所有消息，偏移量独立追踪
  ✓ 两个成员可以协作处理消息（分片）

group-2（成员1）：
  ✓ 接收所有消息（与group-1相同的消息）
  ✓ 但偏移量独立管理（消费进度不同）
```

## 📊 方案对比

| 维度 | PUSH/PULL | 消费组管理 | Kafka | Redis Streams |
|------|-----------|----------|-------|----------------|
| **消息隔离** | ✅ 自动 | ✅ 手动 | ✅ 自动 | ✅ 自动 |
| **负载均衡** | ✅ 内置 | ❌ 无 | ✅ 内置 | ✅ 内置 |
| **偏移量管理** | ❌ 无 | ✅ 应用层 | ✅ 内置 | ✅ 内置 |
| **消费者分组** | ❌ 无 | ✅ 有 | ✅ 有 | ✅ 有 |
| **消息持久化** | ❌ 无 | ❌ 无 | ✅ 有 | ✅ 有 |
| **复杂度** | ⭐ 简单 | ⭐⭐⭐ 中等 | ⭐⭐⭐⭐ 复杂 | ⭐⭐ 简单 |
| **性能** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐⭐ |
| **学习曲线** | ✅ 平缓 | ⚠️ 中等 | ❌ 陡峭 | ✅ 平缓 |

## 🎯 选择指南

### 选择 PUSH/PULL 如果你：
- ✅ 需要任务分发/工作队列
- ✅ 不需要追踪消费进度
- ✅ 消费者是临时性的
- ✅ 优先考虑性能和简洁性

```bash
场景示例：
- 任务分发队列
- 爬虫任务分配
- 日志处理队列
- 邮件发送队列
```

### 选择消费组管理如果你：
- ✅ 需要同一消息被多个逻辑分组处理
- ✅ 需要追踪消费进度
- ✅ 需要支持消费重放
- ✅ 消费者是长期运行的应用

```bash
场景示例：
- 事件流处理（多个系统订阅同一事件）
- 日志聚合（不同团队需要不同日志）
- 实时监控（多个监控系统同时订阅）
- 数据分发（多个数据中心同时消费）
```

## 💾 消费组状态管理

### 核心概念

```rust
struct ConsumerGroupState {
    group_id: String,                      // 消费组ID
    members: Vec<String>,                  // 组成员列表
    offsets: HashMap<String, u64>,         // 当前偏移量
    last_committed: HashMap<String, u64>,  // 最后提交的偏移量
}
```

### 偏移量追踪流程

```
消费者接收消息
    ↓
更新当前偏移量 (offsets)
    ↓
处理消息业务逻辑
    ↓
[每N条消息后]
    ↓
提交偏移量 (commit)
    ↓
更新已提交偏移量 (last_committed)
    ↓
[故障恢复时从 last_committed 开始]
```

## 🔄 消费重放

### 实现方式

在消费组管理方案中，可以通过重置偏移量来实现重放：

```rust
// 重放最后10条消息
group.offsets.insert("consumer-1".to_string(), 
    last_committed.get("consumer-1").unwrap_or(&0) - 10
);
```

### 常见重放场景

| 场景 | 偏移量设置 | 用途 |
|------|----------|------|
| 重放最后N条 | `current - N` | 故障恢复 |
| 重放从头开始 | `0` | 重新初始化 |
| 重放特定时间 | 根据时间戳查询 | 回溯分析 |
| 跳过N条消息 | `current + N` | 跳过有问题的消息 |

## 🚀 性能优化建议

### 1. 批量处理

```rust
// 不要每条消息都提交
if received_count % 100 == 0 {
    group.commit_offset(&consumer_id);
}
```

### 2. 消费者分片

对于消费组，可以让不同成员处理不同消息分片：

```rust
// 消费者1处理ID为偶数的消息
// 消费者2处理ID为奇数的消息
if message.id % 2 == 0 && consumer_id == "member-1" {
    process(message);
}
```

### 3. 背压控制

```rust
// 限制内存中的待处理消息
const MAX_QUEUE_SIZE: usize = 1000;

if pending_messages.len() >= MAX_QUEUE_SIZE {
    // 等待处理完成
    tokio::time::sleep(Duration::from_millis(100)).await;
}
```

## 📝 实战场景

### 场景1：任务分发系统（PUSH/PULL）

```bash
# 发布者：生成处理任务
cargo run --example zeromq_push_pull_pattern -- \
  --role publisher --port 5555 --count 1000

# 消费者1-3：并行处理任务
cargo run --example zeromq_push_pull_pattern -- \
  --role subscriber --consumer-id worker-1 --port 5555

# 结果：1000个任务被均衡分配给3个工作者
```

### 场景2：事件广播系统（消费组管理）

```bash
# 发布者：发布系统事件
cargo run --example zeromq_consumer_group -- \
  --role publisher --port 5556 --count 100

# 监控系统：订阅所有事件
cargo run --example zeromq_consumer_group -- \
  --role subscriber --group monitoring --consumer-id monitor-1 --port 5556

# 日志系统：订阅所有事件
cargo run --example zeromq_consumer_group -- \
  --role subscriber --group logging --consumer-id logger-1 --port 5556

# 分析系统：订阅所有事件
cargo run --example zeromq_consumer_group -- \
  --role subscriber --group analytics --consumer-id analyzer-1 --port 5556

# 结果：同一事件被3个不同系统独立接收和处理
```

## 🔧 进阶用法

### 自定义提交策略

```rust
// 自动提交：每收到消息自动提交
fn auto_commit(group: &mut ConsumerGroupState, consumer_id: &str) {
    group.commit_offset(consumer_id);
}

// 手动提交：只有处理成功才提交
fn manual_commit(group: &mut ConsumerGroupState, consumer_id: &str, success: bool) {
    if success {
        group.commit_offset(consumer_id);
    }
}

// 周期提交：定期提交偏移量
tokio::time::interval(Duration::from_secs(5));
```

### 消费者协调

```rust
// 检测消费者数量变化（新成员加入/离开）
if group.members.len() != previous_count {
    println!("消费组成员变化: {} -> {}", previous_count, group.members.len());
    // 触发重新平衡逻辑
}
```

## ⚠️ 常见问题

### Q1: 使用 PUSH/PULL 时消息丢失怎么办？
A: PUSH/PULL 模式下消息由 ZeroMQ 缓存。如果消费者离线：
- 消息会丢失（除非使用消息持久化）
- 解决方案：使用消费组管理方案 + 外部持久化（如 Redis/数据库）

### Q2: 消费组管理中如何处理消费者故障？
A: 实现心跳检测：
```rust
// 定期更新心跳时间
group.update_heartbeat(&consumer_id);

// 检测超时的消费者
for (member, last_heartbeat) in group.heartbeats {
    if now - last_heartbeat > TIMEOUT {
        group.remove_member(&member);
    }
}
```

### Q3: 如何保证消息不重复消费？
A: 依赖正确的偏移量管理：
```rust
// 只处理新消息
let start_offset = group.get_next_offset(consumer_id);
if message.id > start_offset {
    process(message);
    group.update_offset(consumer_id, message.id);
}
```

## 📚 总结

| 需求 | 推荐方案 | 实现文件 |
|------|--------|--------|
| 简单任务分发 | PUSH/PULL | zeromq_push_pull_pattern.rs |
| 事件广播+追踪 | 消费组管理 | zeromq_consumer_group.rs |
| 简单发送 | 原始写入器 | zeromq_writer.rs |

选择合适的方案可以显著提高系统的可靠性和可维护性！
