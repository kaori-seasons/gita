# 存储层模块 - 交互时序图详解

## 🎯 存储层架构图

```mermaid
graph TB
    subgraph "存储接口层"
        SM[存储管理器<br/>StorageManager]
        PM[持久化管理器<br/>PersistenceManager]
        CM[缓存管理器<br/>CacheManager]
        FM[文件管理器<br/>FileManager]
    end

    subgraph "存储引擎"
        SLED[(Sled DB<br/>主数据库)]
        REDIS[(Redis<br/>缓存)]
        FS[文件系统<br/>本地存储]
        S3[S3兼容<br/>对象存储]
    end

    subgraph "数据访问层"
        DAO[数据访问对象<br/>DAO Layer]
        REPO[存储库<br/>Repository]
        MAPPER[对象映射<br/>Object Mapper]
        POOL[连接池<br/>Connection Pool]
    end

    subgraph "数据管理"
        TX[事务管理器<br/>TransactionManager]
        LOCK[锁管理器<br/>LockManager]
        BACKUP[备份管理器<br/>BackupManager]
        RECOVERY[恢复管理器<br/>RecoveryManager]
    end

    subgraph "监控集成"
        METRICS[存储指标<br/>StorageMetrics]
        HEALTH[健康检查<br/>HealthChecker]
        ALERTS[存储告警<br/>StorageAlerts]
    end

    SM --> PM
    SM --> CM
    SM --> FM

    PM --> SLED
    CM --> REDIS
    FM --> FS
    FM --> S3

    DAO --> REPO
    REPO --> MAPPER
    MAPPER --> POOL

    POOL --> SLED
    POOL --> REDIS

    TX --> SLED
    LOCK --> SLED
    BACKUP --> SLED
    RECOVERY --> SLED

    METRICS --> HEALTH
    HEALTH --> ALERTS

    classDef interface fill:#e8f5e8
    classDef engine fill:#fff3e0
    classDef access fill:#fce4ec
    classDef management fill:#f1f8e9
    classDef monitoring fill:#e0f2f1

    class SM,PM,CM,FM interface
    class SLED,REDIS,FS,S3 engine
    class DAO,REPO,MAPPER,POOL access
    class TX,LOCK,BACKUP,RECOVERY management
    class METRICS,HEALTH,ALERTS monitoring
```

## 🔄 存储操作完整时序图

```mermaid
sequenceDiagram
    participant Client
    participant API
    participant StorageManager
    participant CacheManager
    participant PersistenceManager
    participant SledDB
    participant Redis
    participant FileSystem
    participant MetricsCollector

    %% 数据写入流程
    rect rgb(240, 248, 255)
        Client->>API: POST /api/data
        API->>StorageManager: store_data(key, value)

        StorageManager->>CacheManager: write_to_cache(key, value)
        CacheManager->>Redis: SET key value
        Redis-->>CacheManager: 写入成功

        StorageManager->>PersistenceManager: persist_data(key, value)
        PersistenceManager->>SledDB: INSERT key value
        SledDB-->>PersistenceManager: 持久化成功

        alt 需要文件存储
            StorageManager->>FileSystem: write_file(path, content)
            FileSystem-->>StorageManager: 文件写入成功
        end

        StorageManager->>MetricsCollector: record_write_operation()
        MetricsCollector-->>StorageManager: 指标记录完成

        StorageManager-->>API: 存储成功
        API-->>Client: HTTP 201 Created
    end

    %% 数据读取流程
    rect rgb(255, 250, 240)
        Client->>API: GET /api/data/{key}
        API->>StorageManager: get_data(key)

        StorageManager->>CacheManager: read_from_cache(key)
        CacheManager->>Redis: GET key

        alt 缓存命中
            Redis-->>CacheManager: 返回数据
            CacheManager-->>StorageManager: 返回缓存数据
            StorageManager->>MetricsCollector: record_cache_hit()
        else 缓存未命中
            Redis-->>CacheManager: 缓存未命中
            CacheManager-->>StorageManager: 缓存未命中

            StorageManager->>PersistenceManager: read_from_db(key)
            PersistenceManager->>SledDB: SELECT key
            SledDB-->>PersistenceManager: 返回数据
            PersistenceManager-->>StorageManager: 返回数据库数据

            StorageManager->>CacheManager: write_to_cache(key, value)
            CacheManager->>Redis: SET key value

            StorageManager->>MetricsCollector: record_cache_miss()
        end

        MetricsCollector-->>StorageManager: 指标记录完成
        StorageManager-->>API: 返回数据
        API-->>Client: HTTP 200 OK
    end
```

## 📋 详细存储操作时序分析

### 1. 数据写入时序图

