# 实时流式计算部署指南

## 概述

本文档提供在边缘计算环境中部署实时流式计算系统的完整指南。该系统专为4核8G内存、HDD硬盘的工控机环境优化，能够实现低延迟、高吞吐量的实时数据处理。

## 系统架构

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Kafka Broker  │    │  Edge Node      │    │  Monitoring     │
│                 │    │                 │    │  Stack          │
│  ┌────────────┐ │    │  ┌────────────┐ │    │                 │
│  │ Vibration │ │◄──►│  │ Stream     │ │    │  ┌────────────┐ │
│  │ Sensors   │ │    │  │ Processor  │ │    │  │ Prometheus │ │
│  └────────────┘ │    │  └────────────┘ │    │  └────────────┘ │
└─────────────────┘    │  ┌────────────┐ │    │  ┌────────────┐ │
                       │  │ Plugin     │ │    │  │ Grafana    │ │
                       │  │ Chain      │ │    │  └────────────┘ │
                       └─────────────────┘    └─────────────────┘
                                │
                                ▼
                       ┌─────────────────┐
                       │   Alerting      │
                       │   System        │
                       └─────────────────┘
```

## 硬件要求

### 最小配置
- **CPU**: 4核 x86_64架构
- **内存**: 8GB DDR4
- **存储**: 500GB HDD (5400RPM)
- **网络**: 1Gbps以太网

### 推荐配置
- **CPU**: 8核 x86_64架构 (支持AVX2)
- **内存**: 16GB DDR4
- **存储**: 1TB SSD + 2TB HDD
- **网络**: 1Gbps以太网 + WiFi备份

## 软件依赖

### 系统要求
- **操作系统**: Ubuntu 20.04 LTS / CentOS 7+
- **内核版本**: 4.15+
- **文件系统**: ext4 (支持大文件)

### 运行时依赖
```bash
# 系统包
sudo apt-get update
sudo apt-get install -y \
    build-essential \
    cmake \
    pkg-config \
    libssl-dev \
    librdkafka-dev \
    libsasl2-dev \
    libzstd-dev \
    liblz4-dev \
    numactl \
    iperf3

# Rust工具链
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
rustup default stable
rustup target add x86_64-unknown-linux-gnu
```

## 部署步骤

### 1. 环境准备

#### 创建专用用户
```bash
sudo groupadd edge-compute
sudo useradd -r -g edge-compute -s /bin/false edge-compute
sudo mkdir -p /opt/edge-compute
sudo chown edge-compute:edge-compute /opt/edge-compute
```

#### 配置系统参数
```bash
# /etc/sysctl.conf
vm.swappiness = 10
vm.dirty_ratio = 20
vm.dirty_background_ratio = 5
net.core.somaxconn = 65535
net.ipv4.tcp_tw_reuse = 1

# /etc/security/limits.conf
edge-compute soft nofile 65536
edge-compute hard nofile 65536
edge-compute soft nproc 16384
edge-compute hard nproc 16384

# 应用配置
sudo sysctl -p
```

#### 配置CPU亲和性
```bash
# 为边缘计算进程预留CPU核心
echo "1-3" > /sys/fs/cgroup/cpuset/edge-compute/cpuset.cpus
echo "0" > /sys/fs/cgroup/cpuset/edge-compute/cpuset.mems
```

### 2. 构建和安装

#### 下载源码
```bash
cd /opt
git clone https://github.com/your-org/rust-edge-compute.git
cd rust-edge-compute
```

#### 配置构建
```bash
# 针对边缘环境优化构建
export RUSTFLAGS="-C target-cpu=x86-64-v3 -C opt-level=3 -C codegen-units=1"
export CARGO_PROFILE_RELEASE_LTO=true
export CARGO_PROFILE_RELEASE_PANIC=abort
```

#### 构建实时流式计算组件
```bash
# 构建所有组件
cargo build --release --features "kafka,streaming"

# 构建实时流式计算示例
cargo build --release --example realtime_streaming_example
```

#### 安装二进制文件
```bash
sudo cp target/release/examples/realtime_streaming_example /usr/local/bin/
sudo cp target/release/librust_edge_compute.so /usr/local/lib/
sudo ldconfig
```

### 3. 配置系统

#### 创建配置文件
```bash
sudo mkdir -p /etc/edge-compute
sudo cp examples/realtime_streaming_config.json /etc/edge-compute/config.json
sudo chown edge-compute:edge-compute /etc/edge-compute/config.json
sudo chmod 600 /etc/edge-compute/config.json
```

#### 编辑配置文件
```json
{
  "streaming": {
    "kafka": {
      "bootstrap_servers": ["kafka-1:9092", "kafka-2:9093"],
      "group_id": "edge-compute-streaming-001",
      "topics": ["vibration-data", "equipment-status"]
    }
  },
  "deployment": {
    "node_id": "edge-node-001",
    "data_center": "factory-floor-a"
  }
}
```

#### 创建数据目录
```bash
sudo mkdir -p /var/lib/edge-compute/{data,cache,logs}
sudo chown -R edge-compute:edge-compute /var/lib/edge-compute
sudo chmod -R 755 /var/lib/edge-compute
```

### 4. 配置服务

#### 创建Systemd服务
```bash
sudo tee /etc/systemd/system/edge-compute-streaming.service > /dev/null <<EOF
[Unit]
Description=Edge Compute Streaming Service
After=network.target kafka.service
Requires=kafka.service

