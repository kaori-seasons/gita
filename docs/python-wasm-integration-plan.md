# Python自定义算法WASM沙箱集成方案

## 📋 文档信息

- **文档版本**: 1.0.0
- **创建日期**: 2024-01-XX
- **最后更新**: 2024-01-XX
- **作者**: Edge Compute Team
- **状态**: 生产可用方案
- **关联文档**: [Candle集成方案](./candle-integration-plan.md)

---

## 🎯 执行摘要

本文档详细描述了在Rust边缘计算框架中集成**Python自定义算法**和**WASM沙箱环境**的完整方案。该方案将：

1. **WASM沙箱环境**: 使用WebAssembly提供安全的沙箱执行环境
2. **Python算法支持**: 通过PyO3集成Python自定义算法
3. **容器化部署**: 在容器中运行WASM运行时和Python解释器
4. **兼容性保证**: 确保与现有C++算法、Candle ML框架的兼容性

### 核心价值

- ✅ **安全隔离**: WASM提供内存安全、沙箱隔离
- ✅ **Python生态**: 支持丰富的Python ML库和自定义模型
- ✅ **轻量级**: WASM运行时体积小，启动快
- ✅ **跨平台**: WASM可在多种平台运行
- ✅ **兼容性**: 与现有系统无缝集成

---

## 📊 需求分析与分解

### 1. 功能需求

#### 1.1 核心功能需求

| 需求ID | 需求描述 | 优先级 | 复杂度 |
|--------|---------|--------|--------|
| REQ-001 | WASM沙箱环境支持 | P0 | 高 |
| REQ-002 | Python算法执行引擎 | P0 | 高 |
| REQ-003 | PyO3集成和绑定 | P0 | 中 |
| REQ-004 | 容器化WASM运行时 | P0 | 中 |
| REQ-005 | Python依赖管理 | P1 | 中 |
| REQ-006 | 模型加载和缓存 | P1 | 中 |
| REQ-007 | 与现有系统集成 | P0 | 高 |
| REQ-008 | 错误处理和恢复 | P1 | 中 |
| REQ-009 | 性能监控 | P1 | 低 |
| REQ-010 | 安全隔离 | P0 | 高 |

#### 1.2 非功能需求

| 需求ID | 需求描述 | 目标值 |
|--------|---------|--------|
| NFR-001 | 启动时间 | < 2s |
| NFR-002 | 内存占用 | < 512MB (单实例) |
| NFR-003 | CPU开销 | < 10% (空闲时) |
| NFR-004 | 并发支持 | 10+ 并发任务 |
| NFR-005 | 错误率 | < 0.1% |
| NFR-006 | 兼容性 | 100% 向后兼容 |

### 2. 技术需求分解

#### 2.1 WASM沙箱环境

**需求描述**:
- 提供基于WASM的安全沙箱执行环境
- 支持在容器中运行WASM模块
- 提供资源限制和隔离

**技术选型**:
- **WASM运行时**: Wasmtime (Rust原生，性能好)
- **WASI支持**: 支持文件系统、网络等系统调用
- **资源限制**: CPU、内存、执行时间限制

**实现要点**:
```rust
// WASM运行时封装
pub struct WasmSandbox {
    engine: wasmtime::Engine,
    store: wasmtime::Store<WasmContext>,
    instance: wasmtime::Instance,
    limits: ResourceLimits,
}
```

#### 2.2 Python算法执行

**需求描述**:
- 在WASM沙箱中执行Python代码
- 支持PyO3绑定的Candle功能
- 支持自定义Python模型

**技术选型**:
- **Python运行时**: PyO3 (Rust-Python绑定)
- **WASM Python**: Pyodide (Python的WASM版本) 或 自定义Python解释器
- **模型加载**: 支持safetensors、ONNX等格式

**实现要点**:
```rust
// Python执行器
pub struct PythonExecutor {
    py_runtime: PyRuntime,
    wasm_sandbox: WasmSandbox,
    model_loader: ModelLoader,
}
```

#### 2.3 容器化集成

**需求描述**:
- 在Youki容器中运行WASM运行时
- 提供Python环境
- 资源隔离和限制

