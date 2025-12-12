#!/bin/bash

################################################################################
# Rust Edge Compute Framework - 集群部署助手脚本
# 支持: Docker Compose, Kubernetes, Helm
################################################################################

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 日志函数
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

# 显示帮助信息
show_help() {
    cat << EOF
使用方法: $0 [命令] [选项]

命令:
  docker-compose      Docker Compose 相关操作
  kubernetes          Kubernetes 集群操作
  helm                Helm 部署操作
  monitor             监控相关操作
  status              查看部署状态
  logs                查看日志
  help                显示此帮助信息

Docker Compose 命令:
  docker-compose up         启动服务
  docker-compose down       停止服务
  docker-compose logs       查看日志
  docker-compose build      构建镜像

Kubernetes 命令:
  kubernetes init          初始化 K8s 集群
  kubernetes deploy        部署应用
  kubernetes scale <NUM>   扩展副本数
  kubernetes status        查看部署状态
  kubernetes logs          查看日志

Helm 命令:
  helm install <NAME>      使用 Helm 安装
  helm upgrade <NAME>      升级 Helm 发布
  helm uninstall <NAME>    卸载 Helm 发布
  helm status <NAME>       查看 Helm 状态

示例:
  $0 docker-compose up
  $0 kubernetes deploy
  $0 helm install my-release
  $0 status
  $0 logs -f

EOF
}

# ============================================================================
# Docker Compose 操作
# ============================================================================

docker_compose_up() {
    log_info "启动 Docker Compose 服务..."
    
    if [ ! -f "docker/docker-compose.yml" ]; then
        log_error "找不到 docker/docker-compose.yml 文件"
        exit 1
    fi
    
    # 创建必要的目录
    mkdir -p docker/{data,logs,certs,config}
    
    # 启动服务
    docker-compose -f docker/docker-compose.yml up -d
    
    log_success "Docker Compose 启动成功"
    log_info "等待服务就绪..."
    sleep 5
    
    # 验证服务
    if curl -s http://localhost:3000/api/v1/health > /dev/null; then
        log_success "应用健康检查通过"
        echo ""
        log_info "访问地址:"
        echo "  应用:       http://localhost:3000"
        echo "  Grafana:   http://localhost:3001 (admin/admin)"
        echo "  Prometheus: http://localhost:9090"
    else
        log_warning "应用健康检查失败，请检查日志"
        docker-compose -f docker/docker-compose.yml logs rust-edge-compute
    fi
}

docker_compose_down() {
    log_info "停止 Docker Compose 服务..."
    docker-compose -f docker/docker-compose.yml down
    log_success "Docker Compose 服务已停止"
}

docker_compose_logs() {
    log_info "查看 Docker Compose 日志..."
    docker-compose -f docker/docker-compose.yml logs -f "${@:2}"
}

docker_compose_build() {
    log_info "构建 Docker 镜像..."
    docker-compose -f docker/docker-compose.yml build
    log_success "Docker 镜像构建完成"
}

# ============================================================================
# Kubernetes 操作
# ============================================================================

kubernetes_init() {
    log_info "初始化 Kubernetes 集群..."
    
    # 检查 kubectl
    if ! command -v kubectl &> /dev/null; then
        log_error "kubectl 未安装，请先安装 kubectl"
        exit 1
    fi
    
    # 创建命名空间
    log_info "创建命名空间: edge-compute"
    kubectl create namespace edge-compute --dry-run=client -o yaml | kubectl apply -f -
    
    log_success "Kubernetes 集群初始化成功"
    log_info "集群信息:"
    kubectl cluster-info
}

kubernetes_deploy() {
    log_info "部署应用到 Kubernetes..."
    
    if [ ! -f "k8s/deployment.yaml" ]; then
        log_error "找不到 k8s/deployment.yaml 文件"
        exit 1
    fi
    
    # 确保命名空间存在
    kubectl create namespace edge-compute --dry-run=client -o yaml | kubectl apply -f -
    
    # 部署应用
    kubectl apply -f k8s/deployment.yaml -n edge-compute
    
    log_success "应用已部署"
    log_info "等待部署就绪..."
    kubectl rollout status deployment/rust-edge-compute -n edge-compute --timeout=5m
    
    log_success "应用已就绪"
}

