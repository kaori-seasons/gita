# Gita - 企业级边缘计算框架

<div align="center">

![Rust](https://img.shields.io/badge/Rust-1.70+-orange)
![License](https://img.shields.io/badge/License-MIT-green)
![Status](https://img.shields.io/badge/Status-Production%20Ready-blue)

**Rust边缘计算框架**

[快速开始](#快速开始) • [架构设计](#架构设计) • [API文档](#api文档) • [部署指南](#部署指南) • [贡献指南](#贡献指南)

</div>

---

## 项目概述

**Gita** 是一个企业级的边缘计算框架，采用三层架构设计（控制平面、FFI层、执行平面），完整支持从PoC原型到生产环境的全生命周期。

### 核心优势

- 🚀 **高性能**：Tokio异步运行时，支持1000+并发连接
- 🔒 **企业级安全**：TLS 1.3、JWT认证、审计日志、数据加密
- 🐳 **容器化部署**：原生支持Docker、Kubernetes、Helm
- 🔄 **跨语言互操作**：Rust与C++无缝集成，零开销FFI
- 📊 **可观测性**：Prometheus指标、Grafana仪表板、结构化日志
- ✅ **生产就绪**：完善的错误处理、优雅关机、数据持久化

---

## 架构设计

### 三层架构

```
┌─────────────────────────────────────┐
│    控制平面 (Rust/Axum)              │
│ • API服务器  • 任务调度  • 认证授权   │
└──────────────┬──────────────────────┘
               │
               ▼
┌─────────────────────────────────────┐
│   FFI层 (CXX桥接)                   │
│ • 类型安全的跨语言调用  • 内存管理   │
└──────────────┬──────────────────────┘
               │
               ▼
┌─────────────────────────────────────┐
│   执行平面 (C++/Youki)              │
│ • 算法执行  • 容器管理  • 性能监控   │
└─────────────────────────────────────┘
```

### 核心技术栈

| 层级 | 技术 | 用途 |
|------|------|------|
| **运行时** | Tokio | 高性能异步I/O和任务调度 |
| **Web框架** | Axum | 轻量级异步HTTP服务器 |
| **跨语言** | CXX | 类型安全的Rust-C++互操作 |
| **容器** | Youki | OCI标准的容器运行时 |
| **持久化** | Sled | 嵌入式键值数据库 |
| **安全** | rustls/OpenSSL | TLS 1.3加密通信 |
| **监控** | Prometheus/Grafana | 指标收集和可视化 |

### 设计原则

1. **内存安全优先** - 利用Rust的所有权系统避免内存漏洞
2. **异步优先** - Tokio处理高并发，支持io_uring
3. **零开销抽象** - 保证安全的前提下最大化性能
4. **容器化隔离** - Youki提供安全的进程隔离和资源控制

---

## 项目结构

### 核心模块

```
gita/
├── src/                          # 主程序源码
│   ├── core/                     # 核心逻辑
│   │   ├── scheduler.rs          # 任务调度系统
│   │   ├── error.rs              # 错误处理与统计
│   │   ├── persistence.rs        # 数据持久化
│   │   ├── audit.rs              # 审计日志
│   │   ├── tls.rs                # TLS安全传输
│   │   └── types.rs              # 核心数据类型
│   ├── api/                      # HTTP接口层
│   │   ├── handlers.rs           # 业务逻辑处理
│   │   ├── routes.rs             # 路由定义
│   │   ├── server.rs             # HTTP服务器
│   │   ├── auth_middleware.rs    # 认证中间件
│   │   └── container_handlers.rs # 容器管理
│   ├── ffi/                      # Rust-C++互操作
│   │   ├── bridge.rs             # CXX桥接
│   │   ├── cpp/                  # C++算法实现
│   │   ├── memory_manager.rs     # 内存管理
│   │   └── type_converter.rs     # 类型转换
│   ├── config/                   # 配置管理
│   └── lib.rs & main.rs          # 入口点
│
├── rust-edge-compute-core/       # 核心库
├── rust-edge-compute-cpp/        # C++互操作crate
├── rust-edge-compute-ml/         # ML推理支持
├── rust-edge-compute-python/     # Python集成
│
├── cpp_plugins/                  # C++插件实现
│   ├── src/                      # 插件源码
│   ├── include/                  # 头文件
│   ├── tests/                    # 测试用例
│   └── CMakeLists.txt            # CMake构建
│
├── examples/                     # 使用示例
├── tests/                        # 集成测试
├── config/                       # 配置文件
├── deploy/                       # 部署配置
├── docker/                       # Docker文件
├── helm/                         # K8s部署
└── scripts/                      # 自动化脚本
```

---

## 核心功能模块

### 1. 任务调度系统

- **优先级队列**：支持高/中/低三个优先级
- **并发控制**：可配置最大并发任务数（默认10）
- **队列容量**：10000个任务缓冲
- **重试机制**：失败自动重试，可配置重试次数
- **超时控制**：支持任务级别超时设置（默认300s）
- **负载均衡**：任务自动分配到可用worker

### 2. 数据持久化

- **Sled嵌入式数据库**：零配置的键值存储
- **自动备份**：关机时自动保存应用状态
- **错误统计持久化**：记录所有错误和恢复信息
- **任务状态保存**：支持任务中断后恢复
- **配置存储**：运行时配置持久化

### 3. 安全体系

| 维度 | 实现 | 说明 |
|------|------|------|
| **传输安全** | TLS 1.3 | HTTPS加密通信 |
| **认证** | JWT | Token认证，24h过期 |
| **授权** | RBAC | 角色-权限访问控制 |
| **加密存储** | AES-256-GCM | 敏感数据加密 |
| **速率限制** | 令牌桶 | 防DDoS、防滥用 |
| **审计日志** | 结构化日志 | 所有操作记录 |
| **输入验证** | 白名单校验 | 防注入攻击 |

### 4. 可观测性

- **Prometheus指标**：性能、错误率、队列深度等
- **Grafana仪表板**：实时监控可视化
- **结构化日志**：JSON格式日志，便于查询
- **分布式追踪**：OpenTelemetry集成（可选）
- **健康检查**：端点监控系统整体健康状态

### 5. 错误处理与恢复

- **分层错误处理**：API层、业务层、FFI层统一处理
- **错误分类**：临时错误、永久错误、业务错误
- **自动恢复**：临时错误自动重试，定期健康检查
- **错误统计**：详细的错误率和趋势分析
- **优雅降级**：单点故障不影响整体服务

### 6. 容器管理

- **Youki运行时**：OCI标准兼容的轻量级容器
- **生命周期管理**：创建、启动、停止、删除容器
- **资源控制**：CPU、内存、I/O限制
- **隔离执行**：安全的进程隔离和沙箱环境
- **状态监控**：容器运行状态实时跟踪

### 7. 优雅关机

- **信号处理**：SIGTERM/SIGINT优雅关闭
- **状态保存**：关闭前保存所有重要数据
- **组件协调**：确保各组件有序关闭
- **超时控制**：防止无限等待（默认30s）
- **再启动恢复**：关闭前保存的状态自动恢复

---

## 快速开始

### 环境要求

- **Rust**: 1.70+ ([rustup](https://rustup.rs/) 安装)
- **C++**: 编译器（gcc/clang）用于CXX桥接
- **Linux**: 推荐Linux环境（Youki容器支持）
- **Docker**: 可选，用于容器化部署

### 本地开发

```bash
# 1. 克隆项目
git clone <repository-url>
cd gita

# 2. 构建项目
cargo build --release

# 3. 运行服务
./target/release/rust-edge-compute

# 4. 测试API（新开终端）
curl http://localhost:3000/api/v1/health
```

### 运行测试

```bash
# 完整测试套件
./test_runner.sh

# 单元测试
cargo test --lib

# 集成测试
cargo test --test '*'

# 特定测试
cargo test algorithm_name

# 显示println输出
cargo test -- --nocapture
```

### Docker部署

```bash
# 构建镜像
docker build -t gita:latest .

# 运行容器
docker run -p 3000:3000 -p 443:443 gita:latest

# 使用Docker Compose（含监控栈）
docker-compose -f docker/docker-compose.yml up -d
```

### Kubernetes部署

```bash
# 使用Helm部署
helm install gita ./helm

# 检查部署
kubectl get pods
kubectl logs -f deployment/gita

# 查看服务
kubectl get svc
```

---

## API文档

### 认证

所有API均需JWT认证（除登录外）：

```bash
# 1. 登录获取token
curl -X POST http://localhost:3000/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username": "admin", "password": "password"}'

# 响应
{
  "token": "eyJ0eXAiOiJKV1QiLCJhbGc...",
  "expires_in": 86400
}

# 2. 使用token访问API
curl -H "Authorization: Bearer <token>" \
  http://localhost:3000/api/v1/compute
```

### 主要端点

#### 健康检查
```bash
GET /api/v1/health

# 响应
{
  "status": "healthy",
  "service": "gita",
  "version": "0.1.0",
  "timestamp": "2024-01-01T12:00:00Z"
}
```

#### 提交任务
```bash
POST /api/v1/compute
Authorization: Bearer <token>
Content-Type: application/json

{
  "algorithm": "matrix_multiplication",
  "priority": "high",
  "timeout_seconds": 300,
  "parameters": {"a": [[1, 2]], "b": [[3], [4]]}
}

# 响应
{
  "task_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "submitted",
  "created_at": "2024-01-01T12:00:00Z"
}
```

#### 查询任务状态
```bash
GET /api/v1/task/{task_id}
Authorization: Bearer <token>

# 响应
{
  "task_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "completed",
  "result": {...},
  "created_at": "2024-01-01T12:00:00Z",
  "completed_at": "2024-01-01T12:00:15Z"
}
```

#### 调度器状态
```bash
GET /api/v1/scheduler/status
Authorization: Bearer <token>

# 响应
{
  "active_tasks": 5,
  "queued_tasks": 12,
  "max_concurrent": 10,
  "average_latency_ms": 45.2,
  "uptime_seconds": 86400
}
```

#### 错误统计
```bash
GET /api/v1/errors/stats
Authorization: Bearer <token>

# 响应
{
  "total_errors": 15,
  "error_rate": 0.001,
  "recent_errors": [
    {"error": "timeout", "count": 5, "timestamp": "..."}
  ]
}
```

详细API文档请参考 [API完整文档](docs/api.md)

---

## 部署指南

### 生产环境检查清单

- [ ] 配置TLS证书（`deploy/config/`）
- [ ] 设置强密码和JWT密钥
- [ ] 配置监控告警规则
- [ ] 准备数据备份策略
- [ ] 配置日志轮转和保留期
- [ ] 进行负载测试和性能基准
- [ ] 编写运维手册和故障响应流程
- [ ] 设置自动化健康检查

### 关键配置文件

- `config/production.toml` - 生产环境配置
- `deploy/config/production.toml` - 部署特定配置
- `deploy/monitoring/monitor.sh` - 监控脚本

详细部署说明请参考 [部署指南](DEPLOYMENT_QUICK_REFERENCE.md)

---

## 开发指南

### 项目构建

```bash
# 所有crate编译
cargo build --release

# 特定crate编译
cargo build -p rust-edge-compute-core --release

# 编译并运行
cargo run --release

# 编译C++部分
cd cpp_plugins && ./build.sh
```

### 代码规范

```bash
# 格式检查
cargo fmt --check

# 自动格式化
cargo fmt

# Lint检查
cargo clippy -- -D warnings

# 文档生成
cargo doc --open
```

### 添加新的算法

#### 方式1：C++算法

1. 在 `cpp_plugins/include/` 中定义接口
2. 在 `cpp_plugins/src/` 中实现逻辑
3. 在 `src/ffi/bridge.rs` 中添加CXX桥接
4. 在 `src/api/handlers.rs` 中注册API端点

#### 方式2：Rust算法

1. 在 `src/core/` 中添加算法模块
2. 在 `src/ffi/bridge.rs` 中实现算法逻辑
3. 在 `src/api/handlers.rs` 中注册API端点

---

## 生产实践

### 性能调优

1. **并发配置**
   ```toml
   [scheduler]
   max_concurrent_tasks = 20  # 根据服务器能力调整
   queue_capacity = 50000
   ```

2. **内存优化**
   ```bash
   # 构建优化二进制
   cargo build --release
   strip target/release/rust-edge-compute
   ```

3. **监控优化**
   - 配置合理的metrics采集间隔
   - 生产环境日志级别设为INFO
   - 启用结构化日志便于分析

### 故障处理

| 故障 | 症状 | 排查步骤 |
|------|------|---------|
| **高CPU占用** | 任务调度不及时 | 检查并发配置，增加worker数 |
| **内存溢出** | OOM杀进程 | 检查持久化配置，清理过期数据 |
| **连接超时** | 客户端请求失败 | 检查网络配置，增加超时时间 |
| **任务堆积** | 队列不空 | 检查算法执行时间，调整并发数 |

详细故障排查参考 [运维手册](docs/operations.md)

---

## 常见问题

### Q: 如何修改服务监听的端口？
A: 编辑 `config/production.toml`，修改 `server.port` 参数。

### Q: 如何启用HTTPS？
A: 配置TLS证书：
```toml
[server]
use_tls = true
cert_path = "/path/to/cert.pem"
key_path = "/path/to/key.pem"
```

### Q: 如何扩展任务调度的并发数？
A: 修改配置后重启服务：
```toml
[scheduler]
max_concurrent_tasks = 20  # 增加此值
```

### Q: 如何查看系统日志？
A:
```bash
# 本地运行
journalctl -f -u gita

# Docker容器
docker logs -f <container_id>

# Kubernetes
kubectl logs -f -l app=gita
```

### Q: 性能基准是多少？
A: 详见 [MEMORY_MANAGER_PRODUCTION_REPORT.md](MEMORY_MANAGER_PRODUCTION_REPORT.md)

---

## 文档导航

- 📖 [API完整文档](docs/api.md) - 所有API接口详细说明
- 🏗️ [架构设计文档](docs/architecture.md) - 深入理解系统设计
- 🚀 [部署指南](DEPLOYMENT_QUICK_REFERENCE.md) - Docker/K8s部署步骤
- 📊 [监控配置](deploy/monitoring/) - Prometheus/Grafana配置
- 🔒 [安全策略](docs/security.md) - TLS、认证、审计
- ⚙️ [运维手册](docs/operations.md) - 运维和故障处理
- 📈 [性能基准](MEMORY_MANAGER_PRODUCTION_REPORT.md) - 性能测试结果

---

## 项目进度

### ✅ 已完成功能

| 阶段 | 内容 | 进度 |
|------|------|------|
| **Phase 1: PoC** | 项目架构、HTTP服务、CXX集成、容器管理、端到端测试 | ✅ 100% |
| **Phase 2: MVP** | 任务调度、错误处理、数据持久化、优雅关机、安全配置 | ✅ 100% |
| **Phase 3: 生产就绪** | 安全加固、性能优化、监控工具、OTA更新、生产部署 | ✅ 100% |

### 📊 系统规格

| 指标 | 规格 |
|------|------|
| **并发连接** | 1000+ |
| **并发任务** | 10个 |
| **队列容量** | 10000个任务 |
| **任务调度延迟** | <100ms |
| **平均响应时间** | <1s |
| **加密方案** | TLS 1.3 + AES-256-GCM |
| **认证方式** | JWT + RBAC |
| **可用性** | 优雅关机 + 自动恢复 + 数据持久化 |

---

## 贡献指南

我们欢迎所有贡献！请遵循以下步骤：

1. **Fork项目** - 创建自己的副本
2. **创建分支** - 为新功能创建分支
   ```bash
   git checkout -b feature/your-feature
   ```
3. **提交代码** - 遵循代码规范
   ```bash
   cargo fmt
   cargo clippy
   cargo test
   ```
4. **提交PR** - 描述你的改进

### 开发规范

- ✅ 所有代码必须通过 `cargo fmt` 和 `cargo clippy`
- ✅ 新功能必须包含单元测试
- ✅ 必须更新相关文档
- ✅ 提交信息要清晰明确
- ✅ 遵循Rust API文档惯例

---

## 许可证

MIT License - 详见 [LICENSE](LICENSE) 文件

---

## 联系方式

- 📧 Email: support@example.com
- 📝 Issues: [GitHub Issues](https://github.com/your-org/gita/issues)
- 📚 Wiki: [项目Wiki](https://github.com/your-org/gita/wiki)

---

<div align="center">

**Made with ❤️ by the Edge Compute Team**

[⬆ 返回顶部](#gita---企业级边缘计算框架)

</div>
