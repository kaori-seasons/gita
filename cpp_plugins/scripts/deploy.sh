#!/bin/bash

# Algorithm Plugins Framework 生产部署脚本
# 支持Windows和Linux平台的生产环境部署

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 配置变量
PROJECT_NAME="AlgorithmPlugins"
VERSION="1.0.0"
BUILD_TYPE="Release"
INSTALL_PREFIX="/opt/algorithm_plugins"
SERVICE_USER="algorithm"
SERVICE_GROUP="algorithm"
LOG_DIR="/var/log/algorithm_plugins"
CONFIG_DIR="/etc/algorithm_plugins"
CACHE_DIR="/var/cache/algorithm_plugins"

# 打印带颜色的消息
print_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# 检查系统要求
check_system_requirements() {
    print_info "检查系统要求..."
    
    # 检查操作系统
    if [[ "$OSTYPE" == "linux-gnu"* ]]; then
        OS="linux"
        print_info "检测到Linux系统"
    elif [[ "$OSTYPE" == "msys" || "$OSTYPE" == "win32" ]]; then
        OS="windows"
        print_info "检测到Windows系统"
    else
        print_error "不支持的操作系统: $OSTYPE"
        exit 1
    fi
    
    # 检查内存
    if [[ "$OS" == "linux" ]]; then
        MEMORY_GB=$(free -g | awk '/^Mem:/{print $2}')
        if [ "$MEMORY_GB" -lt 2 ]; then
            print_warning "系统内存不足2GB，可能影响性能"
        fi
        
        # 检查磁盘空间
        DISK_GB=$(df -BG . | awk 'NR==2{print $4}' | sed 's/G//')
        if [ "$DISK_GB" -lt 5 ]; then
            print_error "磁盘空间不足5GB"
            exit 1
        fi
    fi
    
    print_success "系统要求检查完成"
}

# 创建用户和组
create_user_and_group() {
    if [[ "$OS" == "linux" ]]; then
        print_info "创建用户和组..."
        
        # 创建组
        if ! getent group "$SERVICE_GROUP" > /dev/null 2>&1; then
            groupadd "$SERVICE_GROUP"
            print_info "创建组: $SERVICE_GROUP"
        else
            print_info "组已存在: $SERVICE_GROUP"
        fi
        
        # 创建用户
        if ! getent passwd "$SERVICE_USER" > /dev/null 2>&1; then
            useradd -r -g "$SERVICE_GROUP" -d "$INSTALL_PREFIX" -s /bin/false "$SERVICE_USER"
            print_info "创建用户: $SERVICE_USER"
        else
            print_info "用户已存在: $SERVICE_USER"
        fi
        
        print_success "用户和组创建完成"
    fi
}

# 创建目录结构
create_directories() {
    print_info "创建目录结构..."
    
    # 创建安装目录
    mkdir -p "$INSTALL_PREFIX"/{bin,lib,include,plugins,config,logs,cache}
    
    # 创建配置目录
    mkdir -p "$CONFIG_DIR"
    
    # 创建日志目录
    mkdir -p "$LOG_DIR"
    
    # 创建缓存目录
    mkdir -p "$CACHE_DIR"
    
    # 设置权限
    if [[ "$OS" == "linux" ]]; then
        chown -R "$SERVICE_USER:$SERVICE_GROUP" "$INSTALL_PREFIX"
        chown -R "$SERVICE_USER:$SERVICE_GROUP" "$CONFIG_DIR"
        chown -R "$SERVICE_USER:$SERVICE_GROUP" "$LOG_DIR"
        chown -R "$SERVICE_USER:$SERVICE_GROUP" "$CACHE_DIR"
        
        chmod 755 "$INSTALL_PREFIX"
        chmod 755 "$CONFIG_DIR"
        chmod 755 "$LOG_DIR"
        chmod 755 "$CACHE_DIR"
    fi
    
    print_success "目录结构创建完成"
}

# 构建项目
build_project() {
    print_info "构建项目..."
    
    # 创建构建目录
    mkdir -p build
    cd build
    
    # 配置CMake
    if [[ "$OS" == "linux" ]]; then
        cmake .. \
            -DCMAKE_BUILD_TYPE="$BUILD_TYPE" \
            -DCMAKE_INSTALL_PREFIX="$INSTALL_PREFIX" \
            -DENABLE_PRODUCTION=ON \
            -DENABLE_OPTIMIZATION=ON \
            -DENABLE_SECURITY=ON
    else
        cmake .. \
            -G "Visual Studio 16 2019" -A x64 \
            -DCMAKE_INSTALL_PREFIX="$INSTALL_PREFIX" \
            -DENABLE_PRODUCTION=ON \
            -DENABLE_OPTIMIZATION=ON \
            -DENABLE_SECURITY=ON
    fi
    
    # 编译
    if [[ "$OS" == "linux" ]]; then
        make -j$(nproc)
    else
        cmake --build . --config "$BUILD_TYPE" --parallel
    fi
    
    print_success "项目构建完成"
}