[Service]
Type=simple
User=edge-compute
Group=edge-compute
Environment=RUST_LOG=info
Environment=RUST_BACKTRACE=1
ExecStart=/usr/local/bin/realtime_streaming_example --config /etc/edge-compute/config.json
ExecReload=/bin/kill -HUP \$MAINPID
Restart=always
RestartSec=5
LimitNOFILE=65536
LimitNPROC=16384

# CPU亲和性
CPUAffinity=1 2 3
MemoryLimit=6G
CPUQuota=300%

# 安全设置
NoNewPrivileges=true
ProtectHome=true
ProtectSystem=strict
ReadWritePaths=/var/lib/edge-compute /tmp
PrivateTmp=true

[Install]
WantedBy=multi-user.target
EOF
```

#### 配置日志轮转
```bash
sudo tee /etc/logrotate.d/edge-compute > /dev/null <<EOF
/var/lib/edge-compute/logs/*.log {
    daily
    missingok
    rotate 7
    compress
    delaycompress
    notifempty
    create 644 edge-compute edge-compute
    postrotate
        systemctl reload edge-compute-streaming
    endscript
}
EOF
```

### 5. 启动服务

#### 启动服务
```bash
sudo systemctl daemon-reload
sudo systemctl enable edge-compute-streaming
sudo systemctl start edge-compute-streaming
```

#### 验证服务状态
```bash
# 检查服务状态
sudo systemctl status edge-compute-streaming

# 查看日志
sudo journalctl -u edge-compute-streaming -f

# 检查端口监听
sudo netstat -tlnp | grep :8080
```

## 监控配置

### Prometheus配置
```yaml
# /etc/prometheus/prometheus.yml
global:
  scrape_interval: 15s

scrape_configs:
  - job_name: 'edge-compute-streaming'
    static_configs:
      - targets: ['localhost:8080']
    metrics_path: '/metrics'
    scrape_interval: 5s
```

### Grafana仪表板

#### 导入仪表板
1. 登录Grafana (http://localhost:3000)
2. 导入仪表板ID: `edge-compute-streaming`
3. 选择Prometheus数据源

#### 关键指标监控
- **吞吐量**: `rate(edge_compute_messages_processed_total[5m])`
- **延迟**: `histogram_quantile(0.95, rate(edge_compute_processing_duration_bucket[5m]))`
- **错误率**: `rate(edge_compute_errors_total[5m]) / rate(edge_compute_messages_processed_total[5m])`
- **资源使用**: CPU、内存、磁盘I/O

## 性能优化

### 内存优化
```bash
# 启用大页内存
echo 1024 > /sys/kernel/mm/hugepages/hugepages-2048kB/nr_hugepages

# 配置内存映射
echo 268435456 > /proc/sys/vm/max_map_count

# 设置透明大页
echo always > /sys/kernel/mm/transparent_hugepage/enabled
```

### 磁盘优化
```bash
# 禁用atime
sudo mount -o remount,noatime /data

# 配置I/O调度器
echo deadline > /sys/block/sda/queue/scheduler

# 增加I/O队列深度
echo 256 > /sys/block/sda/queue/nr_requests
```

### 网络优化
```bash
# 增加网络缓冲区
sudo sysctl -w net.core.rmem_max=16777216
sudo sysctl -w net.core.wmem_max=16777216
sudo sysctl -w net.ipv4.tcp_rmem="4096 87380 16777216"
sudo sysctl -w net.ipv4.tcp_wmem="4096 65536 16777216"

# 启用TCP快速打开
sudo sysctl -w net.ipv4.tcp_fastopen=3
```

## 故障排除

### 常见问题

#### 1. Kafka连接失败
```bash
# 检查Kafka服务状态
sudo systemctl status kafka

# 测试连接
telnet kafka-server 9092

# 检查防火墙
sudo ufw status
```

#### 2. 内存不足
```bash
# 监控内存使用
free -h
vmstat 1

# 调整JVM堆大小
export RUST_MIN_STACK=8388608

# 启用内存压缩
echo 1 > /sys/kernel/mm/transparent_hugepage/defrag
```

#### 3. CPU使用率过高
```bash
# 检查进程CPU使用
top -p $(pgrep realtime_streaming)

# 调整线程数
export RAYON_NUM_THREADS=3

# 检查是否有CPU绑定冲突
taskset -p $(pgrep realtime_streaming)
```

#### 4. 磁盘I/O瓶颈
```bash
# 监控磁盘I/O
iostat -x 1

# 检查磁盘健康
sudo smartctl -a /dev/sda

# 调整I/O优先级
ionice -c 2 -n 0 -p $(pgrep realtime_streaming)
```

### 日志分析
```bash
# 查看错误日志
grep "ERROR" /var/lib/edge-compute/logs/*.log | tail -20

# 分析性能日志
grep "processing_time" /var/lib/edge-compute/logs/*.log | \
  awk '{sum+=$2; count++} END {print "Average:", sum/count, "ms"}'
```

## 备份和恢复

### 数据备份
```bash
# 创建备份脚本
sudo tee /usr/local/bin/edge-compute-backup > /dev/null <<EOF
#!/bin/bash
BACKUP_DIR="/var/backups/edge-compute"
TIMESTAMP=\$(date +%Y%m%d_%H%M%S)

# 创建备份目录
mkdir -p \$BACKUP_DIR

# 备份配置
cp /etc/edge-compute/config.json \$BACKUP_DIR/config_\$TIMESTAMP.json

# 备份数据
tar -czf \$BACKUP_DIR/data_\$TIMESTAMP.tar.gz /var/lib/edge-compute/data/

# 清理旧备份（保留7天）
find \$BACKUP_DIR -name "*.tar.gz" -mtime +7 -delete
find \$BACKUP_DIR -name "*.json" -mtime +7 -delete

echo "Backup completed: \$TIMESTAMP"
EOF

sudo chmod +x /usr/local/bin/edge-compute-backup
```

### 配置定时备份
```bash
# 添加到crontab
sudo crontab -e

# 每天凌晨2点执行备份
0 2 * * * /usr/local/bin/edge-compute-backup
```

## 扩展和高可用

### 水平扩展
```bash
# 在新节点上重复部署步骤
# 修改配置文件中的node_id
# 使用相同的group_id加入同一个消费者组
```

### 负载均衡
```bash
# 配置Nginx负载均衡
upstream edge_compute_streaming {
    server edge-node-001:8080 weight=10;
    server edge-node-002:8080 weight=10;
    server edge-node-003:8080 weight=5 backup;
}

server {
    listen 80;
    server_name streaming.example.com;

    location / {
        proxy_pass http://edge_compute_streaming;
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
    }
}
```

## 安全配置

### 网络安全
```bash
# 配置防火墙
sudo ufw default deny incoming
sudo ufw default allow outgoing
sudo ufw allow 22/tcp
sudo ufw allow 8080/tcp
sudo ufw allow 9092/tcp
sudo ufw --force enable
```

### 数据加密
```json
{
  "security": {
    "encryption": {
      "data_at_rest_enabled": true,
      "data_in_transit_enabled": true,
      "certificate_path": "/etc/ssl/certs/edge-compute.crt",
      "private_key_path": "/etc/ssl/private/edge-compute.key"
    }
  }
}
```

## 维护指南

### 定期维护任务
```bash
# 每周清理日志
0 3 * * 1 /usr/bin/find /var/lib/edge-compute/logs -name "*.log.*" -mtime +7 -delete

# 每月更新系统
0 4 1 * * /usr/bin/apt-get update && /usr/bin/apt-get upgrade -y

# 每月重启服务（可选）
0 5 1 * * /usr/bin/systemctl restart edge-compute-streaming
```

### 性能监控
- 每天检查系统资源使用情况
- 每周分析性能指标趋势
- 每月进行完整性检查和优化

### 应急预案
1. **服务宕机**: 自动重启机制
2. **网络故障**: 切换到备份网络
3. **磁盘故障**: 使用RAID或分布式存储
4. **数据丢失**: 从备份恢复

## 技术支持

### 联系方式
- **技术支持**: support@edge-compute.io
- **紧急联系**: +1-800-EDGE-HELP
- **文档**: https://docs.edge-compute.io

### 诊断信息收集
```bash
# 收集系统信息
sudo tee /tmp/system_info.txt > /dev/null <<EOF
=== System Information ===
Date: \$(date)
Uptime: \$(uptime)
Memory: \$(free -h)
Disk: \$(df -h)
CPU: \$(lscpu | grep -E 'Model name|Socket|Core|Thread')
Network: \$(ip addr show)
Processes: \$(ps aux | grep edge-compute | head -10)
EOF

# 收集应用日志
sudo tar -czf /tmp/edge-compute-logs.tar.gz /var/lib/edge-compute/logs/
```

---

**🎉 实时流式计算系统部署完成！**

该系统现在已经准备好处理来自Kafka的实时振动数据流，提供低延迟、高可靠性的边缘计算服务。
