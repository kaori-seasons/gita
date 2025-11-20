# Executor 分包使用指南

## 📦 概述

本项目支持为不同的 executor 创建独立的依赖包，实现模块化部署和选择性安装。

## 🎯 分包优势

1. **模块化部署**：只部署需要的 executor，减少资源占用
2. **体积优化**：每个包只包含必要的依赖，最小化体积
3. **灵活配置**：支持不同的特性组合（CUDA、Metal、Python、WASM 等）
4. **独立版本**：每个 executor 可以独立版本管理

## 📋 可用的包

### 核心库包

**包名**：`rust-edge-compute-core-{version}.tar.gz`

**描述**：所有 executor 共享的基础库

**内容**：
- 核心库文件（`.rlib`, `.so`）
- 依赖列表
- 使用文档

**依赖**：
- tokio
- serde
- 其他基础依赖

**使用场景**：所有 executor 都需要此包

---

### C++ Executor 包

**包名**：`rust-edge-compute-cpp-{version}.tar.gz`

**描述**：C++ 算法执行器

**内容**：
- 库文件（`.so`, `.rlib`）
- C++ 头文件（`cpp_bridge.h`, `json_parser.h`）
- 依赖列表
- 使用文档

**特性选项**：
- **基础版本**：无额外特性
- **FFTW 版本**：支持 FFTW（需要手动触发构建）

**依赖**：
- rust-edge-compute-core
- cxx
- cxx-build

**使用场景**：需要执行 C++ 算法时

---

### ML Executor 包

**包名**：`rust-edge-compute-ml-{version}-{variant}.tar.gz`

**描述**：机器学习模型推理执行器

**变体**：

#### 1. CPU 版本
- **包名**：`rust-edge-compute-ml-{version}-cpu.tar.gz`
- **特性**：无 GPU 支持
- **适用场景**：CPU 推理
- **体积**：较小

#### 2. CUDA 版本
- **包名**：`rust-edge-compute-ml-{version}-cuda.tar.gz`
- **特性**：NVIDIA GPU 支持
- **适用场景**：GPU 加速推理
- **体积**：较大（包含 CUDA 依赖）
- **触发方式**：手动触发

#### 3. Metal 版本
- **包名**：`rust-edge-compute-ml-{version}-metal.tar.gz`
- **特性**：Apple Metal 支持
- **适用场景**：macOS/iOS GPU 推理
- **体积**：中等
- **触发方式**：手动触发

**内容**：
- 库文件
- 预处理和后处理模块
- 依赖列表
- 使用文档

**依赖**：
- rust-edge-compute-core
- candle-core
- candle-nn
- candle-transformers

**使用场景**：需要执行 ML 模型推理时

---

### Python Executor 包

**包名**：`rust-edge-compute-python-{version}-{variant}.tar.gz`

**描述**：Python 和 WASM 执行器

**变体**：

#### 1. Base 版本
- **包名**：`rust-edge-compute-python-{version}-base.tar.gz`
- **特性**：无 Python 和 WASM 支持
- **适用场景**：基础功能
- **体积**：最小

#### 2. Python 版本
- **包名**：`rust-edge-compute-python-{version}-python.tar.gz`
- **特性**：Python 支持
- **适用场景**：需要执行 Python 代码
- **体积**：中等（包含 PyO3 依赖）

#### 3. WASM 版本
- **包名**：`rust-edge-compute-python-{version}-wasm.tar.gz`
- **特性**：WASM 支持
- **适用场景**：需要执行 WASM 模块
- **体积**：中等（包含 Wasmtime 依赖）

#### 4. Full 版本
- **包名**：`rust-edge-compute-python-{version}-full.tar.gz`
- **特性**：Python + WASM 支持
- **适用场景**：需要完整功能
- **体积**：最大
- **触发方式**：手动触发

**内容**：
- 库文件
- Python 模块（如果启用）
- WASM 模块（如果启用）
- 依赖列表
- 使用文档

**依赖**：
- rust-edge-compute-core
- pyo3（如果启用 python 特性）
- wasmtime（如果启用 wasm 特性）

**使用场景**：需要执行 Python 代码或 WASM 模块时

---

## 🚀 使用方法

### 1. 下载包

在 GitLab 流水线页面：
1. 进入 **CI/CD** > **流水线**
2. 选择已完成的流水线
3. 找到 `package:*` 作业
4. 点击 **浏览** 下载包

### 2. 解压包

```bash
# 解压核心库包
tar -xzf rust-edge-compute-core-{version}.tar.gz

# 解压 C++ Executor 包
tar -xzf rust-edge-compute-cpp-{version}.tar.gz

# 解压 ML Executor 包（CPU 版本）
tar -xzf rust-edge-compute-ml-{version}-cpu.tar.gz

# 解压 Python Executor 包（Python 版本）
tar -xzf rust-edge-compute-python-{version}-python.tar.gz
```

### 3. 安装包

#### Linux