```mermaid
sequenceDiagram
    participant Application
    participant StorageManager
    participant TransactionManager
    participant CacheManager
    participant PersistenceManager
    participant LockManager
    participant WAL
    participant SledDB
    participant MetricsCollector

    Application->>StorageManager: store_data(key, value, options)
    StorageManager->>TransactionManager: begin_transaction()

    alt 需要事务支持
        TransactionManager->>LockManager: acquire_locks(keys)
        LockManager-->>TransactionManager: 锁获取成功
    end

    TransactionManager-->>StorageManager: 事务开始

    StorageManager->>CacheManager: write_cache(key, value)
    CacheManager-->>StorageManager: 缓存写入完成

    StorageManager->>PersistenceManager: write_persistent(key, value)
    PersistenceManager->>WAL: write_ahead_log(operation)
    WAL-->>PersistenceManager: WAL写入成功

    PersistenceManager->>SledDB: insert_record(key, value)
    SledDB-->>PersistenceManager: 记录插入成功
    PersistenceManager-->>StorageManager: 持久化完成

    alt 事务提交
        StorageManager->>TransactionManager: commit_transaction()
        TransactionManager->>WAL: commit_wal()
        TransactionManager->>LockManager: release_locks()
        LockManager-->>TransactionManager: 锁释放完成
        TransactionManager-->>StorageManager: 事务提交成功
    else 事务回滚
        StorageManager->>TransactionManager: rollback_transaction()
        TransactionManager->>WAL: rollback_wal()
        TransactionManager->>LockManager: release_locks()
        TransactionManager-->>StorageManager: 事务回滚完成
    end

    StorageManager->>MetricsCollector: record_operation("write", success)
    MetricsCollector-->>StorageManager: 指标记录完成

    StorageManager-->>Application: 操作完成
```

### 2. 缓存策略时序图

```mermaid
sequenceDiagram
    participant Application
    participant CacheManager
    participant CacheStrategy
    participant Redis
    participant PersistenceManager
    participant MetricsCollector

    Application->>CacheManager: get(key)

    CacheManager->>CacheStrategy: should_check_cache(key)
    CacheStrategy-->>CacheManager: 需要检查缓存

    CacheManager->>Redis: GET key
    alt 缓存命中
        Redis-->>CacheManager: value
        CacheManager->>CacheStrategy: update_access_pattern(key)
        CacheStrategy-->>CacheManager: 访问模式更新完成
        CacheManager->>MetricsCollector: record_cache_hit()
        CacheManager-->>Application: 返回缓存数据
    else 缓存未命中
        Redis-->>CacheManager: null
        CacheManager->>MetricsCollector: record_cache_miss()

        CacheManager->>PersistenceManager: load_from_persistence(key)
        PersistenceManager-->>CacheManager: value

        CacheManager->>CacheStrategy: should_cache_value(value)
        CacheStrategy-->>CacheManager: 需要缓存

        CacheManager->>Redis: SET key value EX ttl
        Redis-->>CacheManager: 设置成功

        CacheManager->>CacheStrategy: record_cache_write(key)
        CacheStrategy-->>CacheManager: 缓存写入记录完成

        CacheManager-->>Application: 返回数据
    end
```

### 3. 备份和恢复时序图

```mermaid
sequenceDiagram
    participant Scheduler
    participant BackupManager
    participant StorageManager
    participant SnapshotCreator
    participant FileSystem
    participant RemoteStorage
    participant RecoveryManager
    participant AlertManager

    %% 定期备份
    Scheduler->>BackupManager: trigger_backup(schedule)
    BackupManager->>StorageManager: acquire_backup_lock()
    StorageManager-->>BackupManager: 锁获取成功

    BackupManager->>SnapshotCreator: create_snapshot()
    SnapshotCreator->>StorageManager: flush_all_data()
    StorageManager-->>SnapshotCreator: 数据刷新完成

    SnapshotCreator->>FileSystem: write_snapshot_file(path)
    FileSystem-->>SnapshotCreator: 快照文件写入完成
    SnapshotCreator-->>BackupManager: 快照创建完成

    BackupManager->>RemoteStorage: upload_backup_file(file)
    RemoteStorage-->>BackupManager: 备份文件上传完成

    BackupManager->>StorageManager: release_backup_lock()
    StorageManager-->>BackupManager: 锁释放完成

    BackupManager->>AlertManager: send_backup_success_alert()
    AlertManager-->>BackupManager: 告警发送完成

    %% 数据恢复
    Scheduler->>RecoveryManager: trigger_recovery(backup_id)
    RecoveryManager->>RemoteStorage: download_backup_file(backup_id)
    RemoteStorage-->>RecoveryManager: 备份文件下载完成

    RecoveryManager->>StorageManager: prepare_recovery()
    StorageManager-->>RecoveryManager: 恢复准备完成

    RecoveryManager->>FileSystem: extract_backup_data(file)
    FileSystem-->>RecoveryManager: 数据提取完成

    RecoveryManager->>StorageManager: restore_data(data)
    StorageManager-->>RecoveryManager: 数据恢复完成

    RecoveryManager->>StorageManager: verify_restored_data()
    StorageManager-->>RecoveryManager: 数据验证完成

    RecoveryManager->>AlertManager: send_recovery_success_alert()
    AlertManager-->>RecoveryManager: 告警发送完成
```