**技术选型**:
- **容器运行时**: Youki (现有)
- **WASM运行时**: Wasmtime
- **Python环境**: Pyodide或标准Python

**实现要点**:
```rust
// 容器化WASM执行器
pub struct ContainerizedWasmExecutor {
    container_manager: Arc<YoukiContainerManager>,
    wasm_runtime: Arc<WasmRuntime>,
    python_env: Arc<PythonEnvironment>,
}
```

### 3. 兼容性需求

#### 3.1 与现有系统兼容

| 组件 | 兼容性要求 | 实现方式 |
|------|-----------|---------|
| 任务调度器 | 统一接口 | 实现AlgorithmExecutor trait |
| API接口 | 统一格式 | 使用相同的ComputeRequest/Response |
| 错误处理 | 统一错误类型 | 扩展EdgeComputeError |
| 监控系统 | 统一指标 | 使用相同的Metrics接口 |
| 配置系统 | 统一配置 | 扩展Settings结构 |

#### 3.2 向后兼容

- 现有C++算法继续工作
- 现有Candle ML算法继续工作
- 现有API接口保持不变
- 现有配置格式兼容

---

## 🏗️ 架构设计

### 1. 整体架构

```
┌─────────────────────────────────────────────────────────────┐
│                      客户端层                                 │
│              (REST API / WebSocket / gRPC)                   │
└────────────────────────┬────────────────────────────────────┘
                         │
┌────────────────────────▼────────────────────────────────────┐
│                   任务调度层                                   │
│  ┌────────────────────────────────────────────────────┐    │
│  │           统一任务调度器 (TaskScheduler)              │    │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐        │    │
│  │  │C++算法   │  │Candle ML │  │Python    │        │    │
│  │  │任务队列  │  │任务队列  │  │WASM任务  │        │    │
│  │  └──────────┘  └──────────┘  └──────────┘        │    │
│  └────────────────────────────────────────────────────┘    │
└────────────────────────┬────────────────────────────────────┘
                         │
        ┌────────────────┼────────────────┐
        │                │                │
┌───────▼──────┐ ┌───────▼──────┐ ┌───────▼──────┐
│  C++算法执行 │ │ Candle ML    │ │ Python WASM  │
│   (FFI)      │ │   (Native)   │ │  (Sandbox)    │
└──────────────┘ └───────────────┘ └──────────────┘
                         │
        ┌────────────────┼────────────────┐
        │                │                │
┌───────▼──────┐ ┌───────▼──────┐ ┌───────▼──────┐
│  容器运行时  │ │  模型缓存     │ │ WASM运行时   │
│  (Youki)     │ │  (Memory)    │ │ (Wasmtime)   │
└──────────────┘ └───────────────┘ └──────────────┘
```

### 2. 核心模块设计

#### 2.1 WASM沙箱模块

```rust
// src/wasm/mod.rs

pub mod sandbox;
pub mod runtime;
pub mod wasi;
pub mod limits;

// src/wasm/sandbox.rs
pub struct WasmSandbox {
    engine: wasmtime::Engine,
    store: wasmtime::Store<WasmContext>,
    instance: Option<wasmtime::Instance>,
    limits: ResourceLimits,
    config: SandboxConfig,
}

pub struct SandboxConfig {
    /// 最大内存限制（MB）
    max_memory_mb: usize,
    /// 最大执行时间（秒）
    max_execution_time: Duration,
    /// 最大栈大小（MB）
    max_stack_size_mb: usize,
    /// 允许的系统调用
    allowed_syscalls: Vec<String>,
    /// WASI配置
    wasi_config: WasiConfig,
}

impl WasmSandbox {
    /// 创建新的WASM沙箱
    pub fn new(config: SandboxConfig) -> Result<Self>;
    
    /// 加载WASM模块
    pub fn load_module(&mut self, wasm_bytes: &[u8]) -> Result<()>;
    
    /// 执行函数
    pub fn call_function(
        &mut self,
        name: &str,
        args: &[WasmValue],
    ) -> Result<Vec<WasmValue>>;
    
    /// 设置资源限制
    pub fn set_limits(&mut self, limits: ResourceLimits) -> Result<()>;
}
```

#### 2.2 Python执行器模块

