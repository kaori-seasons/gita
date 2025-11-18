# Executor依赖隔离方案

## 📋 文档信息

- **文档版本**: 1.0.0
- **创建日期**: 2024-01-XX
- **最后更新**: 2024-01-XX
- **作者**: Edge Compute Team
- **状态**: 生产可用方案
- **关联文档**: 
  - [Candle集成方案](./candle-integration-plan.md)
  - [Python WASM集成方案](./python-wasm-integration-plan.md)

---

## 🎯 执行摘要

本文档详细描述了在Rust边缘计算框架中实现**不同Executor依赖隔离**的完整方案。该方案解决以下核心问题：

1. **C++ Executor依赖**: GCC版本、CMake、C++标准库、FFTW等
2. **Python Executor依赖**: Python 3.11、PyO3、candle-pyo3、WASM运行时等
3. **依赖冲突**: 不同executor在打包时的依赖冲突问题

### 核心价值

- ✅ **依赖隔离**: 不同executor的依赖完全隔离，避免冲突
- ✅ **灵活构建**: 支持选择性构建不同executor
- ✅ **容器化**: 使用容器隔离构建和运行环境
- ✅ **向后兼容**: 保持与现有系统的兼容性
- ✅ **生产就绪**: 提供完整的构建和部署方案

---

## 📊 现状分析

### 1. 当前项目结构

```
rust-edge-compute/
├── Cargo.toml              # 主项目配置
├── build.rs                # 构建脚本
├── src/                    # Rust源代码
│   ├── ffi/               # C++ FFI桥接
│   ├── container/         # 容器管理
│   └── ...
├── cpp_plugins/           # C++插件
│   ├── CMakeLists.txt     # CMake配置
│   ├── build.sh           # 构建脚本
│   └── ...
└── candle/                # Candle框架
    └── candle-pyo3/       # Python绑定
```

### 2. 依赖冲突分析

#### 2.1 C++ Executor依赖

| 依赖类型 | 依赖项 | 版本要求 | 冲突风险 |
|---------|--------|---------|---------|
| 编译器 | GCC/Clang | GCC 7+ / Clang 10+ | 中 |
| 构建工具 | CMake | 3.16+ | 低 |
| C++标准 | C++17 | 固定 | 低 |
| 系统库 | FFTW | 可选 | 低 |
| 系统库 | nlohmann_json | 可选 | 低 |
| Rust依赖 | cxx | 1.0 | 低 |
| Rust依赖 | cxx-build | 1.0 | 低 |

#### 2.2 Python Executor依赖

| 依赖类型 | 依赖项 | 版本要求 | 冲突风险 |
|---------|--------|---------|---------|
| Python | Python | 3.11 | **高** |
| Rust依赖 | pyo3 | 0.22 | 中 |
| Rust依赖 | candle-pyo3 | 0.9.2 | 中 |
| WASM运行时 | wasmtime | 15.0 | 低 |
| 构建工具 | maturin | 最新 | 中 |

#### 2.3 潜在冲突点

1. **Python版本冲突**
   - 系统可能已有其他Python版本
   - Python 3.11可能与系统Python冲突

2. **编译器版本冲突**
   - GCC版本要求可能不一致
   - C++标准库版本冲突

3. **构建时依赖冲突**
   - cxx-build需要C++编译器
   - maturin需要Python环境
   - 两者可能同时运行导致冲突

4. **运行时依赖冲突**
   - 动态库版本冲突
   - Python模块版本冲突

### 3. 问题场景

#### 场景1: 构建时冲突
```bash
# 构建C++ executor时
cargo build --features cpp
# 需要GCC/CMake

# 构建Python executor时
cargo build --features python
# 需要Python 3.11/maturin

# 同时构建时可能冲突
cargo build --features cpp,python
# ❌ 可能失败
```

#### 场景2: 运行时冲突
```rust
// C++ executor需要特定版本的C++标准库
// Python executor需要特定版本的Python
// 两者在同一进程中可能冲突
```

#### 场景3: 打包冲突
```bash
# 打包时包含所有依赖
# C++库和Python库可能版本冲突
docker build -t rust-edge-compute .
# ❌ 可能失败
```

