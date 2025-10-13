#!/bin/bash

# Algorithm Plugins Framework 构建脚本
# 支持Windows (MSVC) 和 Linux (GCC) 平台

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

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

# 检查依赖
check_dependencies() {
    print_info "检查构建依赖..."
    
    # 检查CMake
    if ! command -v cmake &> /dev/null; then
        print_error "CMake 未安装，请先安装 CMake 3.16 或更高版本"
        exit 1
    fi
    
    # 检查编译器
    if [[ "$OSTYPE" == "msys" || "$OSTYPE" == "win32" ]]; then
        if ! command -v cl &> /dev/null; then
            print_error "MSVC 编译器未找到，请安装 Visual Studio 2019 或更高版本"
            exit 1
        fi
    else
        if ! command -v g++ &> /dev/null; then
            print_error "GCC 编译器未找到，请安装 GCC 7 或更高版本"
            exit 1
        fi
    fi
    
    print_success "依赖检查完成"
}

# 创建构建目录
create_build_dir() {
    print_info "创建构建目录..."
    
    if [ -d "build" ]; then
        print_warning "构建目录已存在，将清理现有内容"
        rm -rf build
    fi
    
    mkdir -p build
    cd build
    
    print_success "构建目录创建完成"
}

# 配置CMake
configure_cmake() {
    print_info "配置CMake..."
    
    local cmake_args=""
    
    # 根据平台设置不同的配置
    if [[ "$OSTYPE" == "msys" || "$OSTYPE" == "win32" ]]; then
        # Windows平台
        cmake_args="-G \"Visual Studio 16 2019\" -A x64"
        print_info "使用 Visual Studio 2019 生成器"
    else
        # Linux平台
        cmake_args="-DCMAKE_BUILD_TYPE=Release"
        print_info "使用 Release 构建类型"
    fi
    
    # 设置安装前缀
    cmake_args="$cmake_args -DCMAKE_INSTALL_PREFIX=../install"
    
    # 执行CMake配置
    eval "cmake .. $cmake_args"
    
    if [ $? -eq 0 ]; then
        print_success "CMake配置完成"
    else
        print_error "CMake配置失败"
        exit 1
    fi
}

# 编译项目
build_project() {
    print_info "开始编译项目..."
    
    # 获取CPU核心数
    local cores=$(nproc 2>/dev/null || echo 4)
    
    if [[ "$OSTYPE" == "msys" || "$OSTYPE" == "win32" ]]; then
        # Windows平台使用MSBuild
        cmake --build . --config Release --parallel $cores
    else
        # Linux平台使用make
        make -j$cores
    fi
    
    if [ $? -eq 0 ]; then
        print_success "编译完成"
    else
        print_error "编译失败"
        exit 1
    fi
}

# 运行测试
run_tests() {
    print_info "运行测试..."
    
    if [ -f "plugin_tests" ]; then
        ./plugin_tests
        if [ $? -eq 0 ]; then
            print_success "测试通过"
        else
            print_warning "部分测试失败"
        fi
    else
        print_warning "测试程序未找到，跳过测试"
    fi
}

# 安装项目
install_project() {
    print_info "安装项目..."
    
    if [[ "$OSTYPE" == "msys" || "$OSTYPE" == "win32" ]]; then
        cmake --install . --config Release
    else
        make install
    fi
    
    if [ $? -eq 0 ]; then
        print_success "安装完成"
    else
        print_error "安装失败"
        exit 1
    fi
}

# 创建发布包
create_package() {
    print_info "创建发布包..."
    
    if command -v cpack &> /dev/null; then
        cpack
        if [ $? -eq 0 ]; then
            print_success "发布包创建完成"
        else
            print_warning "发布包创建失败"
        fi
    else
        print_warning "CPack 未找到，跳过打包"
    fi
}

# 清理函数
cleanup() {
    print_info "清理临时文件..."
    cd ..
    # 这里可以添加清理逻辑
    print_success "清理完成"
}

# 显示帮助信息
show_help() {
    echo "Algorithm Plugins Framework 构建脚本"
    echo ""
    echo "用法: $0 [选项]"
    echo ""
    echo "选项:"
    echo "  -h, --help     显示此帮助信息"
    echo "  -c, --clean    清理构建目录"
    echo "  -t, --test     仅运行测试"
    echo "  -i, --install  仅安装项目"
    echo "  -p, --package  创建发布包"
    echo "  --debug        使用Debug构建类型"
    echo "  --release      使用Release构建类型（默认）"
    echo ""
    echo "示例:"
    echo "  $0                    # 完整构建流程"
    echo "  $0 --clean           # 清理构建目录"
    echo "  $0 --test            # 仅运行测试"
    echo "  $0 --debug           # Debug构建"
}

# 主函数
main() {
    local build_type="Release"
    local clean_only=false
    local test_only=false
    local install_only=false
    local package_only=false
    
    # 解析命令行参数
    while [[ $# -gt 0 ]]; do
        case $1 in
            -h|--help)
                show_help
                exit 0
                ;;
            -c|--clean)
                clean_only=true
                shift
                ;;
            -t|--test)
                test_only=true
                shift
                ;;
            -i|--install)
                install_only=true
                shift
                ;;
            -p|--package)
                package_only=true
                shift
                ;;
            --debug)
                build_type="Debug"
                shift
                ;;
            --release)
                build_type="Release"
                shift
                ;;
            *)
                print_error "未知参数: $1"
                show_help
                exit 1
                ;;
        esac
    done
    
    print_info "Algorithm Plugins Framework 构建开始"
    print_info "构建类型: $build_type"
    
    # 清理模式
    if [ "$clean_only" = true ]; then
        if [ -d "build" ]; then
            rm -rf build
            print_success "构建目录已清理"
        fi
        if [ -d "install" ]; then
            rm -rf install
            print_success "安装目录已清理"
        fi
        exit 0
    fi
    
    # 检查依赖
    check_dependencies
    
    # 创建构建目录
    create_build_dir
    
    # 配置CMake
    configure_cmake
    
    # 仅测试模式
    if [ "$test_only" = true ]; then
        run_tests
        exit 0
    fi
    
    # 编译项目
    build_project
    
    # 运行测试
    run_tests
    
    # 仅安装模式
    if [ "$install_only" = true ]; then
        install_project
        exit 0
    fi
    
    # 安装项目
    install_project
    
    # 仅打包模式
    if [ "$package_only" = true ]; then
        create_package
        exit 0
    fi
    
    # 创建发布包
    create_package
    
    print_success "构建流程完成！"
    print_info "安装目录: $(pwd)/../install"
    print_info "库文件: $(pwd)/../install/lib"
    print_info "头文件: $(pwd)/../install/include"
}

# 设置错误处理
trap cleanup EXIT

# 执行主函数
main "$@"
