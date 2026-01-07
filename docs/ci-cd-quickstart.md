# CI/CD 快速开始指南

## 🚀 5分钟快速上手

### 第一步：确保 GitLab Runner 可用

1. 进入 GitLab 项目
2. 点击 **设置** > **CI/CD**
3. 展开 **Runners** 部分
4. 确保至少有一个 Runner 处于活跃状态（绿色圆圈）

如果没有 Runner，请：
- 使用 GitLab.com 的共享 Runner（自动可用）
- 或安装自托管 Runner（推荐用于大型项目）

### 第二步：提交 CI/CD 配置

```bash
# 添加 CI/CD 配置文件
git add .gitlab-ci.yml
git commit -m "Add CI/CD pipeline"
git push origin main
```

### 第三步：查看流水线

1. 点击左侧边栏的 **CI/CD** > **流水线**
2. 等待流水线运行完成
3. 查看各个作业的状态

## 📊 流水线概览

```
┌─────────────┐
│   Validate  │ 检查配置和依赖
└──────┬──────┘
       │
┌──────▼──────┐
│    Lint     │ 代码格式和检查
└──────┬──────┘
       │
┌──────▼──────┐
│    Test     │ 运行测试
└──────┬──────┘
       │
┌──────▼──────┐
│    Build    │ 编译项目
└──────┬──────┘
       │
┌──────▼──────┐
│   Package   │ 创建发布包
└──────┬──────┘
       │
┌──────▼──────┐
│   Deploy    │ 部署（可选）
└─────────────┘
```

## 🎯 常用操作

### 查看测试结果

```bash
# 在 GitLab 界面中
CI/CD > 流水线 > [选择流水线] > test:all
```

### 下载构建产物

```bash
# 在 GitLab 界面中
CI/CD > 流水线 > [选择流水线] > build:release > 浏览
```

### 手动触发部署

```bash
# 在 GitLab 界面中
CI/CD > 流水线 > [选择流水线] > deploy:staging > ▶️
```

### 跳过 CI（紧急情况）

```bash
git commit -m "[skip ci] 紧急修复"
git push
```

## ⚙️ 自定义配置

### 修改 Rust 版本

编辑 `.gitlab-ci.yml`：

```yaml
variables:
  RUST_VERSION: "1.75.0"  # 指定版本
```

### 调整并行度

```yaml
variables:
  CARGO_BUILD_JOBS: "8"  # 根据 Runner CPU 核心数调整
```

### 添加环境变量

在 GitLab 项目中：
1. **设置** > **CI/CD** > **变量**
2. 添加变量（如 `API_KEY`）
3. 在 `.gitlab-ci.yml` 中使用 `${API_KEY}`

## 🔍 故障排查

### 流水线失败？

1. **查看日志**：点击失败的作业查看详细错误
2. **检查代码**：确保代码可以本地编译
3. **检查依赖**：运行 `cargo check` 验证依赖

### 构建太慢？

1. **启用缓存**：确保缓存配置正确
2. **使用更快的 Runner**：考虑使用自托管 Runner
3. **减少并行度**：如果 Runner 资源有限

### 测试失败？

1. **本地运行**：`cargo test` 验证测试
2. **检查环境**：确保测试环境配置正确
3. **查看日志**：检查测试输出

## 📚 更多信息

- 详细文档：[CI/CD 使用指南](ci-cd-guide.md)
- rust-ci 项目：https://gitlab.com/rust-ci/rust-ci
- GitLab CI 文档：https://docs.gitlab.com/ee/ci/

## 💡 最佳实践

1. **小步提交**：频繁提交，便于定位问题
2. **查看日志**：定期检查 CI/CD 日志
3. **优化缓存**：合理使用缓存加速构建
4. **并行执行**：利用并行作业提高效率
5. **及时修复**：修复失败的作业，保持流水线健康