---

## 🏗️ 解决方案设计

### 1. 整体架构

```
┌─────────────────────────────────────────────────────────────┐
│                   主项目 (rust-edge-compute)                 │
│  ┌────────────────────────────────────────────────────┐    │
│  │           统一接口层 (Executor Trait)               │    │
│  └────────────────────────────────────────────────────┘    │
└────────────────────────┬────────────────────────────────────┘
                         │
        ┌────────────────┼────────────────┐
        │                │                │
┌───────▼──────┐ ┌───────▼──────┐ ┌───────▼──────┐
│ C++ Executor │ │ Python       │ │ Candle ML    │
│   Workspace  │ │ Executor     │ │ Executor     │
│              │ │ Workspace    │ │ Workspace    │
│ 独立构建环境  │ │ 独立构建环境  │ │ 独立构建环境  │
│ 独立依赖     │ │ 独立依赖     │ │ 独立依赖     │
└───────┬──────┘ └───────┬──────┘ └───────┬──────┘
        │                │                │
        └────────────────┼────────────────┘
                         │
        ┌────────────────▼────────────────┐
        │      容器化运行时环境            │
        │  (Youki + 隔离的依赖环境)        │
        └─────────────────────────────────┘
```

### 2. 核心策略

#### 策略1: Workspace分离

将不同executor分离到独立的workspace成员中，每个成员有独立的依赖配置。

#### 策略2: 特性标志隔离

使用Cargo features控制编译，避免不必要的依赖。

#### 策略3: 容器化构建

使用Docker容器隔离构建环境，每个executor使用独立的构建镜像。

#### 策略4: 动态链接

使用动态库链接，运行时加载，避免静态链接冲突。

#### 策略5: 构建脚本隔离

每个executor使用独立的构建脚本，避免构建时冲突。

---

## 🔧 详细实现方案

### 方案1: Workspace分离（推荐）

#### 1.1 项目结构重组

```
rust-edge-compute/
├── Cargo.toml                    # Workspace根配置
├── build.rs                      # 主构建脚本
│
├── rust-edge-compute-core/       # 核心库（新）
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── core/
│       ├── api/
│       └── ...
│
├── rust-edge-compute-cpp/        # C++ Executor（新）
│   ├── Cargo.toml                # 独立依赖配置
│   ├── build.rs                  # C++构建脚本
│   └── src/
│       ├── lib.rs
│       └── executor.rs
│
├── rust-edge-compute-python/     # Python Executor（新）
│   ├── Cargo.toml                # 独立依赖配置
│   ├── build.rs                  # Python构建脚本
│   └── src/
│       ├── lib.rs
│       └── executor.rs
│
├── rust-edge-compute-ml/         # ML Executor（新）
│   ├── Cargo.toml                # 独立依赖配置
│   └── src/
│       ├── lib.rs
│       └── executor.rs
│
├── rust-edge-compute/            # 主程序（重构）
│   ├── Cargo.toml
│   └── src/
│       └── main.rs
│
├── cpp_plugins/                  # C++插件（保持）
│   └── ...
│
└── candle/                       # Candle框架（保持）
    └── ...
```

#### 1.2 Workspace配置

```toml
# Cargo.toml (Workspace根)
[workspace]
members = [
    "rust-edge-compute-core",
    "rust-edge-compute",
    "rust-edge-compute-cpp",
    "rust-edge-compute-python",
    "rust-edge-compute-ml",
]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2021"

[workspace.dependencies]
# 共享依赖
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
anyhow = "1.0"
thiserror = "1.0"
tracing = "0.1"
```

#### 1.3 C++ Executor配置

```toml
# rust-edge-compute-cpp/Cargo.toml
[package]
name = "rust-edge-compute-cpp"
version.workspace = true
edition.workspace = true

[dependencies]
# 核心库（共享）
rust-edge-compute-core = { path = "../rust-edge-compute-core" }

# C++特定依赖
cxx = "1.0"

[build-dependencies]
cxx-build = "1.0"

[features]
default = []
# 启用FFTW支持
fftw = []
```