# 安装项目
install_project() {
    print_info "安装项目..."
    
    if [[ "$OS" == "linux" ]]; then
        make install
    else
        cmake --install . --config "$BUILD_TYPE"
    fi
    
    # 复制配置文件
    cp ../config/plugin_config.json "$CONFIG_DIR/"
    
    # 设置权限
    if [[ "$OS" == "linux" ]]; then
        chown -R "$SERVICE_USER:$SERVICE_GROUP" "$INSTALL_PREFIX"
        chown -R "$SERVICE_USER:$SERVICE_GROUP" "$CONFIG_DIR"
        
        chmod 755 "$INSTALL_PREFIX/bin"/*
        chmod 644 "$INSTALL_PREFIX/lib"/*
        chmod 644 "$CONFIG_DIR"/*
    fi
    
    print_success "项目安装完成"
}

# 创建系统服务
create_system_service() {
    if [[ "$OS" == "linux" ]]; then
        print_info "创建系统服务..."
        
        # 创建systemd服务文件
        cat > /etc/systemd/system/algorithm_plugins.service << EOF
[Unit]
Description=Algorithm Plugins Framework Service
After=network.target

[Service]
Type=simple
User=$SERVICE_USER
Group=$SERVICE_GROUP
WorkingDirectory=$INSTALL_PREFIX
ExecStart=$INSTALL_PREFIX/bin/plugin_service
ExecReload=/bin/kill -HUP \$MAINPID
Restart=always
RestartSec=5
StandardOutput=journal
StandardError=journal
SyslogIdentifier=algorithm_plugins

# 安全设置
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=$INSTALL_PREFIX $LOG_DIR $CACHE_DIR

# 资源限制
LimitNOFILE=65536
LimitNPROC=4096

[Install]
WantedBy=multi-user.target
EOF
        
        # 重新加载systemd
        systemctl daemon-reload
        
        # 启用服务
        systemctl enable algorithm_plugins.service
        
        print_success "系统服务创建完成"
    fi
}

# 创建监控脚本
create_monitoring_scripts() {
    print_info "创建监控脚本..."
    
    # 创建健康检查脚本
    cat > "$INSTALL_PREFIX/bin/health_check.sh" << 'EOF'
#!/bin/bash

# 健康检查脚本
LOG_FILE="/var/log/algorithm_plugins/health_check.log"
TIMESTAMP=$(date '+%Y-%m-%d %H:%M:%S')

# 检查服务状态
if systemctl is-active --quiet algorithm_plugins; then
    echo "[$TIMESTAMP] Service is running" >> "$LOG_FILE"
    exit 0
else
    echo "[$TIMESTAMP] Service is not running" >> "$LOG_FILE"
    exit 1
fi
EOF
    
    # 创建性能监控脚本
    cat > "$INSTALL_PREFIX/bin/performance_monitor.sh" << 'EOF'
#!/bin/bash

# 性能监控脚本
LOG_FILE="/var/log/algorithm_plugins/performance.log"
TIMESTAMP=$(date '+%Y-%m-%d %H:%M:%S')

# 获取CPU使用率
CPU_USAGE=$(top -bn1 | grep "Cpu(s)" | awk '{print $2}' | cut -d'%' -f1)

# 获取内存使用率
MEMORY_USAGE=$(free | grep Mem | awk '{printf("%.2f"), $3/$2 * 100.0}')

# 获取磁盘使用率
DISK_USAGE=$(df -h / | awk 'NR==2{print $5}' | cut -d'%' -f1)

# 记录性能数据
echo "[$TIMESTAMP] CPU: ${CPU_USAGE}%, Memory: ${MEMORY_USAGE}%, Disk: ${DISK_USAGE}%" >> "$LOG_FILE"
EOF
    
    # 设置执行权限
    chmod +x "$INSTALL_PREFIX/bin/health_check.sh"
    chmod +x "$INSTALL_PREFIX/bin/performance_monitor.sh"
    
    if [[ "$OS" == "linux" ]]; then
        chown "$SERVICE_USER:$SERVICE_GROUP" "$INSTALL_PREFIX/bin/health_check.sh"
        chown "$SERVICE_USER:$SERVICE_GROUP" "$INSTALL_PREFIX/bin/performance_monitor.sh"
    fi
    
    print_success "监控脚本创建完成"
}

# 配置日志轮转
configure_log_rotation() {
    if [[ "$OS" == "linux" ]]; then
        print_info "配置日志轮转..."
        
        # 创建logrotate配置
        cat > /etc/logrotate.d/algorithm_plugins << EOF
$LOG_DIR/*.log {
    daily
    missingok
    rotate 30
    compress
    delaycompress
    notifempty
    create 644 $SERVICE_USER $SERVICE_GROUP
    postrotate
        systemctl reload algorithm_plugins > /dev/null 2>&1 || true
    endscript
}
EOF
        
        print_success "日志轮转配置完成"
    fi
}

# 创建防火墙规则
configure_firewall() {
    if [[ "$OS" == "linux" ]]; then
        print_info "配置防火墙..."
        
        # 检查防火墙状态
        if systemctl is-active --quiet firewalld; then
            # 添加防火墙规则
            firewall-cmd --permanent --add-port=8080/tcp
            firewall-cmd --permanent --add-port=9090/tcp
            firewall-cmd --reload
            print_info "firewalld规则已添加"
        elif systemctl is-active --quiet ufw; then
            # Ubuntu/Debian防火墙
            ufw allow 8080/tcp
            ufw allow 9090/tcp
            print_info "ufw规则已添加"
        else
            print_warning "未检测到防火墙服务"
        fi
        
        print_success "防火墙配置完成"
    fi
}

# 运行测试
run_tests() {
    print_info "运行生产测试..."
    
    # 运行健康检查
    if [[ -f "$INSTALL_PREFIX/bin/health_check.sh" ]]; then
        "$INSTALL_PREFIX/bin/health_check.sh"
        if [ $? -eq 0 ]; then
            print_success "健康检查通过"
        else
            print_warning "健康检查失败"
        fi
    fi
    
    # 运行性能测试
    if [[ -f "$INSTALL_PREFIX/bin/production_test" ]]; then
        "$INSTALL_PREFIX/bin/production_test"
        if [ $? -eq 0 ]; then
            print_success "性能测试通过"
        else
            print_warning "性能测试失败"
        fi
    fi
    
    print_success "测试完成"
}

# 启动服务
start_service() {
    if [[ "$OS" == "linux" ]]; then
        print_info "启动服务..."
        
        systemctl start algorithm_plugins.service
        
        # 等待服务启动
        sleep 5
        
        if systemctl is-active --quiet algorithm_plugins; then
            print_success "服务启动成功"
        else
            print_error "服务启动失败"
            systemctl status algorithm_plugins.service
            exit 1
        fi
    fi
}

# 显示部署信息
show_deployment_info() {
    print_info "部署信息:"
    echo "  项目名称: $PROJECT_NAME"
    echo "  版本: $VERSION"
    echo "  安装路径: $INSTALL_PREFIX"
    echo "  配置路径: $CONFIG_DIR"
    echo "  日志路径: $LOG_DIR"
    echo "  缓存路径: $CACHE_DIR"
    
    if [[ "$OS" == "linux" ]]; then
        echo "  服务用户: $SERVICE_USER"
        echo "  服务组: $SERVICE_GROUP"
        echo "  服务状态: $(systemctl is-active algorithm_plugins)"
    fi
    
    echo ""
    print_info "常用命令:"
    if [[ "$OS" == "linux" ]]; then
        echo "  启动服务: systemctl start algorithm_plugins"
        echo "  停止服务: systemctl stop algorithm_plugins"
        echo "  重启服务: systemctl restart algorithm_plugins"
        echo "  查看状态: systemctl status algorithm_plugins"
        echo "  查看日志: journalctl -u algorithm_plugins -f"
    fi
    echo "  健康检查: $INSTALL_PREFIX/bin/health_check.sh"
    echo "  性能监控: $INSTALL_PREFIX/bin/performance_monitor.sh"
}

# 主函数
main() {
    print_info "Algorithm Plugins Framework 生产部署开始"
    print_info "版本: $VERSION"
    
    # 检查系统要求
    check_system_requirements
    
    # 创建用户和组
    create_user_and_group
    
    # 创建目录结构
    create_directories
    
    # 构建项目
    build_project
    
    # 安装项目
    install_project
    
    # 创建系统服务
    create_system_service
    
    # 创建监控脚本
    create_monitoring_scripts
    
    # 配置日志轮转
    configure_log_rotation
    
    # 配置防火墙
    configure_firewall
    
    # 运行测试
    run_tests
    
    # 启动服务
    start_service
    
    # 显示部署信息
    show_deployment_info
    
    print_success "生产部署完成！"
}

# 错误处理
trap 'print_error "部署过程中发生错误，退出码: $?"' ERR

# 执行主函数
main "$@"