```rust
// src/python/mod.rs

pub mod executor;
pub mod runtime;
pub mod pyo3_bridge;
pub mod model_loader;

// src/python/executor.rs
pub struct PythonExecutor {
    /// PyO3 Python运行时
    py_runtime: PyRuntime,
    /// WASM沙箱
    wasm_sandbox: Option<WasmSandbox>,
    /// 模型加载器
    model_loader: Arc<ModelLoader>,
    /// 执行配置
    config: PythonExecutorConfig,
}

pub struct PythonExecutorConfig {
    /// 使用WASM沙箱
    use_wasm_sandbox: bool,
    /// Python版本
    python_version: String,
    /// 依赖管理
    dependency_manager: DependencyManager,
    /// 资源限制
    resource_limits: ResourceLimits,
}

impl PythonExecutor {
    /// 执行Python代码
    pub async fn execute_code(
        &self,
        code: &str,
        inputs: &serde_json::Value,
    ) -> Result<serde_json::Value>;
    
    /// 执行Python函数
    pub async fn call_function(
        &self,
        module: &str,
        function: &str,
        args: &[PyObject],
    ) -> Result<PyObject>;
    
    /// 加载Python模型
    pub async fn load_model(
        &self,
        model_path: &Path,
        model_type: ModelType,
    ) -> Result<PyModel>;
}
```

#### 2.3 PyO3桥接模块

```rust
// src/python/pyo3_bridge.rs

use pyo3::prelude::*;
use pyo3::types::PyModule;

pub struct PyO3Bridge {
    py: Python,
    candle_module: Py<PyModule>,
}

impl PyO3Bridge {
    /// 初始化PyO3桥接
    pub fn new() -> Result<Self> {
        Python::with_gil(|py| {
            // 导入candle-pyo3模块
            let candle_module = PyModule::import_bound(py, "candle")?;
            
            Ok(Self {
                py: py.into(),
                candle_module: candle_module.unbind(),
            })
        })
    }
    
    /// 创建Candle Tensor
    pub fn create_tensor(
        &self,
        data: &[f32],
        shape: &[usize],
    ) -> Result<PyObject> {
        Python::with_gil(|py| {
            let tensor = self.candle_module
                .getattr(py, "Tensor")?
                .call1(py, (data, shape))?;
            Ok(tensor.to_object(py))
        })
    }
    
    /// 执行模型推理
    pub fn run_inference(
        &self,
        model: &PyModel,
        inputs: &PyObject,
    ) -> Result<PyObject> {
        Python::with_gil(|py| {
            let result = model.call_method1(py, "forward", (inputs,))?;
            Ok(result.to_object(py))
        })
    }
}
```

#### 2.4 容器化WASM执行器

```rust
// src/container/wasm_executor.rs

pub struct ContainerizedWasmExecutor {
    container_manager: Arc<YoukiContainerManager>,
    wasm_runtime: Arc<WasmRuntime>,
    python_env: Arc<PythonEnvironment>,
    algorithm_registry: Arc<RwLock<AlgorithmRegistry>>,
}

impl ContainerizedWasmExecutor {
    /// 执行Python算法（在WASM沙箱中）
    pub async fn execute_python_algorithm(
        &self,
        request: ComputeRequest,
    ) -> Result<ComputeResponse> {
        // 1. 获取算法信息
        let algorithm_info = self.get_algorithm_info(&request.algorithm).await?;
        
        // 2. 创建容器配置
        let container_config = self.create_container_config(&algorithm_info)?;
        
        // 3. 创建容器
        let container = self.container_manager
            .create_container(container_config)
            .await?;
        
        // 4. 在容器中启动WASM运行时
        let wasm_sandbox = self.wasm_runtime
            .create_sandbox_in_container(&container)
            .await?;
        
        // 5. 加载Python环境
        let python_env = self.python_env
            .load_in_container(&container)
            .await?;
        
        // 6. 执行算法
        let result = self.execute_in_sandbox(
            &wasm_sandbox,
            &python_env,
            &request,
        ).await?;
        
        // 7. 清理容器
        self.container_manager.delete_container(&container.id).await?;
        
        Ok(result)
    }
}
```

### 3. 数据流设计