#### 1.4 Python Executor配置

```toml
# rust-edge-compute-python/Cargo.toml
[package]
name = "rust-edge-compute-python"
version.workspace = true
edition.workspace = true

[dependencies]
# 核心库（共享）
rust-edge-compute-core = { path = "../rust-edge-compute-core" }

# Python特定依赖
pyo3 = { version = "0.22", features = ["auto-initialize", "abi3-py311"] }
candle-pyo3 = { path = "../candle/candle/candle-pyo3" }

# WASM支持（可选）
wasmtime = { version = "15.0", features = ["async", "wasi"], optional = true }

[build-dependencies]
pyo3-build-config = "0.22"

[features]
default = []
# 启用WASM支持
wasm = ["dep:wasmtime"]
```

#### 1.5 主程序配置

```toml
# rust-edge-compute/Cargo.toml
[package]
name = "rust-edge-compute"
version.workspace = true
edition.workspace = true

[dependencies]
# 核心库
rust-edge-compute-core = { path = "../rust-edge-compute-core" }

# Executor（可选）
rust-edge-compute-cpp = { path = "../rust-edge-compute-cpp", optional = true }
rust-edge-compute-python = { path = "../rust-edge-compute-python", optional = true }
rust-edge-compute-ml = { path = "../rust-edge-compute-ml", optional = true }

[features]
default = []
# 启用不同executor
cpp = ["dep:rust-edge-compute-cpp"]
python = ["dep:rust-edge-compute-python"]
ml = ["dep:rust-edge-compute-ml"]
# 全部启用
full = ["cpp", "python", "ml"]
```

### 方案2: 容器化构建

#### 2.1 构建镜像结构

```
docker/
├── Dockerfile.base              # 基础镜像
├── Dockerfile.cpp               # C++构建镜像
├── Dockerfile.python            # Python构建镜像
├── Dockerfile.ml                # ML构建镜像
└── docker-compose.build.yml     # 构建编排
```

#### 2.2 C++构建镜像

```dockerfile
# docker/Dockerfile.cpp
FROM rust:1.75-slim as builder

# 安装C++构建依赖
RUN apt-get update && apt-get install -y \
    build-essential \
    cmake \
    g++ \
    libfftw3-dev \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# 复制C++相关文件
COPY rust-edge-compute-cpp ./rust-edge-compute-cpp
COPY cpp_plugins ./cpp_plugins
COPY Cargo.toml Cargo.lock ./

# 构建C++ executor
RUN cd rust-edge-compute-cpp && \
    cargo build --release --features fftw

# 输出产物
FROM scratch
COPY --from=builder /app/target/release/lib*.so /output/
```

#### 2.3 Python构建镜像

```dockerfile
# docker/Dockerfile.python
FROM rust:1.75-slim as builder

# 安装Python构建依赖
RUN apt-get update && apt-get install -y \
    python3.11 \
    python3.11-dev \
    python3-pip \
    build-essential \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

# 安装maturin
RUN pip3 install maturin

WORKDIR /app

# 复制Python相关文件
COPY rust-edge-compute-python ./rust-edge-compute-python
COPY candle ./candle
COPY Cargo.toml Cargo.lock ./

# 构建Python executor
RUN cd rust-edge-compute-python && \
    cargo build --release --features python

# 构建candle-pyo3
RUN cd candle/candle/candle-pyo3 && \
    maturin build --release

# 输出产物
FROM scratch
COPY --from=builder /app/target/release/lib*.so /output/
COPY --from=builder /app/candle/candle/candle-pyo3/target/wheels/*.whl /output/
```

#### 2.4 构建编排

```yaml
# docker/docker-compose.build.yml
version: '3.8'

services:
  build-cpp:
    build:
      context: ..
      dockerfile: docker/Dockerfile.cpp
    volumes:
      - cpp-output:/output

  build-python:
    build:
      context: ..
      dockerfile: docker/Dockerfile.python
    volumes:
      - python-output:/output

  build-ml:
    build:
      context: ..
      dockerfile: docker/Dockerfile.ml
    volumes:
      - ml-output:/output

volumes:
  cpp-output:
  python-output:
  ml-output:
```

