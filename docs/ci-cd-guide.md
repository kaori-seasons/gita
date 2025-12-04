# CI/CD 流水线使用指南

本文档介绍如何使用基于 [rust-ci](https://gitlab.com/rust-ci/rust-ci) 项目的高效 CI/CD 流水线。

## 📋 目录

- [概述](#概述)
- [流水线阶段](#流水线阶段)
- [配置说明](#配置说明)
- [使用方法](#使用方法)
- [优化建议](#优化建议)
- [故障排除](#故障排除)

## 概述

本项目使用 GitLab CI/CD 进行持续集成和持续部署。流水线配置基于 [rust-ci](https://gitlab.com/rust-ci/rust-ci) 项目的最佳实践，并针对多 crate workspace 项目进行了优化。

### 主要特性

- ✅ **并行执行**：多个 crate 的测试和构建并行执行，提高效率
- ✅ **智能缓存**：缓存 Cargo 依赖和编译产物，加速后续构建
- ✅ **增量检查**：只对变更的文件运行相关检查
- ✅ **多阶段验证**：格式检查、代码检查、测试、构建、打包
- ✅ **覆盖率报告**：自动生成测试覆盖率报告
- ✅ **发布管理**：自动打包和部署

## 流水线阶段

### 1. Validate（验证阶段）

验证项目配置和依赖的完整性。

- **validate:cargo-toml**：验证 Cargo.toml 格式
- **validate:dependencies**：验证依赖完整性

### 2. Lint（代码检查阶段）

检查代码质量和格式。

- **lint:fmt**：检查代码格式（`cargo fmt --check`）
- **lint:clippy**：运行 Clippy 代码检查
- **lint:clippy:***：针对各个 crate 的并行 Clippy 检查

### 3. Test（测试阶段）

运行各种测试。

- **test:all**：运行所有测试
- **test:unit:***：针对各个 crate 的单元测试（并行执行）
- **test:integration**：集成测试
- **test:doc**：文档测试

### 4. Build（构建阶段）

编译项目。

- **build:debug**：Debug 模式构建
- **build:release**：Release 模式构建（针对边缘部署优化）
- **build:release:***：针对各个 crate 的 Release 构建（并行执行）
- **build:release:binary**：构建主程序二进制文件

### 5. Package（打包阶段）

创建发布包。

- **package:release**：创建压缩包，包含所有二进制文件和库文件

### 6. Deploy（部署阶段）

部署到不同环境（可选）。

- **deploy:staging**：部署到测试环境
- **deploy:production**：部署到生产环境

## 配置说明

### 环境变量

流水线使用以下环境变量：

```yaml
RUST_VERSION: "stable"              # Rust 工具链版本
CARGO_HOME: "${CI_PROJECT_DIR}/.cargo"  # Cargo 主目录
CARGO_BUILD_JOBS: "4"                # 并行编译线程数
CARGO_INCREMENTAL: "0"               # 禁用增量编译
```

### 缓存配置

流水线配置了多层缓存以加速构建：

- **Cargo 注册表缓存**：缓存下载的依赖包
- **Cargo Git 缓存**：缓存 Git 依赖
- **Target 缓存**：缓存编译产物

### 触发规则

- **所有阶段**：默认在代码变更时触发
- **Release 构建**：仅在 main/master 分支和 tags 时触发
- **打包**：仅在 tags 和 main/master 分支时触发
- **部署**：手动触发（`when: manual`）

### 跳过 CI

如果提交信息包含 `[skip ci]` 或 `[ci skip]`，流水线将不会运行。

## 使用方法

### 1. 基本使用

将 `.gitlab-ci.yml` 文件提交到仓库后，GitLab 会自动检测并运行流水线。

```bash
# 提交代码
git add .gitlab-ci.yml
git commit -m "Add CI/CD pipeline"
git push origin main
```

### 2. 查看流水线状态

1. 在 GitLab 项目中，点击左侧边栏的 **CI/CD** > **流水线**
2. 查看流水线状态和各个作业的执行情况
3. 点击作业名称查看详细日志

### 3. 手动触发部署

1. 在流水线页面，找到 `deploy:staging` 或 `deploy:production` 作业
2. 点击作业右侧的 **▶️** 按钮手动触发

### 4. 查看测试覆盖率

1. 在流水线页面，找到 `test:all` 作业
2. 查看作业详情中的覆盖率报告

### 5. 下载构建产物

1. 在流水线页面，找到构建作业（如 `build:release`）
2. 点击 **浏览** 按钮下载构建产物

## 优化建议

### 1. 使用共享 Runner

如果项目使用 GitLab.com 的共享 Runner，建议：

- 配置项目专用的 Runner 以获得更好的性能
- 使用自托管的 Runner 以节省 CI 分钟数

### 2. 调整并行度

根据 Runner 的资源情况，调整 `CARGO_BUILD_JOBS`：

```yaml
variables:
  CARGO_BUILD_JOBS: "8"  # 根据 Runner CPU 核心数调整
```

### 3. 优化缓存策略

- 使用 `key: ${CI_COMMIT_REF_SLUG}` 为不同分支创建独立缓存
- 定期清理过期缓存

### 4. 使用 Docker 镜像缓存

如果使用自定义 Docker 镜像，建议：

- 在 Dockerfile 中使用多阶段构建
- 将依赖安装和代码编译分离
- 使用 Docker 层缓存

### 5. 条件执行

根据项目需求，可以添加条件执行规则：

```yaml
only:
  changes:
    - "rust-edge-compute-core/**/*"
    - "Cargo.toml"
```

## 故障排除

### 1. 流水线失败

**问题**：流水线在某个作业失败

**解决方案**：
- 查看作业日志，找到错误信息
- 检查代码是否有语法错误
- 检查依赖是否正确配置
- 检查 Runner 是否有足够的资源

### 2. 缓存问题

**问题**：缓存导致构建问题

**解决方案**：
- 清除缓存：在 CI/CD 设置中清除缓存
- 更新缓存键：修改 `cache.key` 强制刷新缓存

### 3. 构建超时

**问题**：构建作业超时

**解决方案**：
- 增加作业超时时间
- 优化构建配置（减少并行度、禁用不必要的特性）
- 使用更强大的 Runner

### 4. 依赖下载失败

**问题**：依赖下载超时或失败

**解决方案**：
- 检查网络连接
- 使用国内镜像源（如清华大学镜像）
- 增加重试次数

### 5. Clippy 警告

**问题**：Clippy 检查失败

**解决方案**：
- 修复 Clippy 警告
- 如果警告是误报，可以使用 `#[allow(clippy::warning_name)]` 忽略
- 临时允许失败：设置 `allow_failure: true`

## 高级配置

### 1. 多平台构建

如果需要构建多个平台的二进制文件，可以添加：

```yaml
build:release:linux:
  script:
    - cargo build --target x86_64-unknown-linux-gnu --release

build:release:windows:
  script:
    - cargo build --target x86_64-pc-windows-msvc --release
```

### 2. 性能基准测试

添加性能基准测试：

```yaml
benchmark:
  stage: test
  script:
    - cargo bench --workspace
  only:
    - main
    - master
```

### 3. 安全扫描

添加安全扫描：

```yaml
security:audit:
  stage: validate
  script:
    - cargo audit
  allow_failure: true
```

### 4. 代码覆盖率

配置代码覆盖率报告：

```yaml
test:coverage:
  stage: test
  script:
    - cargo test --workspace --all-features
    - cargo llvm-cov --workspace --lcov --output-path lcov.info
  coverage: '/^test result:.*\s+(\d+\.\d+)%.*$/'
  artifacts:
    reports:
      coverage_report:
        coverage_format: cobertura
        path: lcov.info
```

## 参考资源

- [rust-ci 项目](https://gitlab.com/rust-ci/rust-ci)
- [GitLab CI/CD 文档](https://docs.gitlab.com/ee/ci/)
- [Cargo 文档](https://doc.rust-lang.org/cargo/)
- [Rust 工具链文档](https://rust-lang.github.io/rustup/)

## 更新日志

- **2024-01-XX**：初始版本，基于 rust-ci 项目创建
- 支持多 crate workspace 并行构建
- 优化缓存策略
- 添加部署阶段

