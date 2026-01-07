# 🚀 计算集群部署指南 - Rust Edge Compute Framework

## 快速导航

- [第一部分：架构概述](#架构概述)
- [第二部分：单机部署（Docker Compose）](#单机部署docker-compose)
- [第三部分：Kubernetes集群部署](#kubernetes集群部署)
- [第四部分：多节点集群形成](#多节点集群形成)
- [第五部分：监控与管理](#监控与管理)
- [第六部分：性能优化与扩展](#性能优化与扩展)

---

## 架构概述

### 部署拓扑图

```
┌─────────────────────────────────────────────────────────────┐
│                    计算集群架构                              │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  负载均衡层 (Load Balancing)                                 │
│  ┌─────────────────────────────────────────────────────┐    │
│  │ Nginx LB / Kubernetes Service / Cloud LB            │    │
│  └─────────────────────────────────────────────────────┘    │
│                          │                                    │
│        ┌─────────────────┼─────────────────┐                │
│        ▼                 ▼                 ▼                 │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐       │
│  │  Edge Node 1 │  │  Edge Node 2 │  │  Edge Node N │  计算节点层│
│  │              │  │              │  │              │       │
│  │ ┌──────────┐ │  │ ┌──────────┐ │  │ ┌──────────┐ │       │
│  │ │ Compute  │ │  │ │ Compute  │ │  │ │ Compute  │ │       │
│  │ │ Engine   │ │  │ │ Engine   │ │  │ │ Engine   │ │       │
│  │ └──────────┘ │  │ └──────────┘ │  │ └──────────┘ │       │
│  │              │  │              │  │              │       │
│  │ ┌──────────┐ │  │ ┌──────────┐ │  │ ┌──────────┐ │       │
│  │ │ Plugins  │ │  │ │ Plugins  │ │  │ │ Plugins  │ │       │
│  │ │ & ML     │ │  │ │ & ML     │ │  │ │ & ML     │ │       │
│  │ └──────────┘ │  │ └──────────┘ │  │ └──────────┘ │       │
│  └──────────────┘  └──────────────┘  └──────────────┘       │
│        │                 │                 │                 │
│        └─────────────────┼─────────────────┘                 │
│                          ▼                                    │
│  ┌──────────────────────────────────────────────────────┐    │
│  │          共享服务层 (Shared Services)                 │    │
│  │                                                       │    │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐│    │
│  │  │   Redis      │  │ PostgreSQL   │  │   Kafka      ││    │
│  │  │   Cluster    │  │   Database   │  │   Broker     ││    │
│  │  └──────────────┘  └──────────────┘  └──────────────┘│    │
│  └──────────────────────────────────────────────────────┘    │
│                          │                                    │
│                          ▼                                    │
│  ┌──────────────────────────────────────────────────────┐    │
│  │      监控与可观测性 (Monitoring & Observability)     │    │
│  │                                                       │    │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐│    │
│  │  │ Prometheus   │  │   Grafana    │  │ ELK Stack    ││    │
│  │  │ Metrics      │  │ Dashboards   │  │ Logging      ││    │
│  │  └──────────────┘  └──────────────┘  └──────────────┘│    │
│  └──────────────────────────────────────────────────────┘    │
│                                                               │
└─────────────────────────────────────────────────────────────┘
```

### 部署方式选择

| 部署方式 | 适用场景 | 难度 | 扩展性 | 成本 |
|---------|---------|------|-------|------|
| **Docker Compose** | 单机、开发、测试 | ⭐ 简单 | ⭐ 低 | 💰 低 |
| **Docker Swarm** | 小规模集群（3-10节点） | ⭐⭐ 中等 | ⭐⭐ 中等 | 💰 低 |
| **Kubernetes** | 大规模生产集群 | ⭐⭐⭐ 复杂 | ⭐⭐⭐ 高 | 💰💰 中高 |
| **云服务** | 云原生、自动扩展 | ⭐⭐ 中等 | ⭐⭐⭐ 高 | 💰💰💰 高 |

---

## 单机部署（Docker Compose）

### 前置条件

```bash
# 系统要求
- Docker >= 20.10
- Docker Compose >= 1.29
- Linux 4.15+ kernel (或 Windows 10+ / macOS 10.15+)
- 最小配置: 4核 CPU, 8GB RAM, 20GB 存储

# 安装 Docker
curl -fsSL https://get.docker.com | sh
sudo usermod -aG docker $USER

# 安装 Docker Compose
sudo curl -L "https://github.com/docker/compose/releases/download/v2.20.0/docker-compose-$(uname -s)-$(uname -m)" -o /usr/local/bin/docker-compose
sudo chmod +x /usr/local/bin/docker-compose
```

### 部署步骤

#### 1. 准备环境

```bash
# 1.1 克隆项目
git clone https://github.com/your-org/rust-edge-compute.git
cd rust-edge-compute

# 1.2 创建必要的目录
mkdir -p docker/{data,logs,certs,config}
mkdir -p data/{db,cache}
mkdir -p logs

# 1.3 配置权限
sudo chown -R $(whoami):$(whoami) docker data logs

# 1.4 配置环境变量
cat > .env << EOF
# 应用配置
RUST_LOG=info
SERVER_HOST=0.0.0.0
SERVER_PORT=3000

# 数据库配置
POSTGRES_DB=rust_edge_compute
POSTGRES_USER=rustapp
POSTGRES_PASSWORD=$(openssl rand -base64 32)

# Redis配置
REDIS_PASSWORD=$(openssl rand -base64 32)

# 监控配置
METRICS_ENABLED=true
PROMETHEUS_SCRAPE_INTERVAL=15s

# 集群配置
NODE_ID=edge-node-001
CLUSTER_NAME=my-edge-cluster
EOF
```

#### 2. 启动服务

```bash
# 2.1 构建镜像
docker-compose -f docker/docker-compose.yml build

# 2.2 启动所有服务
docker-compose -f docker/docker-compose.yml up -d

# 2.3 验证服务状态
docker-compose -f docker/docker-compose.yml ps

# 输出应该显示所有服务都在运行 (Up)
```

#### 3. 验证部署

```bash
# 3.1 检查应用健康状况
curl http://localhost:3000/api/v1/health

# 预期响应：{"status":"healthy","uptime":"XXX"}

# 3.2 检查Redis连接
docker-compose exec redis redis-cli ping
# 预期：PONG

# 3.3 检查数据库连接
docker-compose exec postgres psql -U rustapp -d rust_edge_compute -c "\dt"

# 3.4 查看日志
docker-compose logs -f rust-edge-compute
```

#### 4. 配置管理

```bash
# 4.1 应用配置（可选）
# 编辑配置文件
vim config/production.toml

# 4.2 更新配置后重启
docker-compose -f docker/docker-compose.yml restart rust-edge-compute

# 4.3 备份数据
docker-compose exec postgres pg_dump -U rustapp rust_edge_compute > backup_$(date +%Y%m%d).sql
```

### 监控与日志

```bash
# 实时日志
docker-compose logs -f

# 特定服务日志
docker-compose logs -f rust-edge-compute

# 查看资源使用
docker stats

# 访问监控仪表板
# Prometheus: http://localhost:9090
# Grafana: http://localhost:3001 (admin/admin)
```

---

## Kubernetes集群部署

### 前置条件

#### 环境要求

```bash
# Kubernetes集群要求
- Kubernetes >= 1.20
- kubectl >= 1.20
- 节点最小配置: 2核 CPU, 4GB RAM
- 推荐配置: 4核 CPU, 8GB RAM

# 存储要求
- 默认存储类 (Default StorageClass)
- 或自定义 PV (PersistentVolume)

# 网络要求
- Pod 网络插件 (Calico, Flannel等)
- Ingress 控制器 (Nginx, Traefik等)
```

#### 集群初始化

```bash
# 1. 如果使用本地Kubernetes (minikube/Docker Desktop)
minikube start --cpus=4 --memory=8192 --disk-size=20g

# 2. 验证集群
kubectl cluster-info
kubectl get nodes

# 3. 检查存储类
kubectl get storageclass

# 4. 如果没有默认存储，创建一个
kubectl apply -f - << 'EOF'
apiVersion: storage.k8s.io/v1
kind: StorageClass
metadata:
  name: standard
provisioner: kubernetes.io/no-provisioner
volumeBindingMode: WaitForFirstConsumer
EOF
```

### 使用Helm部署（推荐）

Helm 是 Kubernetes 的包管理工具，最简洁的部署方式。

#### 1. 安装Helm

```bash
# 下载并安装Helm
curl https://raw.githubusercontent.com/helm/helm/main/scripts/get-helm-3 | bash

# 验证安装
helm version
```

#### 2. 配置Helm部署

```bash
# 2.1 进入项目目录
cd rust-edge-compute

# 2.2 自定义配置（可选）
# 编辑 helm/values.yaml 中的参数
# 重要参数：
# - replicaCount: 副本数（推荐 >= 3）
# - image.tag: 镜像标签
# - persistence.data.size: 数据存储大小
# - resources.limits: 资源限制

# 2.3 创建命名空间
kubectl create namespace edge-compute

# 2.4 部署应用
helm install rust-edge-compute ./helm \
  --namespace edge-compute \
  --values helm/values.yaml

# 或升级已有部署
helm upgrade rust-edge-compute ./helm \
  --namespace edge-compute \
  --values helm/values.yaml
```

#### 3. 验证Helm部署

```bash
# 3.1 检查发布状态
helm list -n edge-compute

# 3.2 查看部署详情
helm status rust-edge-compute -n edge-compute

# 3.3 获取Release信息
helm get values rust-edge-compute -n edge-compute
helm get manifest rust-edge-compute -n edge-compute
```

### 直接使用Kubernetes YAML部署

```bash
# 1. 创建命名空间
kubectl create namespace edge-compute

# 2. 部署应用
kubectl apply -f k8s/deployment.yaml -n edge-compute

# 3. 检查部署状态
kubectl get deployments -n edge-compute
kubectl get pods -n edge-compute
kubectl get services -n edge-compute

# 4. 查看日志
kubectl logs -f deployment/rust-edge-compute -n edge-compute

# 5. 端口转发（本地测试）
kubectl port-forward svc/rust-edge-compute-service 3000:80 -n edge-compute

# 6. 测试服务
curl http://localhost:3000/api/v1/health
```

### 配置持久化存储

```bash
# 1. 创建 PersistentVolume（可选，如果没有自动存储配置）
kubectl apply -f - << 'EOF'
apiVersion: v1
kind: PersistentVolume
metadata:
  name: edge-compute-data-pv
spec:
  capacity:
    storage: 10Gi
  accessModes:
    - ReadWriteOnce
  hostPath:
    path: "/data/edge-compute"
---
apiVersion: v1
kind: PersistentVolume
metadata:
  name: edge-compute-logs-pv
spec:
  capacity:
    storage: 5Gi
  accessModes:
    - ReadWriteOnce
  hostPath:
    path: "/logs/edge-compute"
EOF

# 2. 验证PV创建
kubectl get pv
```

---

## 多节点集群形成

### 架构规划

对于生产环境集群，推荐以下架构：

```
┌──────────────────────────────────────────────────────────────┐
│                    生产集群架构 (3+ 节点)                    │
├──────────────────────────────────────────────────────────────┤
│                                                                │
│  控制面板 (Control Plane) - 1 节点                            │
│  ├─ Kubernetes API Server                                    │
│  ├─ Controller Manager                                        │
│  └─ Scheduler                                                 │
│                                                                │
│  计算节点 (Worker Nodes) - 3+ 节点                            │
│  ├─ Edge Node 1 (4 CPU, 8GB RAM)                             │
│  ├─ Edge Node 2 (4 CPU, 8GB RAM)                             │
│  └─ Edge Node N (4 CPU, 8GB RAM)                             │
│                                                                │
│  数据层 (Data Plane) - 专用存储节点 (可选)                    │
│  ├─ NFS/Ceph Server                                          │
│  └─ Database Server (High-Availability)                      │
│                                                                │
└──────────────────────────────────────────────────────────────┘
```

### 部署步骤

#### 第一步：准备物理或虚拟节点

```bash
# 在每个节点上执行（Ubuntu 20.04 LTS 为例）

# 1. 更新系统
sudo apt-get update
sudo apt-get upgrade -y

# 2. 安装 Docker
curl -fsSL https://get.docker.com | sh
sudo usermod -aG docker $USER

# 3. 安装 Kubernetes 工具
curl -s https://packages.cloud.google.com/apt/doc/apt-key.gpg | sudo apt-key add -
sudo apt-get install -y kubelet kubeadm kubectl

# 4. 禁用 swap
sudo swapoff -a
sudo sed -i '/ swap / s/^/#/' /etc/fstab

# 5. 启用内核模块
sudo modprobe overlay
sudo modprobe br_netfilter

# 6. 配置系统参数
cat | sudo tee /etc/sysctl.d/99-kubernetes-cri.conf <<EOF
net.bridge.bridge-nf-call-iptables  = 1
net.bridge.bridge-nf-call-ip6tables = 1
net.ipv4.ip_forward                 = 1
EOF
sudo sysctl --system
```

#### 第二步：初始化控制平面（Master节点）

```bash
# 在 Master 节点上执行

# 1. 初始化集群
sudo kubeadm init \
  --pod-network-cidr=10.244.0.0/16 \
  --apiserver-advertise-address=<MASTER_IP> \
  --control-plane-endpoint=<MASTER_HOSTNAME>

# 2. 配置 kubectl
mkdir -p $HOME/.kube
sudo cp /etc/kubernetes/admin.conf $HOME/.kube/config
sudo chown $(id -u):$(id -g) $HOME/.kube/config

# 3. 安装网络插件（Flannel）
kubectl apply -f https://raw.githubusercontent.com/coreos/flannel/master/Documentation/kube-flannel.yml

# 4. 验证 Master 节点就绪
kubectl get nodes  # 应该显示 "NotReady"，待网络插件就绪后变为 "Ready"
```

#### 第三步：加入工作节点（Worker节点）

```bash
# 在 Master 节点上生成加入令牌
kubeadm token create --print-join-command

# 输出会类似：
# kubeadm join <MASTER_IP>:6443 --token <TOKEN> --discovery-token-ca-cert-hash sha256:<HASH>

# 在每个 Worker 节点上执行上述命令（带 sudo）
sudo kubeadm join <MASTER_IP>:6443 --token <TOKEN> --discovery-token-ca-cert-hash sha256:<HASH>

# 验证节点加入
kubectl get nodes  # 应该显示所有节点
```

#### 第四步：配置共享存储（可选但推荐）

##### 方案A: NFS 共享存储

```bash
# 在存储服务器上（或任意节点）
sudo apt-get install -y nfs-kernel-server
sudo mkdir -p /data/k8s-storage
sudo chown nobody:nogroup /data/k8s-storage
sudo chmod 777 /data/k8s-storage

# 编辑 /etc/exports
sudo tee /etc/exports << 'EOF'
/data/k8s-storage *(rw,sync,no_subtree_check,no_root_squash)
EOF

# 重启NFS服务
sudo systemctl restart nfs-kernel-server

# 在 Kubernetes 集群中部署 NFS Provisioner
helm repo add nfs-subdir-external-provisioner https://kubernetes-sigs.github.io/nfs-subdir-external-provisioner/
helm install nfs-provisioner nfs-subdir-external-provisioner/nfs-subdir-external-provisioner \
  --set nfs.server=<NFS_SERVER_IP> \
  --set nfs.path=/data/k8s-storage
```

##### 方案B: 本地存储（每节点本地磁盘）

```bash
# 为每个节点配置本地存储
kubectl apply -f - << 'EOF'
apiVersion: storage.k8s.io/v1
kind: StorageClass
metadata:
  name: local-storage
provisioner: kubernetes.io/no-provisioner
volumeBindingMode: WaitForFirstConsumer
EOF
```

#### 第五步：部署应用到集群

```bash
# 1. 构建并推送镜像到镜像仓库
docker build -t <REGISTRY>/rust-edge-compute:v1.0 .
docker push <REGISTRY>/rust-edge-compute:v1.0

# 2. 更新 helm/values.yaml 中的镜像地址
image:
  repository: <REGISTRY>/rust-edge-compute
  tag: v1.0

# 3. 部署（使用Helm）
helm install rust-edge-compute ./helm \
  --namespace edge-compute \
  --create-namespace

# 4. 验证部署
kubectl get pods -n edge-compute
kubectl get svc -n edge-compute
```

---

## 水平扩展（Scale-Out）

### 动态调整副本数

```bash
# 使用 kubectl scale
kubectl scale deployment rust-edge-compute \
  --replicas=5 \
  -n edge-compute

# 或编辑部署
kubectl edit deployment rust-edge-compute -n edge-compute
# 修改 spec.replicas 的值

# 或使用 Helm
helm upgrade rust-edge-compute ./helm \
  --replicaCount=5 \
  -n edge-compute
```

### 自动扩展（HPA - Horizontal Pod Autoscaler）

```bash
# 1. 启用 Metrics Server（用于收集 Pod 指标）
kubectl apply -f https://github.com/kubernetes-sigs/metrics-server/releases/latest/download/components.yaml

# 2. 创建 HPA
kubectl apply -f - << 'EOF'
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: rust-edge-compute-hpa
  namespace: edge-compute
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: rust-edge-compute
  minReplicas: 3
  maxReplicas: 10
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
  - type: Resource
    resource:
      name: memory
      target:
        type: Utilization
        averageUtilization: 80
  behavior:
    scaleDown:
      stabilizationWindowSeconds: 300
      policies:
      - type: Percent
        value: 50
        periodSeconds: 60
    scaleUp:
      stabilizationWindowSeconds: 0
      policies:
      - type: Percent
        value: 100
        periodSeconds: 15
      - type: Pods
        value: 2
        periodSeconds: 60
      selectPolicy: Max
EOF

# 3. 验证 HPA 状态
kubectl get hpa -n edge-compute
kubectl describe hpa rust-edge-compute-hpa -n edge-compute
```

---

## 负载均衡与服务发现

### Kubernetes Service（内置）

```bash
# 查看服务
kubectl get svc -n edge-compute

# 服务类型说明：
# - ClusterIP: 仅内部访问（默认）
# - NodePort: 通过节点IP+端口访问
# - LoadBalancer: 云负载均衡器
# - ExternalName: 外部DNS别名
```

### Ingress 配置

```yaml
# k8s/ingress.yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: rust-edge-compute-ingress
  namespace: edge-compute
  annotations:
    nginx.ingress.kubernetes.io/rewrite-target: /
    nginx.ingress.kubernetes.io/ssl-redirect: "true"
spec:
  ingressClassName: nginx
  tls:
  - hosts:
    - compute.example.com
    secretName: edge-compute-tls
  rules:
  - host: compute.example.com
    http:
      paths:
      - path: /api
        pathType: Prefix
        backend:
          service:
            name: rust-edge-compute-service
            port:
              number: 3000
      - path: /metrics
        pathType: Prefix
        backend:
          service:
            name: rust-edge-compute-service
            port:
              number: 9090
```

部署 Ingress：

```bash
# 1. 安装 Nginx Ingress Controller（如果未安装）
helm repo add ingress-nginx https://kubernetes.github.io/ingress-nginx
helm install nginx-ingress ingress-nginx/ingress-nginx

# 2. 部署 Ingress 规则
kubectl apply -f k8s/ingress.yaml

# 3. 验证
kubectl get ingress -n edge-compute
```

---

## 监控与管理

### 部署监控栈

#### 方案1: Helm 部署 Prometheus + Grafana

```bash
# 1. 添加 Prometheus 社区 Helm 仓库
helm repo add prometheus-community https://prometheus-community.github.io/helm-charts
helm repo add grafana https://grafana.github.io/helm-charts
helm repo update

# 2. 安装 Prometheus
helm install prometheus prometheus-community/kube-prometheus-stack \
  --namespace monitoring \
  --create-namespace \
  --set prometheus.prometheusSpec.retention=15d \
  --set grafana.adminPassword=YourSecurePassword

# 3. 访问 Grafana
kubectl port-forward -n monitoring svc/prometheus-grafana 3000:80
# 访问: http://localhost:3000 (admin/YourSecurePassword)
```

#### 方案2: 部署应用的 ServiceMonitor

```yaml
# k8s/service-monitor.yaml
apiVersion: monitoring.coreos.com/v1
kind: ServiceMonitor
metadata:
  name: rust-edge-compute
  namespace: edge-compute
spec:
  selector:
    matchLabels:
      app: rust-edge-compute
  endpoints:
  - port: metrics
    interval: 30s
    path: /metrics
```

### 查看日志和指标

```bash
# 1. 实时日志查看
kubectl logs -f deployment/rust-edge-compute -n edge-compute

# 2. 查看特定 Pod 日志
kubectl logs -f <POD_NAME> -n edge-compute

# 3. 查看前面 100 行日志
kubectl logs -n edge-compute deployment/rust-edge-compute --tail=100

# 4. 导出日志用于分析
kubectl logs deployment/rust-edge-compute -n edge-compute > logs.txt

# 5. 查看事件
kubectl get events -n edge-compute
kubectl describe pod <POD_NAME> -n edge-compute
```

---

## 故障排查与恢复

### 常见问题

#### 问题1: Pod 无法启动（CrashLoopBackOff）

```bash
# 1. 查看 Pod 状态
kubectl describe pod <POD_NAME> -n edge-compute

# 2. 查看日志
kubectl logs <POD_NAME> -n edge-compute

# 3. 检查资源限制
kubectl top pods -n edge-compute

# 4. 增加资源限制（在 values.yaml 中）
resources:
  limits:
    cpu: 1000m
    memory: 1024Mi
  requests:
    cpu: 500m
    memory: 512Mi
```

#### 问题2: 服务无法访问

```bash
# 1. 检查 Service
kubectl get svc -n edge-compute
kubectl describe svc rust-edge-compute-service -n edge-compute

# 2. 检查 Pod 是否就绪
kubectl get pods -n edge-compute -o wide

# 3. 测试 Pod 网络连通性
kubectl exec <POD_NAME> -n edge-compute -- curl localhost:3000/api/v1/health

# 4. 检查网络策略
kubectl get networkpolicies -n edge-compute
```

#### 问题3: 存储挂载失败

```bash
# 1. 查看 PVC 状态
kubectl get pvc -n edge-compute
kubectl describe pvc rust-edge-compute-data -n edge-compute

# 2. 查看 PV 状态
kubectl get pv
kubectl describe pv <PV_NAME>

# 3. 检查存储类
kubectl get storageclass

# 4. 查看挂载详情
kubectl exec <POD_NAME> -n edge-compute -- mount | grep /app/data
```

### 备份与恢复

```bash
# 1. 备份 etcd（集群配置数据库）
kubectl exec -n kube-system etcd-<MASTER_NODE> -- \
  etcdctl --endpoints=127.0.0.1:2379 \
  --cacert=/etc/kubernetes/pki/etcd/ca.crt \
  --cert=/etc/kubernetes/pki/etcd/server.crt \
  --key=/etc/kubernetes/pki/etcd/server.key \
  snapshot save /tmp/etcd-backup.db

# 2. 备份应用数据
kubectl get all -n edge-compute -o yaml > edge-compute-backup.yaml

# 3. 恢复应用
kubectl apply -f edge-compute-backup.yaml

# 4. 备份数据库
kubectl exec -n edge-compute <POSTGRES_POD> -- \
  pg_dump -U rustapp rust_edge_compute > db-backup.sql

# 5. 恢复数据库
kubectl exec -i -n edge-compute <POSTGRES_POD> -- \
  psql -U rustapp rust_edge_compute < db-backup.sql
```

---

## 性能优化与扩展

### 性能调优

#### 网络优化

```bash
# 1. 启用 Pod 优先级和抢占
kubectl apply -f - << 'EOF'
apiVersion: scheduling.k8s.io/v1
kind: PriorityClass
metadata:
  name: high-priority
value: 1000
globalDefault: false
description: "High priority for compute tasks"
EOF

# 2. 配置亲和性（亲和 Pod 分布）
affinity:
  podAntiAffinity:
    preferredDuringSchedulingIgnoredDuringExecution:
    - weight: 100
      podAffinityTerm:
        labelSelector:
          matchExpressions:
          - key: app
            operator: In
            values:
            - rust-edge-compute
        topologyKey: kubernetes.io/hostname
```

#### 资源优化

```bash
# 1. 设置合理的资源请求/限制
resources:
  requests:
    cpu: 250m      # 初始分配
    memory: 256Mi
  limits:
    cpu: 500m      # 最大限制
    memory: 512Mi

# 2. 启用 QoS (Quality of Service)
# Guaranteed: requests == limits
# Burstable: requests < limits
# BestEffort: 无requests/limits

# 3. 调整 Pod 驱逐阈值
--eviction-hard=memory.available<5%,nodefs.available<5%
--eviction-soft=memory.available<10%,nodefs.available<10%
--eviction-soft-grace-period=memory.available=1m30s
```

### 扩展性设计

#### 多集群部署

```bash
# 1. 部署在多个地理位置的集群
# Cluster A (Region 1): 3+ nodes
# Cluster B (Region 2): 3+ nodes
# Cluster C (Region 3): 3+ nodes

# 2. 使用 Kubernetes Federation 或 GitOps 同步配置
helm repo add sealed-secrets https://bitnami-labs.github.io/sealed-secrets
helm install sealed-secrets -n kube-system sealed-secrets/sealed-secrets

# 3. 使用 ArgoCD 进行应用部署
helm repo add argocd https://argoproj.github.io/argo-helm
helm install argocd argocd/argo-cd -n argocd --create-namespace
```

#### 自定义插件与扩展

```bash
# 在容器中添加自定义插件
volumeMounts:
- name: plugins
  mountPath: /app/plugins

volumes:
- name: plugins
  configMap:
    name: custom-plugins

# 动态加载插件（不需要重启）
# 应用会自动检测 /app/plugins 目录下的新插件
```

---

## 安全最佳实践

### 网络安全

```yaml
# 1. 网络策略 - 限制流量
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: rust-edge-compute-netpolicy
  namespace: edge-compute
spec:
  podSelector:
    matchLabels:
      app: rust-edge-compute
  policyTypes:
  - Ingress
  - Egress
  ingress:
  - from:
    - namespaceSelector:
        matchLabels:
          name: edge-compute
    ports:
    - protocol: TCP
      port: 3000
  egress:
  - to:
    - namespaceSelector: {}
    ports:
    - protocol: TCP
      port: 443
    - protocol: TCP
      port: 5432
```

### 认证与授权

```bash
# 1. 创建 RBAC (Role-Based Access Control)
kubectl apply -f - << 'EOF'
apiVersion: v1
kind: ServiceAccount
metadata:
  name: edge-compute-sa
  namespace: edge-compute
---
apiVersion: rbac.authorization.k8s.io/v1
kind: Role
metadata:
  name: edge-compute-role
  namespace: edge-compute
rules:
- apiGroups: [""]
  resources: ["configmaps"]
  verbs: ["get", "list", "watch"]
---
apiVersion: rbac.authorization.k8s.io/v1
kind: RoleBinding
metadata:
  name: edge-compute-rolebinding
  namespace: edge-compute
subjects:
- kind: ServiceAccount
  name: edge-compute-sa
  namespace: edge-compute
roleRef:
  kind: Role
  name: edge-compute-role
  apiGroup: rbac.authorization.k8s.io
EOF

# 2. 启用 Pod Security Policy
kubectl apply -f - << 'EOF'
apiVersion: policy/v1beta1
kind: PodSecurityPolicy
metadata:
  name: restricted
spec:
  privileged: false
  allowPrivilegeEscalation: false
  requiredDropCapabilities:
    - ALL
  volumes:
    - 'configMap'
    - 'emptyDir'
    - 'projected'
    - 'secret'
    - 'downwardAPI'
    - 'persistentVolumeClaim'
  runAsUser:
    rule: 'MustRunAsNonRoot'
  seLinux:
    rule: 'MustRunAs'
    seLinuxOptions:
      level: "s0:c123,c456"
EOF
```

### 数据加密

```bash
# 1. 启用 etcd 加密
--encryption-provider-config=/etc/kubernetes/encryption-config.yaml

# 2. 配置 TLS
tls:
  enabled: true
  secretName: edge-compute-tls

# 3. Secret 加密
kubectl create secret generic db-credentials \
  --from-literal=password=$(openssl rand -base64 32) \
  -n edge-compute
```

---

## 常用命令速查

```bash
# ===== 集群管理 =====
kubectl cluster-info                    # 查看集群信息
kubectl get nodes                       # 列出所有节点
kubectl describe node <NODE>            # 节点详情
kubectl top nodes                       # 节点资源使用

# ===== Deployment =====
kubectl apply -f deployment.yaml        # 创建/更新部署
kubectl get deployments                 # 列出部署
kubectl describe deployment <NAME>      # 部署详情
kubectl set image deployment/<NAME> \
  <CONTAINER>=<IMAGE>                   # 更新镜像
kubectl rollout status deployment/<NAME> # 发布状态
kubectl rollout history deployment/<NAME> # 发布历史
kubectl rollout undo deployment/<NAME>  # 回滚部署

# ===== Pod 管理 =====
kubectl get pods                        # 列出 Pod
kubectl describe pod <POD>              # Pod 详情
kubectl logs <POD>                      # 查看日志
kubectl logs <POD> --tail=100           # 最后100行
kubectl logs <POD> -f                   # 实时日志
kubectl exec -it <POD> -- /bin/bash    # 进入 Pod
kubectl port-forward <POD> 3000:3000   # 端口转发

# ===== 服务与网络 =====
kubectl get svc                         # 列出服务
kubectl describe svc <SERVICE>          # 服务详情
kubectl get ingress                     # 列出入站规则
kubectl get networkpolicies             # 网络策略

# ===== 存储 =====
kubectl get pvc                         # PVC 列表
kubectl get pv                          # PV 列表
kubectl describe pvc <PVC>              # PVC 详情

# ===== Helm =====
helm list                               # 列出发布
helm status <RELEASE>                   # 发布状态
helm upgrade <RELEASE> <CHART>          # 升级发布
helm rollback <RELEASE>                 # 回滚发布
helm uninstall <RELEASE>                # 删除发布

# ===== 监控和调试 =====
kubectl top pods                        # Pod 资源使用
kubectl top nodes                       # 节点资源使用
kubectl events                          # 集群事件
kubectl api-resources                   # API 资源列表
kubectl explain <RESOURCE>              # 资源说明
```

---

## 参考资源

### 官方文档
- [Kubernetes 官方文档](https://kubernetes.io/docs/)
- [Docker 官方文档](https://docs.docker.com/)
- [Helm 官方文档](https://helm.sh/docs/)
- [Prometheus 官方文档](https://prometheus.io/docs/)

### 最佳实践
- [Kubernetes 最佳实践](https://kubernetes.io/docs/concepts/configuration/overview/)
- [Docker 最佳实践](https://docs.docker.com/develop/dev-best-practices/)
- [云原生应用开发指南](https://www.cncf.io/)

### 故障排查
- [Kubernetes 故障排查](https://kubernetes.io/docs/tasks/debug-application-cluster/)
- [Docker 故障排查](https://docs.docker.com/config/containers/logging/)

---

## 总结

### 三种部署方式对比

| 特性 | Docker Compose | Kubernetes | 云服务 |
|-----|--------|------------|--------|
| 学习曲线 | ⭐ 简单 | ⭐⭐⭐ 陡 | ⭐⭐ 中 |
| 扩展性 | 单机 | ⭐⭐⭐ 很强 | ⭐⭐⭐ 自动 |
| 高可用 | ❌ 否 | ✅ 是 | ✅ 是 |
| 自动扩展 | ❌ 否 | ✅ 是 | ✅ 是 |
| 运维成本 | 💰 低 | 💰💰 中 | 💰💰💰 高 |
| 部署时间 | ⏱️ 5分钟 | ⏱️ 15分钟 | ⏱️ 5分钟 |

### 选择建议

- **开发/测试**: Docker Compose
- **小规模生产**: Docker Compose + 监控
- **大规模生产**: Kubernetes + Helm + 监控栈
- **云环境**: EKS / AKS / GKE（托管Kubernetes）

---

**🎉 现在您已经掌握了如何部署一个完整的计算集群！祝您的边缘计算系统运行顺利！**