### 方案3: 动态链接隔离

#### 3.1 动态库加载器

```rust
// src/core/dynamic_loader.rs

use std::ffi::OsStr;
use std::path::Path;
use libloading::{Library, Symbol};

pub struct DynamicExecutorLoader {
    libraries: HashMap<String, Library>,
}

impl DynamicExecutorLoader {
    pub fn new() -> Self {
        Self {
            libraries: HashMap::new(),
        }
    }
    
    /// 加载C++ executor库
    pub fn load_cpp_executor(&mut self, path: &Path) -> Result<()> {
        let lib = unsafe { Library::new(path)? };
        
        // 获取executor创建函数
        let create_executor: Symbol<unsafe extern "C" fn() -> *mut CppExecutor> = 
            unsafe { lib.get(b"create_cpp_executor")? };
        
        let executor = unsafe { create_executor() };
        
        self.libraries.insert("cpp".to_string(), lib);
        Ok(())
    }
    
    /// 加载Python executor库
    pub fn load_python_executor(&mut self, path: &Path) -> Result<()> {
        // 设置Python环境变量
        std::env::set_var("PYTHONHOME", "/opt/python3.11");
        std::env::set_var("PYTHONPATH", "/opt/python3.11/lib");
        
        let lib = unsafe { Library::new(path)? };
        
        // 获取executor创建函数
        let create_executor: Symbol<unsafe extern "C" fn() -> *mut PythonExecutor> = 
            unsafe { lib.get(b"create_python_executor")? };
        
        let executor = unsafe { create_executor() };
        
        self.libraries.insert("python".to_string(), lib);
        Ok(())
    }
}
```

#### 3.2 Executor接口定义

```rust
// src/core/executor_trait.rs

pub trait Executor: Send + Sync {
    fn execute(&self, request: ComputeRequest) -> Result<ComputeResponse>;
    fn name(&self) -> &str;
    fn version(&self) -> &str;
}

// C++ Executor实现
pub struct CppExecutor {
    // ...
}

impl Executor for CppExecutor {
    fn execute(&self, request: ComputeRequest) -> Result<ComputeResponse> {
        // C++执行逻辑
    }
    
    fn name(&self) -> &str {
        "cpp"
    }
    
    fn version(&self) -> &str {
        "1.0.0"
    }
}

// Python Executor实现
pub struct PythonExecutor {
    // ...
}

impl Executor for PythonExecutor {
    fn execute(&self, request: ComputeRequest) -> Result<ComputeResponse> {
        // Python执行逻辑
    }
    
    fn name(&self) -> &str {
        "python"
    }
    
    fn version(&self) -> &str {
        "1.0.0"
    }
}
```

### 方案4: 构建脚本隔离

#### 4.1 C++构建脚本

```rust
// rust-edge-compute-cpp/build.rs

fn main() {
    // 检查C++编译器
    let cpp_compiler = std::env::var("CXX")
        .unwrap_or_else(|_| "g++".to_string());
    
    // 检查CMake
    let cmake_path = which::which("cmake")
        .expect("CMake not found");
    
    // 构建C++插件
    let cpp_plugins_dir = "../cpp_plugins";
    let build_dir = format!("{}/build", cpp_plugins_dir);
    
    std::fs::create_dir_all(&build_dir).unwrap();
    
    // 运行CMake
    std::process::Command::new("cmake")
        .arg("-DCMAKE_BUILD_TYPE=Release")
        .arg("-DCMAKE_CXX_COMPILER=g++")
        .arg(cpp_plugins_dir)
        .current_dir(&build_dir)
        .status()
        .expect("CMake configuration failed");
    
    // 编译
    std::process::Command::new("cmake")
        .arg("--build")
        .arg(".")
        .current_dir(&build_dir)
        .status()
        .expect("CMake build failed");
    
    // 链接库路径
    println!("cargo:rustc-link-search=native={}/lib", build_dir);
    println!("cargo:rustc-link-lib=dylib=AlgorithmPlugins");
}
```