kubernetes_scale() {
    local replicas=$2
    
    if [ -z "$replicas" ]; then
        log_error "请指定副本数: $0 kubernetes scale <NUM>"
        exit 1
    fi
    
    log_info "扩展副本数到 $replicas..."
    kubectl scale deployment rust-edge-compute --replicas=$replicas -n edge-compute
    
    log_success "副本数已更新"
    kubectl get deployment rust-edge-compute -n edge-compute
}

kubernetes_status() {
    log_info "Kubernetes 部署状态:"
    echo ""
    
    log_info "部署信息:"
    kubectl get deployment -n edge-compute
    
    echo ""
    log_info "Pod 信息:"
    kubectl get pods -n edge-compute -o wide
    
    echo ""
    log_info "服务信息:"
    kubectl get svc -n edge-compute
    
    echo ""
    log_info "资源使用:"
    kubectl top pods -n edge-compute 2>/dev/null || log_warning "Metrics Server 未安装"
}

kubernetes_logs() {
    log_info "查看 Kubernetes 日志..."
    kubectl logs -f deployment/rust-edge-compute -n edge-compute "${@:2}"
}

# ============================================================================
# Helm 操作
# ============================================================================

helm_install() {
    local release_name=$2
    
    if [ -z "$release_name" ]; then
        release_name="rust-edge-compute"
    fi
    
    log_info "使用 Helm 安装发布: $release_name..."
    
    if ! command -v helm &> /dev/null; then
        log_error "Helm 未安装，请先安装 Helm"
        exit 1
    fi
    
    # 检查 Helm chart
    if [ ! -f "helm/Chart.yaml" ]; then
        log_error "找不到 helm/Chart.yaml 文件"
        exit 1
    fi
    
    # 创建命名空间
    kubectl create namespace edge-compute --dry-run=client -o yaml | kubectl apply -f -
    
    # 安装
    helm install $release_name ./helm \
        --namespace edge-compute \
        --values helm/values.yaml
    
    log_success "Helm 发布已安装"
    helm list -n edge-compute
}

helm_upgrade() {
    local release_name=$2
    
    if [ -z "$release_name" ]; then
        release_name="rust-edge-compute"
    fi
    
    log_info "升级 Helm 发布: $release_name..."
    
    helm upgrade $release_name ./helm \
        --namespace edge-compute \
        --values helm/values.yaml
    
    log_success "Helm 发布已升级"
    helm list -n edge-compute
}

helm_uninstall() {
    local release_name=$2
    
    if [ -z "$release_name" ]; then
        release_name="rust-edge-compute"
    fi
    
    log_warning "准备卸载 Helm 发布: $release_name"
    read -p "确认卸载？(y/n) " -n 1 -r
    echo
    
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        helm uninstall $release_name -n edge-compute
        log_success "Helm 发布已卸载"
    else
        log_info "卸载已取消"
    fi
}

helm_status() {
    local release_name=$2
    
    if [ -z "$release_name" ]; then
        release_name="rust-edge-compute"
    fi
    
    log_info "Helm 发布状态:"
    helm status $release_name -n edge-compute
}

# ============================================================================
# 监控操作
# ============================================================================

setup_monitoring() {
    log_info "设置监控栈..."
    
    if ! command -v helm &> /dev/null; then
        log_error "需要安装 Helm"
        exit 1
    fi
    
    # 添加仓库
    helm repo add prometheus-community https://prometheus-community.github.io/helm-charts
    helm repo add grafana https://grafana.github.io/helm-charts
    helm repo update
    
    # 创建命名空间
    kubectl create namespace monitoring --dry-run=client -o yaml | kubectl apply -f -
    
    # 安装 Prometheus 栈
    log_info "安装 Prometheus 和 Grafana..."
    helm install prometheus prometheus-community/kube-prometheus-stack \
        --namespace monitoring \
        --set prometheus.prometheusSpec.retention=15d \
        --set grafana.adminPassword=YourSecurePassword
    
    log_success "监控栈已安装"
    log_info "访问 Grafana:"
    log_info "  kubectl port-forward -n monitoring svc/prometheus-grafana 3000:80"
    log_info "  访问: http://localhost:3000"
}

