#!/bin/bash

# 矩阵乘法算法插件构建脚本
# 生产级构建脚本，支持多种配置选项

set -e  # 遇到错误立即退出

# 脚本配置
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_NAME="matrix_multiplication_plugin"
BUILD_TYPE="${BUILD_TYPE:-Release}"

# 边缘端优化：默认禁用复杂功能以减少依赖和内存使用
ENABLE_TESTS="${ENABLE_TESTS:-OFF}"
ENABLE_BENCHMARKS="${ENABLE_BENCHMARKS:-OFF}"
ENABLE_OPENBLAS="${ENABLE_OPENBLAS:-OFF}"  # 边缘端禁用OpenBLAS
ENABLE_EIGEN="${ENABLE_EIGEN:-OFF}"        # 边缘端禁用Eigen
ENABLE_COVERAGE="${ENABLE_COVERAGE:-OFF}"

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 日志函数
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

# 检查依赖
check_dependencies() {
    log_info "检查构建依赖..."

    local missing_deps=()

    # 检查必需的工具
    command -v cmake >/dev/null 2>&1 || missing_deps+=("cmake")
    command -v make >/dev/null 2>&1 || missing_deps+=("make")
    command -v g++ >/dev/null 2>&1 || missing_deps+=("g++")

    # 检查可选依赖
    if [ "$ENABLE_OPENBLAS" = "ON" ]; then
        pkg-config --exists openblas >/dev/null 2>&1 || {
            log_warn "OpenBLAS not found, disabling OpenBLAS support"
            ENABLE_OPENBLAS="OFF"
        }
    fi

    if [ "$ENABLE_EIGEN" = "ON" ]; then
        pkg-config --exists eigen3 >/dev/null 2>&1 || {
            command -v find >/dev/null 2>&1 && find /usr -name "Eigen" -type d 2>/dev/null | grep -q Eigen || {
                log_warn "Eigen not found, disabling Eigen support"
                ENABLE_EIGEN="OFF"
            }
        }
    fi

    if [ ${#missing_deps[@]} -ne 0 ]; then
        log_error "缺少必需的依赖: ${missing_deps[*]}"
        log_info "请安装缺失的依赖:"
        log_info "  Ubuntu/Debian: sudo apt-get install build-essential cmake"
        exit 1
    fi

    log_success "依赖检查完成"
}

# 创建构建目录
setup_build_directory() {
    log_info "设置构建目录..."

    BUILD_DIR="$SCRIPT_DIR/build"
    INSTALL_DIR="$SCRIPT_DIR/install"

    # 清理旧的构建目录
    if [ -d "$BUILD_DIR" ]; then
        log_info "清理旧的构建目录..."
        rm -rf "$BUILD_DIR"
    fi

    mkdir -p "$BUILD_DIR"
    mkdir -p "$INSTALL_DIR"

    log_success "构建目录设置完成"
}

# 配置CMake
configure_cmake() {
    log_info "配置CMake..."

    cd "$BUILD_DIR"

    local cmake_args=(
        -DCMAKE_BUILD_TYPE="$BUILD_TYPE"
        -DCMAKE_INSTALL_PREFIX="$INSTALL_DIR"
        -DUSE_OPENBLAS="$ENABLE_OPENBLAS"
        -DUSE_EIGEN="$ENABLE_EIGEN"
        -DBUILD_TESTS="$ENABLE_TESTS"
        -DBUILD_BENCHMARKS="$ENABLE_BENCHMARKS"
    )

    if [ "$ENABLE_COVERAGE" = "ON" ]; then
        cmake_args+=(-DCODE_COVERAGE=ON)
    fi

    log_info "CMake参数: ${cmake_args[*]}"

    cmake "${cmake_args[@]}" "$SCRIPT_DIR"

    if [ $? -ne 0 ]; then
        log_error "CMake配置失败"
        exit 1
    fi

    log_success "CMake配置完成"
}

# 构建项目
build_project() {
    log_info "构建项目..."

    cd "$BUILD_DIR"

    # 边缘端优化：限制并行构建数量
    local cpu_count=$(nproc)
    local make_jobs=$((cpu_count > 4 ? 4 : cpu_count))  # 最多4个并行任务
    local make_args=(-j$make_jobs)

    if [ "$ENABLE_COVERAGE" = "ON" ]; then
        make_args+=("coverage")
    fi

    log_info "Make参数: ${make_args[*]}"

    make "${make_args[@]}"

    if [ $? -ne 0 ]; then
        log_error "构建失败"
        exit 1
    fi

    log_success "项目构建完成"
}

# 安装项目
install_project() {
    log_info "安装项目..."

    cd "$BUILD_DIR"

    make install

    if [ $? -ne 0 ]; then
        log_error "安装失败"
        exit 1
    fi

    log_success "项目安装完成"
}

# 运行测试
run_tests() {
    if [ "$ENABLE_TESTS" = "ON" ]; then
        log_info "运行测试..."

        cd "$BUILD_DIR"

        ctest --output-on-failure

        if [ $? -ne 0 ]; then
            log_error "测试失败"
            exit 1
        fi

        log_success "测试通过"
    fi
}

# 注意：本项目使用纯Youki容器运行时，不依赖Docker
# 如需容器化部署，请直接使用Youki命令行工具或API

# 生成构建报告
generate_report() {
    log_info "生成构建报告..."

    local report_file="$SCRIPT_DIR/build_report.txt"

    {
        echo "========================================"
        echo "  矩阵乘法插件构建报告"
        echo "========================================"
        echo ""
        echo "构建时间: $(date)"
        echo "构建类型: $BUILD_TYPE"
        echo "OpenBLAS支持: $ENABLE_OPENBLAS"
        echo "Eigen支持: $ENABLE_EIGEN"
        echo "测试: $ENABLE_TESTS"
        echo "基准测试: $ENABLE_BENCHMARKS"
        echo "代码覆盖率: $ENABLE_COVERAGE"
        echo ""
        echo "构建目录: $BUILD_DIR"
        echo "安装目录: $INSTALL_DIR"
        echo ""

        if [ -f "$INSTALL_DIR/bin/matrix_multiplication" ]; then
            echo "可执行文件大小: $(du -h "$INSTALL_DIR/bin/matrix_multiplication" | cut -f1)"
            echo "可执行文件权限: $(ls -l "$INSTALL_DIR/bin/matrix_multiplication")"
        fi

        echo ""
        echo "构建完成 ✓"
        echo "========================================"
    } > "$report_file"

    log_success "构建报告已生成: $report_file"
}

# 显示帮助信息
show_help() {
    cat << EOF
矩阵乘法算法插件构建脚本

用法: $0 [选项]

选项:
    -h, --help              显示帮助信息
    -t, --build-type TYPE   构建类型 (Debug/Release) [默认: $BUILD_TYPE]
    --enable-tests          启用单元测试
    --enable-benchmarks     启用性能基准测试
    --disable-openblas      禁用OpenBLAS支持
    --disable-eigen         禁用Eigen支持
    --enable-coverage       启用代码覆盖率
    --docker-only           仅构建Docker镜像
    --clean                 清理构建文件

示例:
    $0                          # 标准构建
    $0 --build-type Debug       # 调试构建
    $0 --enable-tests           # 带测试的构建
    $0 --docker-only            # 仅构建Docker镜像

EOF
}

# 清理构建文件
clean_build() {
    log_info "清理构建文件..."

    if [ -d "$SCRIPT_DIR/build" ]; then
        rm -rf "$SCRIPT_DIR/build"
        log_info "已删除构建目录"
    fi

    if [ -d "$SCRIPT_DIR/install" ]; then
        rm -rf "$SCRIPT_DIR/install"
        log_info "已删除安装目录"
    fi

    log_success "清理完成"
}

# 主函数
main() {
    # 解析命令行参数
    while [[ $# -gt 0 ]]; do
        case $1 in
            -h|--help)
                show_help
                exit 0
                ;;
            -t|--build-type)
                BUILD_TYPE="$2"
                shift 2
                ;;
            --enable-tests)
                ENABLE_TESTS="ON"
                shift
                ;;
            --enable-benchmarks)
                ENABLE_BENCHMARKS="ON"
                shift
                ;;
            --disable-openblas)
                ENABLE_OPENBLAS="OFF"
                shift
                ;;
            --disable-eigen)
                ENABLE_EIGEN="OFF"
                shift
                ;;
            --enable-coverage)
                ENABLE_COVERAGE="ON"
                shift
                ;;
            --docker-only)
                DOCKER_ONLY=true
                shift
                ;;
            --clean)
                clean_build
                exit 0
                ;;
            *)
                log_error "未知选项: $1"
                show_help
                exit 1
                ;;
        esac
    done

    log_info "开始构建 $PROJECT_NAME..."
    log_info "构建类型: $BUILD_TYPE"
    log_info "启用测试: $ENABLE_TESTS"
    log_info "启用基准测试: $ENABLE_BENCHMARKS"
    log_info "OpenBLAS支持: $ENABLE_OPENBLAS"
    log_info "Eigen支持: $ENABLE_EIGEN"
    log_info "代码覆盖率: $ENABLE_COVERAGE"

    # 如果只是构建Docker镜像
    if [ "${DOCKER_ONLY:-false}" = true ]; then
        build_docker_image
        exit 0
    fi

    # 执行构建流程（纯Youki，无Docker）
    check_dependencies
    setup_build_directory
    configure_cmake
    build_project
    install_project
    run_tests
    generate_report

    log_success "🎉 $PROJECT_NAME 构建完成！"

    echo ""
    echo "========================================"
    echo "构建结果:"
    echo "  可执行文件: $INSTALL_DIR/bin/matrix_multiplication"
    echo "  Docker镜像: matrix-multiplication-plugin:1.0.0"
    echo "  构建报告: $SCRIPT_DIR/build_report.txt"
    echo "========================================"

    # 显示使用方法
    echo ""
    echo "使用方法:"
    echo "  # 本地运行"
    echo "  $INSTALL_DIR/bin/matrix_multiplication --help"
    echo ""
    echo "  # Docker运行"
    echo "  docker run --rm matrix-multiplication-plugin:1.0.0 --help"
    echo ""
}

# 执行主函数
main "$@"