### 4. 数据一致性保证时序图

```mermaid
sequenceDiagram
    participant Application
    participant ConsistencyManager
    participant CacheManager
    participant PersistenceManager
    participant ReplicationManager
    participant ConflictResolver

    Application->>ConsistencyManager: write_with_consistency(key, value)

    ConsistencyManager->>CacheManager: invalidate_cache(key)
    CacheManager-->>ConsistencyManager: 缓存失效完成

    ConsistencyManager->>PersistenceManager: write_primary(key, value)
    PersistenceManager-->>ConsistencyManager: 主存储写入成功

    alt 强一致性模式
        ConsistencyManager->>ReplicationManager: replicate_to_secondaries(key, value)
        ReplicationManager-->>ConsistencyManager: 复制完成
        ConsistencyManager-->>Application: 写入成功
    else 最终一致性模式
        ConsistencyManager->>ReplicationManager: async_replicate(key, value)
        ReplicationManager-->>ConsistencyManager: 异步复制启动
        ConsistencyManager-->>Application: 写入成功（异步复制）
    end

    %% 处理复制冲突
    ReplicationManager->>ConflictResolver: check_conflicts(key, versions)
    alt 检测到冲突
        ConflictResolver->>ConflictResolver: resolve_conflict(strategy)
        ConflictResolver-->>ReplicationManager: 冲突解决完成
        ReplicationManager->>ConsistencyManager: conflict_resolved(key)
    else 无冲突
        ReplicationManager->>ConsistencyManager: replication_complete(key)
    end

    ConsistencyManager-->>Application: 数据一致性保证完成
```

## 📊 存储层性能指标

### 存储操作指标
```mermaid
graph LR
    subgraph "读取性能"
        R1[缓存命中率<br/>> 90%]
        R2[读取延迟<br/>< 10ms]
        R3[读取吞吐量<br/>> 1000 RPS]
        R4[读取错误率<br/>< 0.1%]
    end

    subgraph "写入性能"
        W1[写入延迟<br/>< 50ms]
        W2[写入吞吐量<br/>> 500 RPS]
        W3[写入错误率<br/>< 0.1%]
        W4[事务提交率<br/>> 99.9%]
    end

    subgraph "存储容量"
        S1[存储使用率<br/>< 80%]
        S2[数据压缩率<br/>> 50%]
        S3[索引效率<br/>> 90%]
        S4[垃圾回收效率<br/>> 95%]
    end

    subgraph "备份恢复"
        B1[备份成功率<br/>> 99.9%]
        B2[备份时间<br/>< 1小时]
        B3[恢复时间<br/>< 30分钟]
        B4[数据完整性<br/>100%]
    end

    R1 --> MONITOR[监控告警]
    R2 --> MONITOR
    R3 --> MONITOR
    R4 --> MONITOR
    W1 --> MONITOR
    W2 --> MONITOR
    W3 --> MONITOR
    W4 --> MONITOR
    S1 --> MONITOR
    S2 --> MONITOR
    S3 --> MONITOR
    S4 --> MONITOR
    B1 --> MONITOR
    B2 --> MONITOR
    B3 --> MONITOR
    B4 --> MONITOR
```

### 存储健康检查
```mermaid
graph TD
    A[存储健康检查] --> B{检查项目}
    B -->|连接状态| C[数据库连接]
    B -->|性能指标| D[读写性能]
    B -->|存储容量| E[磁盘空间]
    B -->|数据完整性| F[校验和]
    B -->|备份状态| G[备份完整性]

    C --> H{检查结果}
    D --> H
    E --> H
    F --> H
    G --> H

    H -->|正常| I[健康状态]
    H -->|警告| J[警告状态]
    H -->|严重| K[告警状态]

    I --> L[继续监控]
    J --> M[发送通知]
    K --> N[触发告警]

    M --> L
    N --> O[启动修复]
    O --> L
```

## 🔧 存储配置参数

### Sled数据库配置
```toml
[database.sled]
path = "./data/db"
cache_size_mb = 512
compression = true
flush_every_ms = 1000
snapshot_interval_sec = 3600

[database.sled.performance]
max_read_threads = 4
max_write_threads = 2
batch_size = 1000
prefetch_size = 1024
```