```bash
# 复制库文件到系统库目录
sudo cp rust-edge-compute-core-{version}/lib/*.so /usr/local/lib/
sudo cp rust-edge-compute-cpp-{version}/lib/*.so /usr/local/lib/
sudo cp rust-edge-compute-ml-{version}-cpu/lib/*.so /usr/local/lib/
sudo cp rust-edge-compute-python-{version}-python/lib/*.so /usr/local/lib/

# 复制头文件（C++ Executor）
sudo cp -r rust-edge-compute-cpp-{version}/include/* /usr/local/include/

# 更新动态链接库缓存
sudo ldconfig
```

#### macOS

```bash
# 复制库文件到系统库目录
sudo cp rust-edge-compute-core-{version}/lib/*.dylib /usr/local/lib/
sudo cp rust-edge-compute-cpp-{version}/lib/*.dylib /usr/local/lib/
sudo cp rust-edge-compute-ml-{version}-cpu/lib/*.dylib /usr/local/lib/
sudo cp rust-edge-compute-python-{version}-python/lib/*.dylib /usr/local/lib/

# 复制头文件（C++ Executor）
sudo cp -r rust-edge-compute-cpp-{version}/include/* /usr/local/include/
```

#### Windows

```powershell
# 复制库文件到系统目录
Copy-Item rust-edge-compute-core-{version}\lib\*.dll C:\Windows\System32\
Copy-Item rust-edge-compute-cpp-{version}\lib\*.dll C:\Windows\System32\
Copy-Item rust-edge-compute-ml-{version}-cpu\lib\*.dll C:\Windows\System32\
Copy-Item rust-edge-compute-python-{version}-python\lib\*.dll C:\Windows\System32\

# 复制头文件（C++ Executor）
Copy-Item -Recurse rust-edge-compute-cpp-{version}\include\* C:\Program Files\Rust\include\
```

### 4. 验证安装

```bash
# 检查库文件
ldconfig -p | grep rust_edge_compute

# 或（macOS）
otool -L /usr/local/lib/librust_edge_compute_core.dylib
```

## 📊 包体积对比

| 包名 | 基础体积 | 特性 | 体积增加 |
|------|---------|------|---------|
| core | ~5MB | - | - |
| cpp | ~8MB | fftw | +2MB |
| ml (CPU) | ~15MB | - | - |
| ml (CUDA) | ~50MB | cuda | +35MB |
| ml (Metal) | ~25MB | metal | +10MB |
| python (base) | ~8MB | - | - |
| python (python) | ~20MB | python | +12MB |
| python (wasm) | ~25MB | wasm | +17MB |
| python (full) | ~35MB | python,wasm | +27MB |

*注：实际体积取决于依赖和优化选项*

## 🔧 自定义构建

### 手动触发特殊变体

在 GitLab 流水线页面：

1. 找到需要触发的作业（如 `build:release:ml:cuda`）
2. 点击作业右侧的 **▶️** 按钮
3. 等待构建完成
4. 相应的打包作业会自动运行

### 本地构建特定包

```bash
# 构建 C++ Executor
cargo build -p rust-edge-compute-cpp --release

# 构建 ML Executor (CPU)
cargo build -p rust-edge-compute-ml --release

# 构建 ML Executor (CUDA)
cargo build -p rust-edge-compute-ml --release --features cuda

# 构建 Python Executor (Python)
cargo build -p rust-edge-compute-python --release --features python

# 构建 Python Executor (Full)
cargo build -p rust-edge-compute-python --release --features python,wasm
```

## 📝 依赖管理

### 查看依赖列表

每个包都包含 `dependencies.txt` 文件，列出所有依赖：

```bash
cat rust-edge-compute-cpp-{version}/dependencies.txt
```

### 最小化依赖

- **核心库**：只包含基础依赖
- **C++ Executor**：只包含 C++ 相关依赖
- **ML Executor**：只包含 Candle 相关依赖
- **Python Executor**：根据特性只包含必要的依赖

## 🎯 部署建议

### 边缘设备部署

对于资源受限的边缘设备：

1. **只部署需要的 executor**
2. **选择最小特性组合**（如 ML Executor CPU 版本）
3. **使用体积优化构建**（已在 CI 中配置）

### 服务器部署

对于服务器环境：

1. **可以部署多个 executor**
2. **根据硬件选择特性**（如有 GPU 则使用 CUDA 版本）
3. **使用完整特性组合**（如 Python Executor Full 版本）

## 🔍 故障排除

### 包下载失败

- 检查 GitLab Runner 是否正常运行
- 检查网络连接
- 查看作业日志

### 安装后无法使用

- 检查库文件路径是否正确
- 检查动态链接库路径（`LD_LIBRARY_PATH`）
- 检查依赖是否完整安装

### 特性不工作

- 确认下载了正确的变体包
- 检查系统是否满足特性要求（如 CUDA 需要 NVIDIA GPU）
- 查看依赖列表确认特性已启用

## 📚 参考资源

- [CI/CD 使用指南](ci-cd-guide.md)
- [CI/CD 重构方案](ci-cd-refactor-plan.md)
- [项目 README](../README.md)

