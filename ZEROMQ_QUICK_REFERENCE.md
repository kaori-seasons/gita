# ZeroMQ 写入进程快速参考

## 🚀 一键启动

### 启动接收服务器
```bash
cd /Users/windwheel/Documents/gitrepo/gita
cargo run --features cpp --example zeromq_receiver
```

### 启动写入进程
```bash
cd /Users/windwheel/Documents/gitrepo/gita
cargo run --features cpp --example zeromq_writer -- --count 10 --interval 1000
```

## 📊 常用命令

| 功能 | 命令 |
|------|------|
| 发送10条消息 | `--count 10` |
| 改变发送间隔 | `--interval 500` （毫秒）|
| 连接不同主机 | `--host 192.168.1.100` |
| 使用不同端口 | `--port 8080` |
| 持续发送 | 省略 `--count` 参数 |

## 📝 参数组合示例

```bash
# 快速发送100条消息，每条间隔100ms
cargo run --features cpp --example zeromq_writer -- --count 100 --interval 100

# 慢速发送5条消息，每条间隔2秒
cargo run --features cpp --example zeromq_writer -- --count 5 --interval 2000

# 连接到远程服务器
cargo run --features cpp --example zeromq_writer -- \
  --host 10.0.0.1 \
  --port 5555 \
  --count 1000

# 监听所有IP地址
cargo run --features cpp --example zeromq_receiver -- --host 0.0.0.0 --port 5555
```

## 📂 源代码位置

```
项目根目录: /Users/windwheel/Documents/gitrepo/gita/

写入器源代码:
  rust-edge-compute/examples/zeromq_writer.rs

接收器源代码:
  rust-edge-compute/examples/zeromq_receiver.rs

完整文档:
  ZEROMQ_WRITER_GUIDE.md
```

## 🔍 故障排除

| 问题 | 解决方案 |
|------|---------|
| Connection refused | 确保接收器已启动 |
| Port already in use | 使用不同的端口，或 `lsof -i :5555` 查看占用进程 |
| 消息接收不完整 | 检查网络连接，减少发送间隔 |

## 💡 技术栈

- **语言**: Rust
- **异步运行时**: Tokio
- **序列化**: serde_json
- **传输协议**: TCP

## ✅ 已验证

- ✅ 编译通过
- ✅ 接收器能正确监听
- ✅ 写入器能正确连接
- ✅ 消息格式正确
- ✅ 时间间隔准确
