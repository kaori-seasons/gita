#!/bin/bash
# Executor 打包脚本
# 用法: ./package-executor.sh <executor-name> <version> <features>

set -e

EXECUTOR_NAME=$1
VERSION=$2
FEATURES=$3

if [ -z "$EXECUTOR_NAME" ] || [ -z "$VERSION" ]; then
    echo "Usage: $0 <executor-name> <version> [features]"
    exit 1
fi

PACKAGE_NAME="rust-edge-compute-${EXECUTOR_NAME}-${VERSION}"
PACKAGE_DIR="dist/${PACKAGE_NAME}"
CRATE_NAME="rust-edge-compute-${EXECUTOR_NAME}"

echo "Packaging ${EXECUTOR_NAME} executor (version: ${VERSION}, features: ${FEATURES:-default})"

# 创建打包目录
mkdir -p "${PACKAGE_DIR}/lib"
mkdir -p "${PACKAGE_DIR}/include"
mkdir -p "${PACKAGE_DIR}/docs"

# 复制库文件
echo "Copying library files..."
find target/release -name "lib${CRATE_NAME}*.so" -o -name "lib${CRATE_NAME}*.dylib" -o -name "lib${CRATE_NAME}*.dll" | while read lib; do
    cp "$lib" "${PACKAGE_DIR}/lib/" || true
done

find target/release -name "lib${CRATE_NAME}*.rlib" | while read rlib; do
    cp "$rlib" "${PACKAGE_DIR}/lib/" || true
done

# 复制特殊文件（根据 executor 类型）
case "$EXECUTOR_NAME" in
    cpp)
        echo "Copying C++ header files..."
        if [ -d "rust-edge-compute-cpp/src/ffi" ]; then
            cp rust-edge-compute-cpp/src/ffi/*.h "${PACKAGE_DIR}/include/" 2>/dev/null || true
        fi
        ;;
    ml)
        echo "Copying ML preprocessing/postprocessing modules..."
        # 可以复制模型文件或其他资源
        if [ -d "rust-edge-compute-ml/models" ]; then
            mkdir -p "${PACKAGE_DIR}/models"
            cp -r rust-edge-compute-ml/models/* "${PACKAGE_DIR}/models/" 2>/dev/null || true
        fi
        ;;
    python)
        echo "Copying Python/WASM modules..."
        # 复制 Python 相关文件（如果启用 python 特性）
        if [ -d "rust-edge-compute-python/src/python" ]; then
            mkdir -p "${PACKAGE_DIR}/python"
            # 可以复制 Python 脚本或配置
        fi
        # 复制 WASM 相关文件（如果启用 wasm 特性）
        if [ -d "rust-edge-compute-python/src/wasm" ]; then
            mkdir -p "${PACKAGE_DIR}/wasm"
            # 可以复制 WASM 模块或配置
        fi
        ;;
esac

# 生成依赖列表
echo "Generating dependency list..."
if command -v cargo &> /dev/null; then
    cargo tree -p "${CRATE_NAME}" --depth 1 > "${PACKAGE_DIR}/dependencies.txt" 2>/dev/null || true
fi

# 复制 README（如果存在）
if [ -f "rust-edge-compute-${EXECUTOR_NAME}/README.md" ]; then
    cp "rust-edge-compute-${EXECUTOR_NAME}/README.md" "${PACKAGE_DIR}/README.md"
else
    # 创建默认 README（根据 executor 类型）
    case "$EXECUTOR_NAME" in
        cpp)
            cat > "${PACKAGE_DIR}/README.md" << EOF
# C++ Executor

Version: ${VERSION}
Features: ${FEATURES:-default}

## Installation

1. Copy library files to your system library path:
   \`\`\`bash
   sudo cp lib/*.so /usr/local/lib/
   sudo ldconfig
   \`\`\`

2. Copy header files to your include path:
   \`\`\`bash
   sudo cp include/*.h /usr/local/include/
   \`\`\`

## Dependencies

See dependencies.txt for the complete dependency list.

## License

MIT License
EOF
            ;;
        ml)
            cat > "${PACKAGE_DIR}/README.md" << EOF
# ML Executor

Version: ${VERSION}
Features: ${FEATURES:-cpu}

## Installation

1. Copy library files to your system library path:
   \`\`\`bash
   sudo cp lib/*.so /usr/local/lib/
   sudo ldconfig
   \`\`\`

## Requirements

- Rust runtime
- Candle ML library dependencies
EOF
            if [ "${FEATURES:-cpu}" = "cuda" ]; then
                echo "- NVIDIA CUDA toolkit" >> "${PACKAGE_DIR}/README.md"
            fi
            if [ "${FEATURES:-cpu}" = "metal" ]; then
                echo "- Apple Metal framework" >> "${PACKAGE_DIR}/README.md"
            fi
            cat >> "${PACKAGE_DIR}/README.md" << 'EOF'

## Dependencies

See dependencies.txt for the complete dependency list.

## License

MIT License
EOF
            ;;
        python)
            cat > "${PACKAGE_DIR}/README.md" << EOF
# Python Executor

Version: ${VERSION}
Features: ${FEATURES:-base}

## Installation

1. Copy library files to your system library path:
   \`\`\`bash
   sudo cp lib/*.so /usr/local/lib/
   sudo ldconfig
   \`\`\`

## Requirements

- Rust runtime
EOF
            if echo "${FEATURES:-base}" | grep -q "python"; then
                echo "- Python 3.11+" >> "${PACKAGE_DIR}/README.md"
            fi
            if echo "${FEATURES:-base}" | grep -q "wasm"; then
                echo "- WASM runtime" >> "${PACKAGE_DIR}/README.md"
            fi
            cat >> "${PACKAGE_DIR}/README.md" << 'EOF'

## Dependencies

See dependencies.txt for the complete dependency list.

## License

MIT License
EOF
            ;;
        *)
            cat > "${PACKAGE_DIR}/README.md" << EOF
# ${EXECUTOR_NAME^} Executor

Version: ${VERSION}
Features: ${FEATURES:-default}

## Installation

Copy the library files to your system library path.

## Dependencies

See dependencies.txt for the complete dependency list.

## License

MIT License
EOF
            ;;
    esac
fi

# 复制 LICENSE（如果存在）
if [ -f "LICENSE" ]; then
    cp LICENSE "${PACKAGE_DIR}/LICENSE"
fi

# 创建版本信息文件
cat > "${PACKAGE_DIR}/VERSION" << EOF
EXECUTOR=${EXECUTOR_NAME}
VERSION=${VERSION}
FEATURES=${FEATURES:-default}
BUILD_DATE=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
COMMIT_SHA=${CI_COMMIT_SHA:-unknown}
EOF

# 创建压缩包
echo "Creating archive..."
cd dist
tar -czf "${PACKAGE_NAME}.tar.gz" "${PACKAGE_NAME}"
cd ..

echo "Package created: dist/${PACKAGE_NAME}.tar.gz"
ls -lh "dist/${PACKAGE_NAME}.tar.gz"

