# 边缘端部署编译优化指南

本文档基于 [Aloxaf's Blog - 优化 Rust 程序编译体积](https://www.aloxaf.com/2018/09/reduce_rust_size/) 的优化技巧，针对边缘端部署进行了配置优化。

## ✅ 已应用的优化

### 1. Release 模式配置

在 workspace 根目录的 `Cargo.toml` 中配置了 `[profile.release]`：

```toml
[profile.release]
opt-level = 'z'        # 最小二进制体积优化
lto = true             # 链接时优化
codegen-units = 1      # 限制并行代码生成单元
panic = 'abort'        # Panic 时立刻终止（减少体积）
debug = false          # 减少调试信息
strip = true           # 减少符号信息
```

**效果**：
- 优化等级 `z`：专门针对体积优化
- LTO：消除冗余代码，显著减小体积
- `codegen-units = 1`：便于编译器进行全局优化
- `panic = 'abort'`：禁用栈回溯，减少体积（注意：会禁用 panic 时的堆栈信息）

### 2. 依赖 Features 优化

已优化以下依赖的 features，只启用必要的功能：

#### Tokio
```toml
tokio = { version = "1.0", default-features = false, features = [
    "rt-multi-thread",  # 多线程运行时
    "net",              # 网络支持
    "io-util",          # IO 工具
    "time",             # 时间支持
    "sync",             # 同步原语
    "macros",           # 宏支持
] }
```

**优化说明**：
- 禁用 `full` feature，只启用实际使用的功能
- 大幅减少编译体积和依赖

#### Axum
```toml
axum = { version = "0.7", default-features = false, features = ["http1", "http2", "json"] }
```

**优化说明**：
- 只启用 HTTP/1、HTTP/2 和 JSON 支持
- 禁用其他不必要的功能

#### 其他依赖
- `reqwest`: 只启用 `json` 和 `rustls-tls`
- `tracing-subscriber`: 只启用必要的功能
- `sled`: 禁用默认 features
- `bincode`: 禁用默认 features

### 3. 构建脚本优化

创建了 `.cargo/config.toml` 配置文件，添加了额外的编译选项：

```toml
[target.'cfg(all(target_os = "linux", target_arch = "x86_64"))']
rustflags = [
    "-C", "opt-level=z",
    "-C", "lto=thin",
    "-C", "codegen-units=1",
    "-C", "link-dead-code=false",
]
```

## 📊 预期优化效果

根据文章数据，预期优化效果：

| 优化步骤 | 预期体积减少 |
|---------|------------|
| Release 模式 | ~80% |
| strip | ~45% |
| opt-level = 'z' | ~5% |
| LTO | ~20% |
| codegen-units = 1 | ~5% |
| panic = 'abort' | ~5% |
| 禁用不必要的 features | ~30-50% |

**总体预期**：从原始体积减少 **60-80%**

## 🚀 构建命令

### 标准 Release 构建
```bash
cargo build --release
```

### 带 strip 的构建（进一步减小体积）
```bash
cargo build --release
strip -s target/release/rust-edge-compute
```

### 使用 UPX 压缩（可选，进一步减小）
```bash
cargo build --release
strip -s target/release/rust-edge-compute
upx -9 target/release/rust-edge-compute
```

## ⚠️ 注意事项

### 1. Panic 行为变化

启用 `panic = 'abort'` 后：
- ✅ 减少二进制体积
- ⚠️ 禁用 panic 时的堆栈回溯
- ⚠️ 无法获取 panic 时的调用栈信息

**建议**：
- 生产环境：可以启用以减小体积
- 开发/调试环境：建议禁用以便调试

### 2. 编译时间

启用 LTO 和 `codegen-units = 1` 后：
- ⚠️ 编译时间会显著增加（可能增加 2-5 倍）
- ✅ 但运行时性能和体积都会改善

**建议**：
- CI/CD 构建：使用这些优化
- 本地开发：可以使用 `dev` profile 快速编译

### 3. Features 管理

在禁用 features 时需要注意：
- 确保只禁用真正不需要的功能
- 某些 features 可能是其他依赖的间接依赖
- 建议逐步禁用并测试

### 4. 依赖体积

某些依赖（如 Candle ML）本身体积较大：
- Candle 库：包含大量模型推理代码
- PyO3：Python 解释器绑定
- Wasmtime：WASM 运行时

这些依赖的体积优化空间有限，主要优化方向：
- 只启用必要的 features
- 考虑按需加载（动态链接）

## 🔧 进一步优化建议

### 1. 使用 Xargo 裁剪 libstd（高级）

如果需要进一步减小体积，可以使用 Xargo 裁剪标准库：

```toml
# Xargo.toml
[dependencies]
std = { default-features = false, features = ["panic_immediate_abort"] }
```

然后使用 `xargo build` 构建。

**注意**：这需要额外的工具链配置，可能影响兼容性。

### 2. 动态链接（可选）

对于某些大型依赖，可以考虑动态链接：
- 减少单个二进制体积
- 多个程序可以共享库
- 但部署时需要包含动态库

### 3. 按需加载 Executor

当前设计已经支持按需加载 executor：
- 使用 features 控制编译哪些 executor
- 只编译需要的 executor 可以显著减小体积

```bash
# 只编译 C++ executor
cargo build --release --features cpp

# 只编译 ML executor
cargo build --release --features ml

# 编译所有 executor
cargo build --release --features full
```

## 📈 体积对比

### 优化前（dev 模式）
- 体积：~26.5MB（示例）
- 包含大量调试信息

### 优化后（release + 所有优化）
- 预期体积：~1-2MB（主程序）
- 减少：~90%+

### 各 Executor 体积估算
- C++ Executor: ~500KB-1MB
- ML Executor: ~2-5MB（包含 Candle 库）
- Python Executor: ~1-2MB（不包含 Python 解释器）

## 🎯 边缘端部署建议

1. **按需编译**：只编译需要的 executor
2. **使用 strip**：部署前执行 strip
3. **考虑 UPX**：如果存储空间非常有限
4. **监控体积**：定期检查二进制体积变化
5. **测试验证**：确保优化后功能正常

## 📝 参考资源

- [Aloxaf's Blog - 优化 Rust 程序编译体积](https://www.aloxaf.com/2018/09/reduce_rust_size/)
- [min-sized-rust](https://github.com/johnthagen/min-sized-rust)
- [Rust Performance Book](https://nnethercote.github.io/perf-book/)