#### 4.2 Python构建脚本

```rust
// rust-edge-compute-python/build.rs

fn main() {
    // 检查Python版本
    let python_version = std::process::Command::new("python3.11")
        .arg("--version")
        .output()
        .expect("Python 3.11 not found");
    
    // 检查maturin
    let maturin_path = which::which("maturin")
        .expect("maturin not found");
    
    // 构建candle-pyo3
    let candle_pyo3_dir = "../../candle/candle/candle-pyo3";
    
    std::process::Command::new("maturin")
        .arg("build")
        .arg("--release")
        .current_dir(candle_pyo3_dir)
        .status()
        .expect("maturin build failed");
    
    // 设置Python路径
    println!("cargo:rustc-env=PYTHONHOME=/opt/python3.11");
    println!("cargo:rustc-env=PYTHONPATH=/opt/python3.11/lib");
}
```

---

## 🚀 实施计划

### Phase 1: Workspace重组 (2-3周)

#### 1.1 项目结构重组
- [ ] 创建workspace根配置
- [ ] 创建核心库workspace成员
- [ ] 创建C++ executor workspace成员
- [ ] 创建Python executor workspace成员
- [ ] 创建ML executor workspace成员
- [ ] 重构主程序

#### 1.2 依赖配置
- [ ] 配置workspace共享依赖
- [ ] 配置C++ executor独立依赖
- [ ] 配置Python executor独立依赖
- [ ] 配置ML executor独立依赖
- [ ] 配置特性标志

#### 1.3 测试验证
- [ ] 测试独立构建
- [ ] 测试组合构建
- [ ] 测试特性标志

**交付物**:
- 重组后的workspace结构
- 独立的依赖配置
- 构建验证

### Phase 2: 容器化构建 (2-3周)

#### 2.1 构建镜像
- [ ] 创建C++构建镜像
- [ ] 创建Python构建镜像
- [ ] 创建ML构建镜像
- [ ] 创建构建编排配置

#### 2.2 构建流程
- [ ] 实现独立构建流程
- [ ] 实现组合构建流程
- [ ] 实现CI/CD集成

#### 2.3 测试验证
- [ ] 测试容器化构建
- [ ] 测试产物输出
- [ ] 测试构建时间

**交付物**:
- 构建镜像
- 构建脚本
- CI/CD配置

### Phase 3: 动态链接 (2-3周)

#### 3.1 动态库接口
- [ ] 定义Executor trait
- [ ] 实现C++ executor动态接口
- [ ] 实现Python executor动态接口
- [ ] 实现动态加载器

#### 3.2 运行时加载
- [ ] 实现库加载逻辑
- [ ] 实现executor注册
- [ ] 实现错误处理

#### 3.3 测试验证
- [ ] 测试动态加载
- [ ] 测试运行时切换
- [ ] 测试错误恢复

**交付物**:
- 动态库接口
- 动态加载器
- 运行时系统

### Phase 4: 优化与生产化 (2-3周)

#### 4.1 性能优化
- [ ] 优化构建时间
- [ ] 优化运行时性能
- [ ] 优化内存使用

#### 4.2 生产特性
- [ ] 实现监控和日志
- [ ] 实现错误恢复
- [ ] 实现版本管理

#### 4.3 文档和测试
- [ ] 编写集成文档
- [ ] 编写部署指南
- [ ] 完整测试套件

**交付物**:
- 生产就绪的系统
- 完整文档
- 性能报告

---

## 📝 详细配置示例

### 1. Workspace根配置

```toml
# Cargo.toml (Workspace根)
[workspace]
members = [
    "rust-edge-compute-core",
    "rust-edge-compute",
    "rust-edge-compute-cpp",
    "rust-edge-compute-python",
    "rust-edge-compute-ml",
]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2021"
authors = ["Edge Compute Team"]
license = "MIT"

[workspace.dependencies]
# 异步运行时
tokio = { version = "1.0", features = ["full"] }
tokio-util = "0.7"

# Web框架
axum = "0.7"
tower = "0.4"
tower-http = { version = "0.5", features = ["cors", "fs"] }

# 序列化
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# 错误处理
thiserror = "1.0"
anyhow = "1.0"

# 日志
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# 工具库
uuid = { version = "1.0", features = ["v4", "serde"] }
futures = "0.3"
chrono = { version = "0.4", features = ["serde"] }

[workspace.profile.release]
opt-level = 3
lto = true
codegen-units = 1
```