### Redis缓存配置
```toml
[cache.redis]
url = "redis://localhost:6379"
connection_pool_size = 10
key_prefix = "rust_edge_compute:"
default_ttl_seconds = 3600

[cache.redis.cluster]
enabled = false
nodes = ["redis://node1:6379", "redis://node2:6379"]

[cache.redis.performance]
pipeline_batch_size = 50
read_timeout_ms = 5000
write_timeout_ms = 5000
```

### 文件存储配置
```toml
[storage.filesystem]
base_path = "./storage"
max_file_size_mb = 100
allowed_extensions = ["txt", "json", "bin", "log"]
compression_enabled = true

[storage.filesystem.cleanup]
max_age_days = 30
cleanup_interval_hours = 24
delete_empty_dirs = true
```

### 备份配置
```toml
[backup]
enabled = true
schedule = "0 2 * * *"  # 每天凌晨2点
retention_days = 30
compression_level = 6
encryption_enabled = true

[backup.storage]
type = "local"
path = "./backups"
remote_url = ""
remote_access_key = ""
remote_secret_key = ""

[backup.verification]
enabled = true
sample_size_percent = 10
checksum_algorithm = "SHA256"
```

## 🚨 故障处理策略

### 存储故障场景分析
```mermaid
graph TD
    A[存储故障检测] --> B{故障类型}
    B -->|数据库连接失败| C[连接池重试]
    B -->|磁盘空间不足| D[清理过期数据]
    B -->|缓存服务异常| E[降级到直读]
    B -->|文件系统错误| F[切换备用存储]
    B -->|数据损坏| G[从备份恢复]

    C --> H[故障恢复]
    D --> H
    E --> H
    F --> H
    G --> H

    H --> I{恢复成功}
    I -->|是| J[恢复正常]
    I -->|否| K[升级告警]
    K --> L[人工干预]
```

### 自动故障转移
```mermaid
sequenceDiagram
    participant Monitor
    participant StorageManager
    participant FailoverManager
    participant BackupStorage
    participant AlertManager

    Monitor->>Monitor: 检测主存储故障
    Monitor->>StorageManager: report_storage_failure(type, severity)
    StorageManager->>FailoverManager: initiate_failover()

    FailoverManager->>FailoverManager: 评估故障影响
    alt 可以自动恢复
        FailoverManager->>StorageManager: switch_to_backup()
        StorageManager-->>FailoverManager: 切换完成
        FailoverManager->>BackupStorage: promote_to_primary()
        BackupStorage-->>FailoverManager: 提升完成
    else 需要人工干预
        FailoverManager->>AlertManager: send_critical_alert()
        AlertManager-->>FailoverManager: 告警发送
    end

    FailoverManager->>StorageManager: verify_failover_success()
    StorageManager-->>FailoverManager: 验证完成

    FailoverManager-->>Monitor: 故障转移完成
```

## 📈 存储优化策略

### 性能优化
1. **索引优化**: 优化数据库索引结构
2. **查询优化**: 减少复杂查询，添加缓存层
3. **连接池优化**: 调整连接池大小和超时设置
4. **批处理优化**: 实现批量读写操作

### 容量优化
1. **数据压缩**: 启用数据压缩减少存储空间
2. **数据清理**: 定期清理过期和无用数据
3. **存储分层**: 热数据和冷数据分离存储
4. **去重机制**: 检测和删除重复数据

### 可用性优化
1. **数据复制**: 多副本数据存储
2. **故障转移**: 自动故障检测和转移
3. **备份策略**: 定期全量备份和增量备份
4. **监控告警**: 实时监控存储状态

### 扩展性优化
1. **分片策略**: 数据分片和分布式存储
2. **负载均衡**: 读写请求的负载均衡
3. **缓存策略**: 多级缓存和缓存预热
4. **异步处理**: 异步写入和后台处理

## 🎯 存储层总结

存储层是整个系统的核心组件，提供了以下关键功能：

### ✅ 核心特性
- **多级存储**: 内存缓存 + 持久化存储 + 文件存储
- **高可用性**: 数据复制 + 故障转移 + 自动恢复
- **高性能**: 缓存加速 + 索引优化 + 批处理操作
- **数据一致性**: 事务支持 + 锁机制 + 冲突解决
- **安全可靠**: 数据加密 + 访问控制 + 审计日志

### 🚀 性能规格
- **读取性能**: <10ms平均延迟，>90%缓存命中率
- **写入性能**: <50ms平均延迟，>99.9%成功率
- **存储容量**: 智能压缩，<80%存储使用率
- **备份恢复**: <1小时备份，<30分钟恢复

### 📊 可观测性
- **实时监控**: 存储指标、性能指标、健康状态
- **告警系统**: 阈值告警、智能异常检测
- **日志聚合**: 结构化日志、操作审计
- **可视化**: Grafana仪表板、性能图表

这个存储层提供了企业级的存储解决方案，支持高并发、高可用、高性能的应用场景。