```
用户请求
    │
    ▼
任务调度器
    │
    ├─→ C++算法执行器
    ├─→ Candle ML执行器
    └─→ Python WASM执行器
            │
            ├─→ 创建容器
            │       │
            │       ├─→ 启动WASM运行时
            │       │       │
            │       │       └─→ 加载WASM模块
            │       │
            │       └─→ 加载Python环境
            │               │
            │               ├─→ PyO3绑定
            │               └─→ 模型加载
            │
            └─→ 执行算法
                    │
                    ├─→ Python代码执行
                    ├─→ Candle API调用
                    └─→ 结果返回
```

---

## 🔧 技术实现方案

### 1. 依赖集成

#### Cargo.toml 配置

```toml
[dependencies]
# WASM运行时
wasmtime = { version = "15.0", features = ["async", "wasi"] }

# Python绑定
pyo3 = { version = "0.22", features = ["auto-initialize", "abi3-py311"] }

# Candle PyO3（使用本地路径）
candle-pyo3 = { path = "./candle/candle/candle-pyo3", version = "0.9.2-alpha.1" }

# 异步支持
tokio = { version = "1.0", features = ["full"] }
async-trait = "0.1"

# 序列化
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# 错误处理
thiserror = "1.0"
anyhow = "1.0"

[features]
default = []
# WASM支持
wasm = ["wasmtime"]
# Python支持
python = ["pyo3", "candle-pyo3"]
# 完整支持
full = ["wasm", "python"]
```

### 2. 模块结构

```
src/
├── wasm/                          # WASM模块（新增）
│   ├── mod.rs
│   ├── sandbox.rs                # WASM沙箱
│   ├── runtime.rs                 # WASM运行时
│   ├── wasi.rs                    # WASI支持
│   ├── limits.rs                  # 资源限制
│   └── error.rs                   # WASM错误
├── python/                        # Python模块（新增）
│   ├── mod.rs
│   ├── executor.rs                # Python执行器
│   ├── runtime.rs                 # Python运行时
│   ├── pyo3_bridge.rs            # PyO3桥接
│   ├── model_loader.rs           # 模型加载器
│   ├── dependency_manager.rs     # 依赖管理
│   └── error.rs                  # Python错误
├── container/                    # 容器模块（扩展）
│   ├── mod.rs
│   ├── wasm_executor.rs          # 容器化WASM执行器（新增）
│   ├── algorithm_executor.rs     # 现有执行器
│   └── youki_manager.rs          # 现有管理器
├── core/                         # 核心模块（扩展）
│   ├── scheduler.rs              # 扩展支持Python任务
│   └── types.rs                  # 扩展类型定义
└── api/                          # API模块（扩展）
    ├── handlers.rs               # 扩展Python API
    └── routes.rs                 # 扩展路由
```

### 3. 核心实现

#### 3.1 WASM沙箱实现

```rust
// src/wasm/sandbox.rs

use wasmtime::{Engine, Store, Instance, Module, Linker};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder};

pub struct WasmSandbox {
    engine: Engine,
    store: Store<WasmContext>,
    instance: Option<Instance>,
    limits: ResourceLimits,
    config: SandboxConfig,
}

pub struct WasmContext {
    wasi: WasiCtx,
    memory_limit: usize,
    execution_timeout: Duration,
}

impl WasmSandbox {
    pub fn new(config: SandboxConfig) -> Result<Self> {
        // 创建WASM引擎
        let mut engine_config = wasmtime::Config::new();
        engine_config.wasm_multi_memory(true);
        engine_config.wasm_memory64(false);
        
        // 设置资源限制
        engine_config.max_wasm_stack(config.max_stack_size_mb * 1024 * 1024);
        
        let engine = Engine::new(&engine_config)?;
        
        // 创建WASI上下文
        let wasi_ctx = WasiCtxBuilder::new()
            .inherit_stdio()
            .inherit_args()?
            .build();
        
        let mut store = Store::new(
            &engine,
            WasmContext {
                wasi: wasi_ctx,
                memory_limit: config.max_memory_mb * 1024 * 1024,
                execution_timeout: config.max_execution_time,
            },
        );
        
        Ok(Self {
            engine,
            store,
            instance: None,
            limits: ResourceLimits::default(),
            config,
        })
    }
    
    pub async fn load_module(&mut self, wasm_bytes: &[u8]) -> Result<()> {
        // 编译WASM模块
        let module = Module::new(&self.engine, wasm_bytes)?;
        
        // 创建链接器
        let mut linker = Linker::new(&self.engine);
        wasmtime_wasi::add_to_linker(&mut linker, |ctx| &mut ctx.wasi)?;
        
        // 实例化模块
        let instance = linker.instantiate(&mut self.store, &module)?;
        
        self.instance = Some(instance);
        Ok(())
    }
    
    pub async fn call_function(
        &mut self,
        name: &str,
        args: &[WasmValue],
    ) -> Result<Vec<WasmValue>> {
        let instance = self.instance.as_ref()
            .ok_or("Module not loaded")?;
        
        let func = instance.get_func(&mut self.store, name)
            .ok_or_else(|| format!("Function {} not found", name))?;
        
        // 设置超时
        let timeout = self.config.max_execution_time;
        let result = tokio::time::timeout(timeout, async {
            // 调用函数
            func.call_async(&mut self.store, args, &mut []).await
        }).await??;
        
        Ok(result)
    }
}
```

