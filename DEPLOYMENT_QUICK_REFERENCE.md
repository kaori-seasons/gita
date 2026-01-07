# ⚡ 部署快速参考 - 一页纸总结

## 🎯 选择你的部署方案（5秒钟决定）

```
你想要：
  1️⃣  快速测试          → Docker Compose
  2️⃣  单机生产          → Docker Compose + 监控
  3️⃣  小集群 (3-10)     → Docker Swarm
  4️⃣  企业级 (10+)      → Kubernetes
  5️⃣  云原生             → 云托管 K8s (EKS/AKS/GKE)
```

---

## 🚀 三种最流行的部署方式

### 1. Docker Compose（推荐学习者）

**启动一个完整的计算系统，仅需 3 条命令：**

```bash
git clone <repo> && cd rust-edge-compute
docker-compose -f docker/docker-compose.yml up -d
curl http://localhost:3000/api/v1/health
```

**你会得到：**
- ✅ 计算引擎（Rust）
- ✅ 缓存（Redis）
- ✅ 数据库（PostgreSQL）
- ✅ 负载均衡（Nginx）
- ✅ 监控（Prometheus + Grafana）

**花费时间：** ⏱️ 5 分钟

**适用场景：**
- 🧪 开发和测试
- 📚 学习框架
- 🖥️ 单机部署

---

### 2. Kubernetes + Helm（推荐企业）

**启动一个分布式计算集群：**

```bash
# 前提：有 Kubernetes 集群
kubectl create namespace edge-compute
helm install rust-edge-compute ./helm -n edge-compute
```

**你会得到：**
- ✅ 3+ 副本自动部署
- ✅ 自动故障转移
- ✅ 负载均衡
- ✅ 自动扩展（HPA）
- ✅ 版本管理和回滚

**花费时间：** ⏱️ 15 分钟

**适用场景：**
- 🏢 企业级生产
- 🌍 多地域部署
- 📊 需要高可用性

---

### 3. 云托管 Kubernetes（推荐上云）

**在 AWS/Azure/Google Cloud 上启动集群：**

```bash
# AWS EKS
eksctl create cluster --name edge-compute
aws eks update-kubeconfig --name edge-compute
helm install rust-edge-compute ./helm

# 或 Azure AKS
az aks create --name edge-compute --resource-group mygroup
az aks get-credentials --name edge-compute
helm install rust-edge-compute ./helm
```

**你会得到：**
- ✅ 完全托管的 Kubernetes
- ✅ 无需维护 Control Plane
- ✅ 自动更新和升级
- ✅ 集成的日志和监控

**花费时间：** ⏱️ 10 分钟

**适用场景：**
- ☁️ AWS/Azure/Google Cloud 用户
- 🌍 全球分布式系统
- 📊 不想自己管理基础设施

---

## 📊 对比速查表

| 特性 | Docker Compose | Kubernetes | 云 K8s |
|-----|--------|--------|--------|
| 启动时间 | ⚡ 5分钟 | ⚡⚡ 15分钟 | ⚡⚡⚡ 10分钟 |
| 学习难度 | ⭐ 简单 | ⭐⭐⭐ 复杂 | ⭐⭐ 中等 |
| 可扩展性 | ❌ 单机 | ✅ 无限 | ✅ 无限 |
| 高可用性 | ❌ 否 | ✅ 是 | ✅ 是 |
| 自动扩展 | ❌ 否 | ✅ 是 | ✅ 是 |
| 成本 | 💰 免费 | 💰 自购 | 💰💰 订阅 |
| 推荐规模 | 1 个节点 | 10+ 个节点 | 任意规模 |

---

## 🔥 常用命令速查

### Docker Compose

```bash
# 启动所有服务
docker-compose -f docker/docker-compose.yml up -d

# 查看日志
docker-compose logs -f rust-edge-compute

# 停止服务
docker-compose -f docker/docker-compose.yml down

# 进入容器
docker-compose exec rust-edge-compute bash
```

### Kubernetes + Helm

```bash
# 部署
helm install rust-edge-compute ./helm -n edge-compute

# 查看状态
helm status rust-edge-compute -n edge-compute

# 升级
helm upgrade rust-edge-compute ./helm -n edge-compute

# 卸载
helm uninstall rust-edge-compute -n edge-compute

# 查看 Pod
kubectl get pods -n edge-compute

# 查看日志
kubectl logs -f deployment/rust-edge-compute -n edge-compute
```

