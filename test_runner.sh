#!/bin/bash

# Rust Edge Compute Framework - 测试运行器
# 用于运行端到端集成测试

set -e

echo "=========================================="
echo "Rust Edge Compute Framework - Test Runner"
echo "=========================================="

# 检查Rust是否安装
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

# 启动服务器进行集成测试
echo ""
echo "🚀 Starting test server..."
cargo build --release

# 在后台启动服务器
./target/release/rust-edge-compute &
SERVER_PID=$!

# 等待服务器启动
echo "⏳ Waiting for server to start..."
sleep 3

# 检查服务器是否在运行
if kill -0 $SERVER_PID 2>/dev/null; then
    echo "✓ Server started successfully (PID: $SERVER_PID)"

    # 运行集成测试
    echo ""
    echo "🔗 Running integration tests..."
    if cargo test --test integration_test; then
        echo "✓ Integration tests passed"
    else
        echo "❌ Integration tests failed"
    fi

    # 停止服务器
    echo ""
    echo "🛑 Stopping test server..."
    kill $SERVER_PID
    wait $SERVER_PID 2>/dev/null
    echo "✓ Server stopped"
else
    echo "❌ Server failed to start"
    exit 1
fi

echo ""
echo "=========================================="
echo "🎉 All tests completed!"
echo "=========================================="

# 显示测试覆盖率（如果安装了工具）
if command -v grcov &> /dev/null; then
    echo ""
    echo "📊 Generating test coverage report..."
    cargo test --lib -- --test-threads=1
    grcov . --binary-path ./target/debug/ -s . -t html --branch --ignore-not-existing -o ./target/coverage/
    echo "✓ Coverage report generated: ./target/coverage/index.html"
fi

echo ""
echo "📝 Next steps:"
echo "1. Review test results above"
echo "2. Check server logs for any issues"
echo "3. Run 'cargo doc --open' to view API documentation"
echo "4. Consider adding more test cases for edge cases"