#### 3.2 Python执行器实现

```rust
// src/python/executor.rs

use pyo3::prelude::*;
use pyo3::types::PyDict;

pub struct PythonExecutor {
    py_runtime: PyRuntime,
    wasm_sandbox: Option<Arc<WasmSandbox>>,
    model_loader: Arc<ModelLoader>,
    config: PythonExecutorConfig,
}

impl PythonExecutor {
    pub fn new(config: PythonExecutorConfig) -> Result<Self> {
        // 初始化Python运行时
        let py_runtime = PyRuntime::new()?;
        
        // 如果使用WASM沙箱，创建沙箱
        let wasm_sandbox = if config.use_wasm_sandbox {
            Some(Arc::new(WasmSandbox::new(
                SandboxConfig::default()
            )?))
        } else {
            None
        };
        
        // 初始化模型加载器
        let model_loader = Arc::new(ModelLoader::new()?);
        
        Ok(Self {
            py_runtime,
            wasm_sandbox,
            model_loader,
            config,
        })
    }
    
    pub async fn execute_code(
        &self,
        code: &str,
        inputs: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        Python::with_gil(|py| {
            // 创建输入字典
            let input_dict = PyDict::new(py);
            for (key, value) in inputs.as_object().unwrap() {
                let py_value = self.json_to_pyobject(py, value)?;
                input_dict.set_item(key, py_value)?;
            }
            
            // 设置全局变量
            let globals = PyDict::new(py);
            globals.set_item("inputs", input_dict)?;
            
            // 导入candle模块
            let candle = PyModule::import(py, "candle")?;
            globals.set_item("candle", candle)?;
            
            // 执行代码
            let result = py.run(code, Some(globals), None)?;
            
            // 获取结果
            let output = globals.get_item("output")
                .ok_or("No output variable found")?;
            
            // 转换为JSON
            let json_value = self.pyobject_to_json(py, output)?;
            Ok(json_value)
        })
    }
    
    pub async fn load_model(
        &self,
        model_path: &Path,
        model_type: ModelType,
    ) -> Result<PyModel> {
        Python::with_gil(|py| {
            match model_type {
                ModelType::Candle => {
                    // 使用candle-pyo3加载模型
                    let candle = PyModule::import(py, "candle")?;
                    let model = candle.call_method1(
                        "load_model",
                        (model_path.to_str().unwrap(),)
                    )?;
                    Ok(PyModel::new(model))
                }
                ModelType::Custom => {
                    // 加载自定义Python模型
                    let model_code = std::fs::read_to_string(model_path)?;
                    self.execute_code(&model_code, &serde_json::json!({}))?;
                    // ... 获取模型对象
                    Ok(PyModel::new(/* ... */))
                }
            }
        })
    }
}
```

#### 3.3 容器化集成

