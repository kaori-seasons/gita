//! ZeroMQ PUSH/PULL 模式 - 自动消费隔离
//!
//! 这个示例展示了如何使用 PUSH/PULL 套接字实现自动的消费隔离。
//! 每条消息只会被一个消费者接收，适合实现分布式任务处理。
//!
//! 使用方法:
//! 
//! 发布者（PUSH）:
//!   cargo run --features cpp --example zeromq_push_pull_pattern -- \
//!     --role publisher --port 5555 --count 10
//!
//! 订阅者（PULL）- 订阅者1:
//!   cargo run --features cpp --example zeromq_push_pull_pattern -- \
//!     --role subscriber --consumer-id consumer-1 --port 5555
//!
//! 订阅者（PULL）- 订阅者2:
//!   cargo run --features cpp --example zeromq_push_pull_pattern -- \
//!     --role subscriber --consumer-id consumer-2 --port 5555

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::time::{Duration, interval};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::io::AsyncWriteExt;

/// 消息结构
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DataMessage {
    id: u64,
    timestamp: u64,
    device_id: String,
    sensor_type: String,
    values: Vec<f64>,
    consumer_group: Option<String>,  // 消费组标记
    consumed_by: Option<String>,      // 消费者ID
}

impl DataMessage {
    fn new(id: u64) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let sensor_types = vec!["vibration", "temperature", "current", "pressure"];
        let sensor_type = sensor_types[(id as usize) % sensor_types.len()].to_string();

        Self {
            id,
            timestamp,
            device_id: "edge-device-001".to_string(),
            sensor_type,
            values: vec![
                (id as f64 % 10.0) + 1.0,
                (id as f64 % 10.0) + 2.0,
                (id as f64 % 10.0) + 3.0,
            ],
            consumer_group: None,
            consumed_by: None,
        }
    }

    fn to_json_line(&self) -> String {
        format!("{}\n", serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string()))
    }
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let role = get_arg(&args, "--role", "subscriber");

    match role.as_str() {
        "publisher" => publisher_mode(&args).await,
        "subscriber" => subscriber_mode(&args).await,
        _ => {
            eprintln!("❌ 未知角色: {}，使用 publisher 或 subscriber", role);
            std::process::exit(1);
        }
    }
}

fn get_arg(args: &[String], key: &str, default: &str) -> String {
    args.iter()
        .position(|arg| arg == key)
        .and_then(|pos| args.get(pos + 1).cloned())
        .unwrap_or_else(|| default.to_string())
}

async fn publisher_mode(args: &[String]) {
    println!("\n╔════════════════════════════════════════════════════════════════╗");
    println!("║       PUSH/PULL 模式 - 发布者（自动负载均衡）                  ║");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    let host = get_arg(args, "--host", "127.0.0.1");
    let port = get_arg(args, "--port", "5555").parse::<u16>().unwrap_or(5555);
    let count: u64 = get_arg(args, "--count", "0").parse().unwrap_or(0);
    let interval_ms: u64 = get_arg(args, "--interval", "1000").parse().unwrap_or(1000);

    let addr = format!("{}:{}", host, port);
    println!("📋 发布者配置:");
    println!("  监听地址: {}", addr);
    println!("  消息数量: {}", if count == 0 { "无限".to_string() } else { count.to_string() });
    println!("  发送间隔: {}ms", interval_ms);
    println!();

    let listener = match TcpListener::bind(&addr).await {
        Ok(l) => {
            println!("✅ 发布者已启动，等待订阅者连接...\n");
            l
        }
        Err(e) => {
            eprintln!("❌ 绑定失败: {}", e);
            std::process::exit(1);
        }
    };

    let mut message_id = 1u64;
    let mut sent_count = 0u64;
    let mut clients: Vec<TcpStream> = Vec::new();
    let mut ticker = interval(Duration::from_millis(interval_ms));

    println!("{:<6} {:<15} {:<12}", "序号", "客户端数", "已发送");
    println!("{}", "-".repeat(50));

    loop {
        tokio::select! {
            // 接受新客户端连接
            accept_result = listener.accept() => {
                if let Ok((stream, addr)) = accept_result {
                    println!("➕ 新客户端连接: {} (总计: {})", addr, clients.len() + 1);
                    clients.push(stream);
                }
            }

            // 每隔指定时间发送一条消息
            _ = ticker.tick() => {
                if count > 0 && sent_count >= count {
                    println!("\n✅ 已发送 {} 条消息，达到限制", sent_count);
                    break;
                }

                let mut msg = DataMessage::new(message_id);
                let json_line = msg.to_json_line();

                // 轮转发送给不同的客户端（负载均衡）
                if !clients.is_empty() {
                    let client_idx = (sent_count as usize) % clients.len();
                    
                    if let Err(e) = clients[client_idx].write_all(json_line.as_bytes()).await {
                        println!("⚠️  客户端 {} 已断开: {}", client_idx, e);
                        clients.remove(client_idx);
                        continue;
                    }

                    sent_count += 1;
                    message_id += 1;

                    println!("{:<6} {:<15} {:<12}", message_id - 1, clients.len(), sent_count);
                }
            }
        }
    }

    println!("\n╔════════════════════════════════════════════════════════════════╗");
    println!("║  发布者已停止                                                  ║");
    println!("╚════════════════════════════════════════════════════════════════╝");
}

async fn subscriber_mode(args: &[String]) {
    println!("\n╔════════════════════════════════════════════════════════════════╗");
    println!("║       PUSH/PULL 模式 - 订阅者（消费隔离）                      ║");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    let host = get_arg(args, "--host", "127.0.0.1");
    let port = get_arg(args, "--port", "5555").parse::<u16>().unwrap_or(5555);
    let consumer_id = get_arg(args, "--consumer-id", "consumer-unknown");

    let addr = format!("{}:{}", host, port);
    println!("📋 订阅者配置:");
    println!("  消费者ID: {}", consumer_id);
    println!("  连接地址: {}", addr);
    println!();

    let stream = match TcpStream::connect(&addr).await {
        Ok(s) => {
            println!("✅ 已连接到发布者\n");
            s
        }
        Err(e) => {
            eprintln!("❌ 连接失败: {}", e);
            std::process::exit(1);
        }
    };

    let (reader, _) = stream.into_split();
    let mut buf_reader = BufReader::new(reader);
    let mut line = String::new();
    let mut received_count = 0u64;

    println!("{:<6} {:<12} {:<20} {:<15}", "ID", "时间戳", "消费者", "传感器类型");
    println!("{}", "-".repeat(70));

    loop {
        line.clear();
        match buf_reader.read_line(&mut line).await {
            Ok(0) => {
                println!("\n✅ 连接已关闭，共接收 {} 条消息", received_count);
                break;
            }
            Ok(_) => {
                if let Ok(json) = serde_json::from_str::<DataMessage>(&line) {
                    received_count += 1;
                    println!(
                        "{:<6} {:<12} {:<20} {:<15}",
                        json.id, json.timestamp, consumer_id, json.sensor_type
                    );

                    if received_count % 10 == 0 {
                        println!("\n📊 已接收 {} 条消息 [{}]", received_count, consumer_id);
                    }
                }
            }
            Err(e) => {
                eprintln!("\n❌ 读取失败: {}", e);
                break;
            }
        }
    }

    println!("\n╔════════════════════════════════════════════════════════════════╗");
    println!("║  订阅者已断开 [{}]", consumer_id);
    println!("╚════════════════════════════════════════════════════════════════╝");
}