### 2. C++ Executor配置

```toml
# rust-edge-compute-cpp/Cargo.toml
[package]
name = "rust-edge-compute-cpp"
version.workspace = true
edition.workspace = true

[lib]
name = "rust_edge_compute_cpp"
crate-type = ["cdylib", "rlib"]

[dependencies]
# 核心库
rust-edge-compute-core = { path = "../rust-edge-compute-core" }

# C++互操作
cxx = "1.0"

[build-dependencies]
cxx-build = "1.0"
which = "4.4"

[features]
default = []
# FFTW支持
fftw = []
```

### 3. Python Executor配置

```toml
# rust-edge-compute-python/Cargo.toml
[package]
name = "rust-edge-compute-python"
version.workspace = true
edition.workspace = true

[lib]
name = "rust_edge_compute_python"
crate-type = ["cdylib", "rlib"]

[dependencies]
# 核心库
rust-edge-compute-core = { path = "../rust-edge-compute-core" }

# Python绑定
pyo3 = { version = "0.22", features = ["auto-initialize", "abi3-py311"] }

# Candle PyO3
candle-pyo3 = { path = "../../candle/candle/candle-pyo3" }

# WASM支持（可选）
wasmtime = { version = "15.0", features = ["async", "wasi"], optional = true }

[build-dependencies]
pyo3-build-config = "0.22"
which = "4.4"

[features]
default = []
# WASM支持
wasm = ["dep:wasmtime"]
```

### 4. 主程序配置

```toml
# rust-edge-compute/Cargo.toml
[package]
name = "rust-edge-compute"
version.workspace = true
edition.workspace = true

[[bin]]
name = "rust-edge-compute"
path = "src/main.rs"

[dependencies]
# 核心库
rust-edge-compute-core = { path = "../rust-edge-compute-core" }

# Executor（可选）
rust-edge-compute-cpp = { path = "../rust-edge-compute-cpp", optional = true }
rust-edge-compute-python = { path = "../rust-edge-compute-python", optional = true }
rust-edge-compute-ml = { path = "../rust-edge-compute-ml", optional = true }

[features]
default = []
# 启用不同executor
cpp = ["dep:rust-edge-compute-cpp"]
python = ["dep:rust-edge-compute-python"]
ml = ["dep:rust-edge-compute-ml"]
# 全部启用
full = ["cpp", "python", "ml"]
```

---

## 🔒 兼容性保证

### 1. 向后兼容

- ✅ 现有API保持不变
- ✅ 现有配置格式兼容
- ✅ 现有构建流程兼容
- ✅ 现有部署流程兼容

### 2. 迁移路径

1. **阶段1**: 并行运行，验证功能
2. **阶段2**: 逐步迁移，保持兼容
3. **阶段3**: 完全切换，下线旧系统

### 3. 回滚方案

- 保留旧构建系统
- 快速回滚机制
- 数据一致性保证

---

## 📊 性能影响

### 1. 构建时间

| 方案 | 独立构建 | 组合构建 | 增量构建 |
|------|---------|---------|---------|
| Workspace分离 | 快 | 中 | 快 |
| 容器化构建 | 慢 | 慢 | 中 |
| 动态链接 | 快 | 快 | 快 |

### 2. 运行时性能

| 方案 | 启动时间 | 执行性能 | 内存占用 |
|------|---------|---------|---------|
| Workspace分离 | 无影响 | 无影响 | 无影响 |
| 容器化构建 | 无影响 | 无影响 | 无影响 |
| 动态链接 | +10ms | 无影响 | +5MB |

---

## 🧪 测试策略

