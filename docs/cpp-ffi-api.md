# C++ FFI API 端点文档

本文档描述了 Rust Edge Compute Framework 中的 C++ FFI（外部函数接口）API 端点。

## 概述

通过这些 API 端点，可以直接调用 C++ 实现的算法执行器，执行各种计算任务。

## 端点列表

### 1. 执行 C++ 算法

**端点**: `POST /api/v1/cpp/algorithms/execute`

执行指定的 C++ 算法。

#### 请求体

```json
{
  "algorithm_name": "vibrate31",
  "parameters": {
    "threshold": 0.5,
    "window_size": 128
  },
  "device_id": "device_001"
}
```

**参数说明:**
- `algorithm_name` (string, 必需): 要执行的算法名称
- `parameters` (object, 可选): 算法参数，作为 JSON 对象传递
- `device_id` (string, 可选): 目标设备ID，默认为 "default_device"

#### 响应

**成功响应 (200 OK)**:
```json
{
  "success": true,
  "algorithm": "vibrate31",
  "device_id": "device_001",
  "result": {
    "status": "processed",
    "value": 123.45
  },
  "error_message": "",
  "execution_time_ms": 45,
  "memory_used_bytes": 1024000
}
```

**失败响应 (500 Internal Server Error)**:
```json
{
  "success": false,
  "algorithm": "vibrate31",
  "error": "Algorithm execution failed: ..."
}
```

#### 示例 cURL

```bash
curl -X POST http://localhost:3000/api/v1/cpp/algorithms/execute \
  -H "Content-Type: application/json" \
  -d '{
    "algorithm_name": "vibrate31",
    "parameters": {"threshold": 0.5},
    "device_id": "device_001"
  }'
```

### 2. 列出可用算法

**端点**: `GET /api/v1/cpp/algorithms`

获取所有可用的 C++ 算法列表。

#### 请求参数

无

#### 响应

**成功响应 (200 OK)**:
```json
{
  "algorithms": [
    "vibrate31",
    "add",
    "multiply",
    "reverse",
    "sort"
  ],
  "count": 5,
  "source": "cpp_plugins"
}
```

#### 示例 cURL

```bash
curl http://localhost:3000/api/v1/cpp/algorithms
```

### 3. 获取算法信息

**端点**: `GET /api/v1/cpp/algorithms/{algorithm_name}`

获取指定算法的详细信息。

#### 请求参数

- `algorithm_name` (string, 路径参数, 必需): 算法名称

#### 响应

**成功响应 (200 OK)**:
```json
{
  "name": "vibrate31",
  "version": "1.0.0",
  "description": "Vibration analysis algorithm",
  "input_type": "float_array",
  "output_type": "json",
  "parameters": [
    {
      "name": "threshold",
      "type": "float",
      "required": true,
      "default": 0.5
    }
  ]
}
```

**失败响应 (404 Not Found)**:
```json
{
  "error": "C++ algorithm 'unknown_algo' not found"
}
```

#### 示例 cURL

```bash
curl http://localhost:3000/api/v1/cpp/algorithms/vibrate31
```

## 错误处理

所有错误响应都遵循以下格式:

```json
{
  "error": "Error message describing what went wrong",
  "details": "Optional additional details"
}
```

常见的 HTTP 状态码:
- `200 OK` - 请求成功
- `400 Bad Request` - 请求格式错误或参数无效
- `404 Not Found` - 算法未找到
- `500 Internal Server Error` - 服务器内部错误
- `503 Service Unavailable` - 执行器未初始化

## 集成示例

### Python 示例

```python
import requests
import json

# 基础URL
base_url = "http://localhost:3000/api/v1"

# 列出可用算法
response = requests.get(f"{base_url}/cpp/algorithms")
algorithms = response.json()['algorithms']
print(f"Available algorithms: {algorithms}")

# 执行算法
execute_url = f"{base_url}/cpp/algorithms/execute"
payload = {
    "algorithm_name": "vibrate31",
    "parameters": {
        "threshold": 0.5,
        "window_size": 128
    },
    "device_id": "edge_device_001"
}

response = requests.post(execute_url, json=payload)
result = response.json()

if result['success']:
    print(f"Execution successful: {result['result']}")
    print(f"Execution time: {result['execution_time_ms']}ms")
    print(f"Memory used: {result['memory_used_bytes']} bytes")
else:
    print(f"Execution failed: {result['error']}")
```

### Rust 示例

```rust
use reqwest::Client;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    
    // 执行算法
    let response = client
        .post("http://localhost:3000/api/v1/cpp/algorithms/execute")
        .json(&json!({
            "algorithm_name": "vibrate31",
            "parameters": {
                "threshold": 0.5
            },
            "device_id": "device_001"
        }))
        .send()
        .await?;

    let result: serde_json::Value = response.json().await?;
    println!("Result: {}", result);
    
    Ok(())
}
```

### JavaScript/Node.js 示例

```javascript
const axios = require('axios');

const baseUrl = 'http://localhost:3000/api/v1';

async function executeCppAlgorithm() {
  try {
    const response = await axios.post(
      `${baseUrl}/cpp/algorithms/execute`,
      {
        algorithm_name: 'vibrate31',
        parameters: {
          threshold: 0.5,
          window_size: 128
        },
        device_id: 'device_001'
      }
    );

    console.log('Execution result:', response.data);
    if (response.data.success) {
      console.log(`Time: ${response.data.execution_time_ms}ms`);
      console.log(`Memory: ${response.data.memory_used_bytes} bytes`);
    }
  } catch (error) {
    console.error('Error executing algorithm:', error.message);
  }
}

executeCppAlgorithm();
```

## 性能考虑

- **并发限制**: 系统的并发执行能力由任务调度器和可用资源决定
- **超时**: 根据系统配置，长时间运行的算法可能会超时
- **内存**: 监控 `memory_used_bytes` 以确保内存使用在预期范围内
- **执行时间**: 使用 `execution_time_ms` 监控算法性能

## 认证和授权

这些端点受到应用级认证中间件的保护。需要有效的认证令牌才能访问。

## 限流

API 受到速率限制，防止滥用。限流规则由 `rate_limit_middleware` 管理。

## 相关文档

- [主要 API 文档](../docs/api.md)
- [FFI 架构设计](../docs/ffi-architecture.md)
- [C++ 插件开发指南](../docs/cpp-plugin-development.md)
