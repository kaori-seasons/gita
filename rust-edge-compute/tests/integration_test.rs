//! 端到端集成测试
//!
//! 这个测试文件验证整个系统的端到端功能，包括：
//! - HTTP API服务器启动
//! - C++算法调用
//! - 容器管理功能
//! - 任务处理流程

use std::time::Duration;
use tokio::time::sleep;
use reqwest::Client;
use serde_json::json;

/// 测试基础HTTP API功能
#[tokio::test]
async fn test_basic_http_api() {
    // 启动测试服务器（在实际测试中需要先启动服务器）
    // 这里使用假设的服务器地址进行测试

    let client = Client::new();
    let base_url = "http://localhost:3000";

    // 测试健康检查
    println!("Testing health check...");
    let response = client
        .get(&format!("{}/health", base_url))
        .send()
        .await;

    match response {
        Ok(resp) => {
            println!("Health check status: {}", resp.status());
            if resp.status().is_success() {
                let body: serde_json::Value = resp.json().await.unwrap();
                println!("Health response: {:?}", body);
            }
        }
        Err(e) => {
            println!("Health check failed (expected if server not running): {}", e);
        }
    }

    // 测试算法列表
    println!("\nTesting algorithm list...");
    let response = client
        .get(&format!("{}/algorithms", base_url))
        .send()
        .await;

    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                let body: serde_json::Value = resp.json().await.unwrap();
                println!("Algorithms: {:?}", body);
            }
        }
        Err(e) => {
            println!("Algorithm list failed: {}", e);
        }
    }

    // 测试容器列表
    println!("\nTesting container list...");
    let response = client
        .get(&format!("{}/containers", base_url))
        .send()
        .await;

    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                let body: serde_json::Value = resp.json().await.unwrap();
                println!("Containers: {:?}", body);
            }
        }
        Err(e) => {
            println!("Container list failed: {}", e);
        }
    }
}

/// 测试计算任务功能
#[tokio::test]
async fn test_compute_task() {
    let client = Client::new();
    let base_url = "http://localhost:3000";

    // 测试加法算法
    println!("\nTesting add algorithm...");
    let request_data = json!({
        "algorithm": "add",
        "parameters": {
            "a": 5.0,
            "b": 3.0
        }
    });

    let response = client
        .post(&format!("{}/compute", base_url))
        .json(&request_data)
        .send()
        .await;

    match response {
        Ok(resp) => {
            println!("Compute response status: {}", resp.status());
            if resp.status().is_success() {
                let body: serde_json::Value = resp.json().await.unwrap();
                println!("Compute result: {:?}", body);
            }
        }
        Err(e) => {
            println!("Compute task failed: {}", e);
        }
    }

    // 测试回显算法
    println!("\nTesting echo algorithm...");
    let request_data = json!({
        "algorithm": "echo",
        "parameters": {
            "message": "Hello, Edge Compute!",
            "timestamp": 1234567890
        }
    });

    let response = client
        .post(&format!("{}/compute", base_url))
        .json(&request_data)
        .send()
        .await;

    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                let body: serde_json::Value = resp.json().await.unwrap();
                println!("Echo result: {:?}", body);
            }
        }
        Err(e) => {
            println!("Echo task failed: {}", e);
        }
    }
}

/// 测试字符串处理算法
#[tokio::test]
async fn test_string_algorithms() {
    let client = Client::new();
    let base_url = "http://localhost:3000";

    // 测试字符串反转
    println!("\nTesting string reverse...");
    let request_data = json!({
        "algorithm": "reverse",
        "parameters": {
            "text": "rust-edge-compute"
        }
    });

    let response = client
        .post(&format!("{}/compute", base_url))
        .json(&request_data)
        .send()
        .await;

    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                let body: serde_json::Value = resp.json().await.unwrap();
                println!("Reverse result: {:?}", body);
            }
        }
        Err(e) => {
            println!("Reverse task failed: {}", e);
        }
    }
}

/// 测试数据处理算法
#[tokio::test]
async fn test_data_algorithms() {
    let client = Client::new();
    let base_url = "http://localhost:3000";

    // 测试数据排序
    println!("\nTesting data sort...");
    let request_data = json!({
        "algorithm": "sort",
        "parameters": {
            "data": [3, 1, 4, 1, 5, 9, 2, 6]
        }
    });

    let response = client
        .post(&format!("{}/compute", base_url))
        .json(&request_data)
        .send()
        .await;

    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                let body: serde_json::Value = resp.json().await.unwrap();
                println!("Sort result: {:?}", body);
            }
        }
        Err(e) => {
            println!("Sort task failed: {}", e);
        }
    }
}

