#!/bin/bash

# Rust Edge Compute Framework - 编译测试脚本
# 用于验证项目是否能够成功编译

set -e

echo "=========================================="
echo "Rust Edge Compute Framework - Build Test"
echo "=========================================="

# 检查Rust工具链
if ! command -v cargo &> /dev/null; then
    echo "❌ Cargo not found. Please install Rust first."
    echo "Visit: https://rustup.rs/"
    exit 1
fi

echo "✓ Rust/Cargo detected"

# 检查项目结构
if [ ! -f "Cargo.toml" ]; then
    echo "❌ Cargo.toml not found. Please run this script from the project root."
    exit 1
fi

echo "✓ Project structure verified"

# 清理之前的构建
echo ""
echo "🧹 Cleaning previous build..."
cargo clean

# 检查语法
echo ""
echo "🔍 Checking syntax..."
if cargo check; then
    echo "✓ Syntax check passed"
else
    echo "❌ Syntax check failed"
    exit 1
fi

# 编译项目
echo ""
echo "🔨 Building project..."
if cargo build --release; then
    echo "✓ Build successful"
else
    echo "❌ Build failed"
    exit 1
fi

# 运行单元测试
echo ""
echo "🧪 Running unit tests..."
if cargo test --lib; then
    echo "✓ Unit tests passed"
else
    echo "❌ Unit tests failed"
    exit 1
fi

# 检查二进制文件
echo ""
echo "📦 Checking binary..."
if [ -f "target/release/rust-edge-compute" ]; then
    echo "✓ Binary created successfully"
    ls -la target/release/rust-edge-compute
else
    echo "❌ Binary not found"
    exit 1
fi

echo ""
echo "=========================================="
echo "🎉 ALL TESTS PASSED!"
echo "=========================================="
echo ""
echo "✅ Syntax check: PASSED"
echo "✅ Build: PASSED"
echo "✅ Unit tests: PASSED"
echo "✅ Binary creation: PASSED"
echo ""
echo "🚀 Project is ready for deployment!"
echo ""
echo "Next steps:"
echo "1. Run './target/release/rust-edge-compute' to start the server"
echo "2. Test the API endpoints"
echo "3. Deploy to production environment"
echo ""
echo "=========================================="