---

## 🌐 访问你的应用

部署完成后，访问以下地址：

| 服务 | 地址 | 用户名 | 密码 |
|-----|------|--------|------|
| **API 应用** | http://localhost:3000 | - | - |
| **Grafana** | http://localhost:3001 | admin | admin |
| **Prometheus** | http://localhost:9090 | - | - |

---

## ❓ 常见问题（快速解答）

**Q1: 我是新手，应该选哪个？**  
A: 选择 **Docker Compose**。5 分钟启动，最简单！

**Q2: 我想要高可用性（HA）**  
A: 选择 **Kubernetes**。支持自动故障转移。

**Q3: 我只有一台服务器**  
A: 选择 **Docker Compose**。单机最佳选择。

**Q4: 我有 10 个节点，想要自动扩展**  
A: 选择 **Kubernetes**。支持 HPA 自动扩展。

**Q5: 我想用 AWS/Azure/GCP**  
A: 选择**云托管 K8s**。无需维护基础设施。

**Q6: 如何从 Docker Compose 迁移到 K8s？**  
A: 配置和数据持久化方式改变。详见完整指南。

**Q7: 需要多少资源？**  
A: Docker Compose: 4GB RAM；K8s: 8GB+ RAM

---

## 📚 完整文档

| 文档 | 用途 |
|-----|------|
| `QUICK_START_CLUSTER.md` | 👈 **推荐首先阅读** |
| `CLUSTER_DEPLOYMENT_GUIDE.md` | 详细的部署步骤和配置 |
| `DEPLOYMENT_OPTIONS_SUMMARY.md` | 各种部署方案深度对比 |

---

## 🎯 5分钟快速开始

### 步骤 1: 进入项目目录
```bash
cd rust-edge-compute
```

### 步骤 2: 启动（选择一种方式）

**方式A: Docker Compose（推荐初学者）**
```bash
docker-compose -f docker/docker-compose.yml up -d
```

**方式B: Kubernetes（推荐企业）**
```bash
kubectl create namespace edge-compute
helm install rust-edge-compute ./helm -n edge-compute
```

### 步骤 3: 验证
```bash
curl http://localhost:3000/api/v1/health
# 预期输出: {"status":"healthy",...}
```

### 步骤 4: 访问监控
```bash
# 打开浏览器
http://localhost:3001  # Grafana
http://localhost:9090  # Prometheus
```

✅ **完成！你的第一个计算集群已就绪！**

---

## 🚀 下一步行动

1. **现在就试：** 按照上面的 5 分钟步骤启动
2. **了解更多：** 阅读 `QUICK_START_CLUSTER.md`
3. **深入学习：** 查看 `CLUSTER_DEPLOYMENT_GUIDE.md`
4. **生产部署：** 根据规模选择合适的方案

---

## 💡 最佳实践

✅ **开发环境**
```bash
docker-compose up
# 简单、快速、包含所有工具
```

✅ **测试环境**
```bash
docker-compose up
# 或小规模 Kubernetes (3 节点)
```

✅ **生产环境**
```bash
# 大规模（10+ 节点）
helm install ... -n production
# 启用自动扩展、监控告警、备份恢复
```

✅ **多地域部署**
```bash
# 使用云托管 K8s (EKS/AKS/GKE)
# 每个地区一个集群
# 通过 GitOps 同步配置
```

---

## 📞 需要帮助？

| 问题 | 查看文件 |
|-----|--------|
| 如何快速开始？ | `QUICK_START_CLUSTER.md` |
| 详细的部署步骤？ | `CLUSTER_DEPLOYMENT_GUIDE.md` |
| 如何选择部署方案？ | `DEPLOYMENT_OPTIONS_SUMMARY.md` |
| 脚本命令帮助？ | `./scripts/cluster-deploy.sh help` |

---

## 🎉 准备好了吗？

**现在就启动你的计算集群：**

```bash
cd rust-edge-compute
docker-compose -f docker/docker-compose.yml up -d
```

**然后访问：** http://localhost:3000

**祝你使用愉快！** 🚀