```rust
// src/container/wasm_executor.rs

pub struct ContainerizedWasmExecutor {
    container_manager: Arc<YoukiContainerManager>,
    wasm_runtime: Arc<WasmRuntime>,
    python_env: Arc<PythonEnvironment>,
}

impl ContainerizedWasmExecutor {
    pub async fn execute_python_algorithm(
        &self,
        request: ComputeRequest,
    ) -> Result<ComputeResponse> {
        // 1. 创建容器配置
        let container_config = ContainerConfig {
            image: "python-wasm:latest".to_string(),
            command: vec!["/usr/local/bin/wasm-python".to_string()],
            env: vec![
                "PYTHONPATH=/app".to_string(),
                "WASM_SANDBOX=true".to_string(),
            ],
            resources: ResourceRequirements {
                cpu_cores: 1.0,
                memory_mb: 512,
                disk_mb: 1024,
            },
            mounts: vec![
                MountPoint {
                    host_path: PathBuf::from("/models"),
                    container_path: PathBuf::from("/app/models"),
                    readonly: true,
                },
            ],
        };
        
        // 2. 创建容器
        let container = self.container_manager
            .create_container(container_config)
            .await?;
        
        // 3. 在容器中启动WASM运行时
        let wasm_sandbox = self.wasm_runtime
            .create_sandbox_in_container(&container)
            .await?;
        
        // 4. 加载Python代码
        let python_code = self.load_python_algorithm(&request.algorithm).await?;
        
        // 5. 在WASM沙箱中执行
        let result = wasm_sandbox
            .execute_python_code(&python_code, &request.parameters)
            .await?;
        
        // 6. 清理
        self.container_manager.delete_container(&container.id).await?;
        
        Ok(ComputeResponse {
            id: request.id,
            result: result,
            execution_time_ms: /* ... */,
        })
    }
}
```

### 4. API扩展

#### 4.1 Python API路由

```rust
// src/api/routes.rs

pub fn create_python_routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/python/algorithms", get(list_python_algorithms))
        .route("/api/v1/python/algorithms/:name", get(get_python_algorithm))
        .route("/api/v1/python/algorithms", post(register_python_algorithm))
        .route("/api/v1/python/execute", post(execute_python_code))
        .route("/api/v1/python/models", get(list_python_models))
        .route("/api/v1/python/models/:name", post(load_python_model))
        .route("/api/v1/python/dependencies", get(list_dependencies))
        .route("/api/v1/python/dependencies", post(install_dependency))
}
```

#### 4.2 Python API处理器

```rust
// src/api/python_handlers.rs

/// 执行Python代码
pub async fn execute_python_code(
    state: State<AppState>,
    Json(request): Json<PythonExecuteRequest>,
) -> Response {
    let executor = state.python_executor.clone();
    
    match executor.execute_code(&request.code, &request.inputs).await {
        Ok(result) => {
            (StatusCode::OK, Json(json!({
                "result": result,
                "status": "success"
            }))).into_response()
        }
        Err(e) => {
            (StatusCode::INTERNAL_SERVER_ERROR,
             Json(json!({"error": e.to_string()}))).into_response()
        }
    }
}

/// 注册Python算法
pub async fn register_python_algorithm(
    state: State<AppState>,
    Json(request): Json<PythonAlgorithmRequest>,
) -> Response {
    // 验证Python代码
    // 注册算法
    // 返回结果
}
```

### 5. 配置扩展

```toml
# config/default.toml

[python]
# 启用Python支持
enabled = true
# Python版本
version = "3.11"
# 使用WASM沙箱
use_wasm_sandbox = true
# Python路径
python_path = "/usr/local/bin/python3"

[python.wasm]
# WASM配置
enabled = true
# WASM运行时
runtime = "wasmtime"
# 最大内存（MB）
max_memory_mb = 512
# 最大执行时间（秒）
max_execution_time = 300
# 最大栈大小（MB）
max_stack_size_mb = 64

[python.dependencies]
# 依赖管理
manager = "pip"
# 依赖缓存目录
cache_dir = "./python_cache"
# 自动安装依赖
auto_install = true

[python.models]
# 模型目录
model_dir = "./python_models"
# 自动扫描
auto_scan = true
# 扫描间隔（秒）
scan_interval_seconds = 60
```

---

## 🚀 实施计划

### Phase 1: 基础WASM支持 (2-3周)