# ============================================================================
# 状态检查
# ============================================================================

check_status() {
    log_info "检查部署状态..."
    echo ""
    
    # Docker Compose
    if [ -f "docker/docker-compose.yml" ]; then
        if docker-compose -f docker/docker-compose.yml ps | grep -q "Up"; then
            log_success "Docker Compose: 运行中"
            docker-compose -f docker/docker-compose.yml ps | tail -n +2 | awk '{print "  " $0}'
        else
            log_warning "Docker Compose: 未运行"
        fi
        echo ""
    fi
    
    # Kubernetes
    if command -v kubectl &> /dev/null; then
        if kubectl get namespace edge-compute 2>/dev/null; then
            log_success "Kubernetes: 集群可用"
            echo "  部署:"
            kubectl get deployment -n edge-compute 2>/dev/null | tail -n +2 | awk '{print "    " $0}' || true
            echo "  Pod:"
            kubectl get pods -n edge-compute 2>/dev/null | tail -n +2 | awk '{print "    " $0}' || true
        else
            log_warning "Kubernetes: 集群不可用或未初始化"
        fi
        echo ""
    fi
    
    # Helm
    if command -v helm &> /dev/null; then
        if helm list -n edge-compute 2>/dev/null | grep -q "rust-edge-compute"; then
            log_success "Helm: 发布已安装"
            helm list -n edge-compute 2>/dev/null | tail -n +2 | awk '{print "  " $0}' || true
        else
            log_warning "Helm: 无已安装的发布"
        fi
    fi
}

# ============================================================================
# 主函数
# ============================================================================

main() {
    if [ $# -eq 0 ]; then
        show_help
        exit 0
    fi
    
    local command=$1
    
    case "$command" in
        # Docker Compose
        docker-compose)
            case "$2" in
                up)
                    docker_compose_up
                    ;;
                down)
                    docker_compose_down
                    ;;
                logs)
                    docker_compose_logs "$@"
                    ;;
                build)
                    docker_compose_build
                    ;;
                *)
                    log_error "未知的 Docker Compose 命令: $2"
                    show_help
                    exit 1
                    ;;
            esac
            ;;
        
        # Kubernetes
        kubernetes)
            case "$2" in
                init)
                    kubernetes_init
                    ;;
                deploy)
                    kubernetes_deploy
                    ;;
                scale)
                    kubernetes_scale "$@"
                    ;;
                status)
                    kubernetes_status
                    ;;
                logs)
                    kubernetes_logs "$@"
                    ;;
                *)
                    log_error "未知的 Kubernetes 命令: $2"
                    show_help
                    exit 1
                    ;;
            esac
            ;;
        
        # Helm
        helm)
            case "$2" in
                install)
                    helm_install "$@"
                    ;;
                upgrade)
                    helm_upgrade "$@"
                    ;;
                uninstall)
                    helm_uninstall "$@"
                    ;;
                status)
                    helm_status "$@"
                    ;;
                *)
                    log_error "未知的 Helm 命令: $2"
                    show_help
                    exit 1
                    ;;
            esac
            ;;
        
        # 监控
        monitor)
            case "$2" in
                setup|install)
                    setup_monitoring
                    ;;
                *)
                    log_error "未知的监控命令: $2"
                    show_help
                    exit 1
                    ;;
            esac
            ;;
        
        # 状态
        status)
            check_status
            ;;
        
        # 日志
        logs)
            case "$2" in
                docker)
                    docker_compose_logs "$@"
                    ;;
                kubernetes)
                    kubernetes_logs "$@"
                    ;;
                *)
                    log_error "请指定日志来源: docker 或 kubernetes"
                    exit 1
                    ;;
            esac
            ;;
        
        # 帮助
        help)
            show_help
            ;;
        
        *)
            log_error "未知的命令: $command"
            show_help
            exit 1
            ;;
    esac
}

# 运行主函数
main "$@"