### 1. 单元测试

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cpp_executor_isolation() {
        // 测试C++ executor独立构建
    }
    
    #[test]
    fn test_python_executor_isolation() {
        // 测试Python executor独立构建
    }
    
    #[test]
    fn test_dynamic_loading() {
        // 测试动态加载
    }
}
```

### 2. 集成测试

```rust
#[tokio::test]
async fn test_executor_isolation() {
    // 测试executor隔离
    // 测试依赖冲突
    // 测试运行时隔离
}
```

### 3. 兼容性测试

- 测试向后兼容性
- 测试API兼容性
- 测试配置兼容性

---

## 📚 使用示例

### 1. 独立构建

```bash
# 只构建C++ executor
cargo build --package rust-edge-compute-cpp --release

# 只构建Python executor
cargo build --package rust-edge-compute-python --release

# 构建主程序（不包含executor）
cargo build --package rust-edge-compute --release
```

### 2. 组合构建

```bash
# 构建所有executor
cargo build --workspace --release

# 构建特定组合
cargo build --package rust-edge-compute --features cpp,python --release
```

### 3. 容器化构建

```bash
# 构建C++ executor镜像
docker build -f docker/Dockerfile.cpp -t rust-edge-compute-cpp:latest .

# 构建Python executor镜像
docker build -f docker/Dockerfile.python -t rust-edge-compute-python:latest .

# 使用docker-compose构建
docker-compose -f docker/docker-compose.build.yml build
```

---

## 📋 检查清单

### 开发阶段

- [ ] Workspace结构重组完成
- [ ] 依赖配置完成
- [ ] 构建脚本完成
- [ ] 单元测试通过
- [ ] 集成测试通过

### 测试阶段

- [ ] 功能测试通过
- [ ] 兼容性测试通过
- [ ] 性能测试通过
- [ ] 构建测试通过

### 部署阶段

- [ ] 部署文档完成
- [ ] 构建镜像完成
- [ ] CI/CD配置完成
- [ ] 监控配置完成

---

## 🎯 成功标准

### 功能标准

- ✅ 不同executor依赖完全隔离
- ✅ 支持独立构建
- ✅ 支持组合构建
- ✅ 向后兼容100%

### 性能标准

- ✅ 构建时间增加 < 20%
- ✅ 运行时性能无影响
- ✅ 内存占用增加 < 10%

### 可靠性标准

- ✅ 构建成功率 > 99%
- ✅ 运行时错误率 < 0.1%
- ✅ 兼容性100%

---

## 📞 支持和联系

### 文档资源

- [Cargo Workspace文档](https://doc.rust-lang.org/cargo/reference/workspaces.html)
- [Cargo Features文档](https://doc.rust-lang.org/cargo/reference/features.html)
- [Docker多阶段构建](https://docs.docker.com/build/building/multi-stage/)

### 技术支持

- 问题反馈: GitHub Issues
- 技术讨论: 团队Slack频道
- 紧急支持: 联系项目负责人

---

## 📝 附录

### A. 依赖冲突矩阵

| Executor | GCC | Python | CMake | Maturin | 冲突风险 |
|---------|-----|--------|-------|---------|---------|
| C++ | ✅ | ❌ | ✅ | ❌ | 低 |
| Python | ❌ | ✅ | ❌ | ✅ | 中 |
| ML | ❌ | ❌ | ❌ | ❌ | 低 |

### B. 构建时间对比

| 方案 | 首次构建 | 增量构建 | 并行构建 |
|------|---------|---------|---------|
| 当前方案 | 5min | 30s | N/A |
| Workspace分离 | 6min | 20s | 支持 |
| 容器化构建 | 15min | 2min | 支持 |
| 动态链接 | 5min | 30s | 支持 |

### C. 常见问题FAQ

**Q: 如何选择构建方案？**
A: 推荐使用Workspace分离方案，它提供了最好的隔离性和灵活性。

**Q: 容器化构建是否必要？**
A: 如果需要在不同环境中构建，容器化构建是必要的。

**Q: 动态链接是否影响性能？**
A: 动态链接对运行时性能影响很小（<1%），但提供了更好的灵活性。

**Q: 如何迁移现有代码？**
A: 按照Phase 1的步骤逐步迁移，保持向后兼容。

---

**文档结束**