#### 1.1 WASM运行时集成
- [ ] 添加wasmtime依赖
- [ ] 实现WasmSandbox基础结构
- [ ] 实现WASI支持
- [ ] 实现资源限制

#### 1.2 测试验证
- [ ] 单元测试：WASM沙箱
- [ ] 集成测试：WASM模块加载和执行
- [ ] 性能测试：WASM执行性能

**交付物**:
- WASM沙箱基础功能
- WASI支持
- 资源限制功能

### Phase 2: Python集成 (3-4周)

#### 2.1 PyO3集成
- [ ] 集成candle-pyo3
- [ ] 实现PythonExecutor
- [ ] 实现PyO3桥接
- [ ] 实现模型加载器

#### 2.2 Python算法支持
- [ ] 实现Python代码执行
- [ ] 实现Python函数调用
- [ ] 实现依赖管理
- [ ] 实现模型加载

#### 2.3 测试验证
- [ ] 单元测试：Python执行器
- [ ] 集成测试：Python算法执行
- [ ] 兼容性测试：与Candle集成

**交付物**:
- Python执行引擎
- PyO3桥接
- 模型加载功能

### Phase 3: 容器化集成 (2-3周)

#### 3.1 容器化WASM
- [ ] 实现ContainerizedWasmExecutor
- [ ] 集成Youki容器
- [ ] 实现容器内WASM运行时
- [ ] 实现Python环境部署

#### 3.2 任务调度集成
- [ ] 扩展TaskScheduler支持Python任务
- [ ] 实现Python任务优先级
- [ ] 实现资源管理
- [ ] 实现错误处理

#### 3.3 API开发
- [ ] 实现Python API路由
- [ ] 实现Python API处理器
- [ ] 实现API文档
- [ ] 实现API测试

**交付物**:
- 容器化WASM执行器
- Python API接口
- 任务调度集成

### Phase 4: 优化与生产化 (3-4周)

#### 4.1 性能优化
- [ ] 实现WASM模块缓存
- [ ] 实现Python环境缓存
- [ ] 实现批处理执行
- [ ] 性能基准测试

#### 4.2 生产特性
- [ ] 实现安全隔离
- [ ] 实现监控和指标
- [ ] 实现日志记录
- [ ] 实现错误恢复

#### 4.3 文档和测试
- [ ] 编写集成文档
- [ ] 编写API文档
- [ ] 编写部署指南
- [ ] 完整测试套件

**交付物**:
- 生产就绪的Python WASM服务
- 完整文档
- 性能报告

---

## 🔒 安全考虑

### 1. WASM沙箱安全

- **内存隔离**: WASM提供内存隔离
- **系统调用限制**: 限制允许的系统调用
- **资源限制**: CPU、内存、执行时间限制
- **代码验证**: 验证WASM模块完整性

### 2. Python代码安全

- **代码审查**: 审查Python代码
- **依赖验证**: 验证Python依赖
- **沙箱执行**: 在WASM沙箱中执行
- **资源限制**: 限制Python资源使用

### 3. 容器安全

- **容器隔离**: 使用Youki容器隔离
- **权限控制**: 最小权限原则
- **网络隔离**: 限制网络访问
- **文件系统隔离**: 只读挂载

---

## 📊 性能优化

### 1. WASM优化

- **模块缓存**: 缓存编译后的WASM模块
- **预热**: 预热常用模块
- **并行执行**: 支持并行WASM执行
- **资源池**: 复用WASM运行时

### 2. Python优化

- **环境缓存**: 缓存Python环境
- **模块缓存**: 缓存Python模块
- **预加载**: 预加载常用模块
- **批处理**: 批量执行Python代码

### 性能目标

| 指标 | 目标值 |
|------|--------|
| WASM启动时间 | < 100ms |
| Python初始化时间 | < 500ms |
| 代码执行延迟 | < 50ms (简单代码) |
| 内存占用 | < 512MB (单实例) |
| 并发支持 | 10+ 并发任务 |

---

## 🧪 测试策略