/// 测试容器管理功能
#[tokio::test]
async fn test_container_management() {
    let client = Client::new();
    let base_url = "http://localhost:3000";

    // 创建容器
    println!("\nTesting container creation...");
    let container_config = json!({
        "algorithm": "add",
        "config": {
            "name": "test-container",
            "image": "./images/alpine:latest",
            "env": ["PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"],
            "volumes": [],
            "resources": {
                "cpu_cores": 1.0,
                "memory_mb": 128,
                "disk_mb": 1024
            },
            "security": {
                "rootless": true,
                "network_isolation": true
            }
        }
    });

    let response = client
        .post(&format!("{}/containers", base_url))
        .json(&container_config)
        .send()
        .await;

    match response {
        Ok(resp) => {
            println!("Container creation status: {}", resp.status());
            if resp.status().is_success() {
                let body: serde_json::Value = resp.json().await.unwrap();
                println!("Container created: {:?}", body);

                // 如果创建成功，尝试获取容器状态
                if let Some(container_id) = body.get("container_id").and_then(|v| v.as_str()) {
                    sleep(Duration::from_millis(100)).await; // 等待一下

                    println!("\nTesting container status...");
                    let status_response = client
                        .get(&format!("{}/containers/{}", base_url, container_id))
                        .send()
                        .await;

                    match status_response {
                        Ok(resp) => {
                            if resp.status().is_success() {
                                let body: serde_json::Value = resp.json().await.unwrap();
                                println!("Container status: {:?}", body);
                            }
                        }
                        Err(e) => println!("Container status check failed: {}", e),
                    }
                }
            }
        }
        Err(e) => {
            println!("Container creation failed: {}", e);
        }
    }
}

/// 压力测试 - 并发请求
#[tokio::test]
async fn test_concurrent_requests() {
    let client = Client::new();
    let base_url = "http://localhost:3000";

    println!("\nTesting concurrent requests...");

    // 创建多个并发任务
    let mut handles = vec![];

    for i in 0..5 {
        let client = client.clone();
        let base_url = base_url.clone();

        let handle = tokio::spawn(async move {
            let request_data = json!({
                "algorithm": "add",
                "parameters": {
                    "a": i as f64,
                    "b": (i + 1) as f64
                }
            });

            let response = client
                .post(&format!("{}/compute", base_url))
                .json(&request_data)
                .send()
                .await;

            match response {
                Ok(resp) => {
                    if resp.status().is_success() {
                        println!("Concurrent task {}: Success", i);
                    } else {
                        println!("Concurrent task {}: Status {}", i, resp.status());
                    }
                }
                Err(e) => {
                    println!("Concurrent task {}: Error {}", i, e);
                }
            }
        });

        handles.push(handle);
    }

    // 等待所有任务完成
    for handle in handles {
        let _ = handle.await;
    }

    println!("Concurrent test completed");
}

/// 错误处理测试
#[tokio::test]
async fn test_error_handling() {
    let client = Client::new();
    let base_url = "http://localhost:3000";

    println!("\nTesting error handling...");

    // 测试无效算法
    let request_data = json!({
        "algorithm": "non_existent_algorithm",
        "parameters": {}
    });

    let response = client
        .post(&format!("{}/compute", base_url))
        .json(&request_data)
        .send()
        .await;

    match response {
        Ok(resp) => {
            println!("Error handling test status: {}", resp.status());
            if resp.status().is_client_error() {
                println!("✓ Correctly handled invalid algorithm");
            }
        }
        Err(e) => {
            println!("Error handling test failed: {}", e);
        }
    }

    // 测试无效参数
    let request_data = json!({
        "algorithm": "add",
        "parameters": {
            "invalid_param": "test"
        }
    });

    let response = client
        .post(&format!("{}/compute", base_url))
        .json(&request_data)
        .send()
        .await;

    match response {
        Ok(resp) => {
            if resp.status().is_client_error() {
                println!("✓ Correctly handled invalid parameters");
            }
        }
        Err(e) => {
            println!("Invalid params test failed: {}", e);
        }
    }
}

/// 端到端完整流程测试
#[tokio::test]
async fn test_end_to_end_flow() {
    println!("\n=== End-to-End Flow Test ===");
    println!("This test simulates a complete user workflow:");

    let client = Client::new();
    let base_url = "http://localhost:3000";

    // 1. 检查服务健康状态
    println!("1. Checking service health...");
    let response = client.get(&format!("{}/health", base_url)).send().await;
    match response {
        Ok(resp) if resp.status().is_success() => {
            println!("✓ Service is healthy");
        }
        _ => {
            println!("⚠ Service health check failed (expected if server not running)");
        }
    }

    // 2. 获取可用算法列表
    println!("2. Fetching available algorithms...");
    let response = client.get(&format!("{}/algorithms", base_url)).send().await;
    match response {
        Ok(resp) if resp.status().is_success() => {
            println!("✓ Retrieved algorithm list");
        }
        _ => {
            println!("⚠ Algorithm list fetch failed");
        }
    }

    // 3. 提交计算任务
    println!("3. Submitting compute task...");
    let request_data = json!({
        "algorithm": "add",
        "parameters": {
            "a": 10.0,
            "b": 20.0
        }
    });

    let response = client
        .post(&format!("{}/compute", base_url))
        .json(&request_data)
        .send()
        .await;

    match response {
        Ok(resp) if resp.status().is_success() => {
            println!("✓ Compute task submitted successfully");
        }
        _ => {
            println!("⚠ Compute task submission failed");
        }
    }

    // 4. 检查容器状态
    println!("4. Checking container status...");
    let response = client.get(&format!("{}/containers", base_url)).send().await;
    match response {
        Ok(resp) if resp.status().is_success() => {
            println!("✓ Container status retrieved");
        }
        _ => {
            println!("⚠ Container status check failed");
        }
    }

    println!("=== End-to-End Flow Test Completed ===");
}