### 1. 单元测试

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_wasm_sandbox() {
        let config = SandboxConfig::default();
        let mut sandbox = WasmSandbox::new(config).unwrap();
        // 测试WASM沙箱
    }
    
    #[tokio::test]
    async fn test_python_executor() {
        let executor = PythonExecutor::new(Default::default()).unwrap();
        // 测试Python执行器
    }
}
```

### 2. 集成测试

```rust
#[tokio::test]
async fn test_python_algorithm_execution() {
    // 启动测试服务器
    let app = create_test_app().await;
    
    // 测试Python算法执行
    let response = app.post("/api/v1/python/execute")
        .json(&python_request)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
}
```

### 3. 兼容性测试

- 测试与现有C++算法的兼容性
- 测试与Candle ML的兼容性
- 测试API向后兼容性
- 测试配置兼容性

---

## 📚 使用示例

### 1. Python算法注册

```python
# my_algorithm.py
import candle

def my_custom_model(input_data):
    # 使用Candle创建模型
    x = candle.Tensor(input_data)
    
    # 自定义模型逻辑
    # ...
    
    return result
```

```bash
curl -X POST http://localhost:3000/api/v1/python/algorithms \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $JWT_TOKEN" \
  -d '{
    "name": "my_custom_model",
    "code": "import candle\ndef my_custom_model(input_data):\n    ...",
    "dependencies": ["candle"],
    "resource_requirements": {
      "cpu_cores": 1.0,
      "memory_mb": 256
    }
  }'
```

### 2. Python算法执行

```bash
curl -X POST http://localhost:3000/api/v1/python/execute \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $JWT_TOKEN" \
  -d '{
    "algorithm": "my_custom_model",
    "inputs": {
      "data": [1.0, 2.0, 3.0]
    }
  }'
```

### 3. Rust代码示例

```rust
use rust_edge_compute::python::{PythonExecutor, PythonExecuteRequest};

#[tokio::main]
async fn main() -> Result<()> {
    // 创建Python执行器
    let executor = PythonExecutor::new(Default::default())?;
    
    // 执行Python代码
    let code = r#"
import candle

def process(inputs):
    x = candle.Tensor(inputs["data"])
    result = x * 2.0
    return {"output": result.tolist()}
    "#;
    
    let inputs = serde_json::json!({
        "data": [1.0, 2.0, 3.0]
    });
    
    let result = executor.execute_code(code, &inputs).await?;
    println!("Result: {}", result);
    
    Ok(())
}
```

---

## 🔄 兼容性保证

### 1. API兼容性

- 所有现有API保持不变
- 新增Python API不影响现有API
- 响应格式保持一致

### 2. 配置兼容性

- 现有配置继续有效
- 新增配置可选
- 配置向后兼容

### 3. 功能兼容性

- 现有C++算法继续工作
- 现有Candle ML算法继续工作
- 新增Python算法不影响现有功能

---

## 📋 检查清单

### 开发阶段

- [ ] WASM运行时集成完成
- [ ] Python执行器实现
- [ ] PyO3桥接完成
- [ ] 容器化集成完成
- [ ] 单元测试通过
- [ ] 集成测试通过

### 测试阶段

- [ ] 功能测试通过
- [ ] 性能测试通过
- [ ] 兼容性测试通过
- [ ] 安全测试通过

### 部署阶段

- [ ] 部署文档完成
- [ ] 监控配置完成
- [ ] 文档更新完成

---

## 🎯 成功标准

### 功能标准

- ✅ 支持Python自定义算法
- ✅ 支持WASM沙箱执行
- ✅ 支持PyO3 Candle集成
- ✅ 支持容器化部署
- ✅ 与现有系统兼容

### 性能标准

- ✅ WASM启动时间 < 100ms
- ✅ Python初始化 < 500ms
- ✅ 代码执行延迟 < 50ms
- ✅ 内存占用 < 512MB
- ✅ 支持10+并发任务

### 可靠性标准

- ✅ 错误率 < 0.1%
- ✅ 可用性 > 99.9%
- ✅ 向后兼容 100%

---

## 📞 支持和联系

### 文档资源

- [Wasmtime文档](https://docs.wasmtime.dev/)
- [PyO3文档](https://pyo3.rs/)
- [Candle PyO3文档](./candle/candle/candle-pyo3/README.md)

### 技术支持

- 问题反馈: GitHub Issues
- 技术讨论: 团队Slack频道
- 紧急支持: 联系项目负责人

---

**文档结束**

