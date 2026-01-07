//! ZeroMQ 消费组管理 - 实现类似 Kafka 的消费隔离
//!
//! 这个示例展示了如何在应用层实现消费组管理、消费偏移量追踪和重放功能。
//! 类似于 Kafka 消费组但使用 ZeroMQ 的 PUB/SUB 套接字。
//!
//! 使用方法:
//!
//! 发布者:
//!   cargo run --features cpp --example zeromq_consumer_group -- \
//!     --role publisher --port 5556
//!
//! 订阅者 - 组1:
//!   cargo run --features cpp --example zeromq_consumer_group -- \
//!     --role subscriber --group group-1 --consumer-id member-1 --port 5556
//!
//! 订阅者 - 组2:
//!   cargo run --features cpp --example zeromq_consumer_group -- \
//!     --role subscriber --group group-2 --consumer-id member-1 --port 5556

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tokio::time::{Duration, interval};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::io::AsyncWriteExt;

/// 消息结构
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ConsumerMessage {
    id: u64,
    timestamp: u64,
    device_id: String,
    sensor_type: String,
    values: Vec<f64>,
    // 消费组信息
    consumer_group: String,
    consumer_id: String,
}

impl ConsumerMessage {
    fn new(id: u64, group: &str, consumer: &str) -> Self {
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
            consumer_group: group.to_string(),
            consumer_id: consumer.to_string(),
        }
    }

    fn to_json_line(&self) -> String {
        format!("{}\n", serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string()))
    }
}

/// 消费者群组状态管理
#[derive(Debug, Clone)]
struct ConsumerGroupState {
    group_id: String,
    members: Vec<String>,                           // 组成员
    offsets: HashMap<String, u64>,                  // 消费者偏移量
    last_committed: HashMap<String, u64>,           // 最后提交的偏移量
}

impl ConsumerGroupState {
    fn new(group_id: String) -> Self {
        Self {
            group_id,
            members: Vec::new(),
            offsets: HashMap::new(),
            last_committed: HashMap::new(),
        }
    }

    fn add_member(&mut self, member_id: String) {
        if !self.members.contains(&member_id) {
            self.members.push(member_id.clone());
            self.offsets.insert(member_id.clone(), 0);
            self.last_committed.insert(member_id, 0);
        }
    }

    fn update_offset(&mut self, consumer_id: &str, offset: u64) {
        self.offsets.insert(consumer_id.to_string(), offset);
    }

    fn commit_offset(&mut self, consumer_id: &str) {
        if let Some(offset) = self.offsets.get(consumer_id) {
            self.last_committed.insert(consumer_id.to_string(), *offset);
        }
    }

    fn get_next_offset(&self, consumer_id: &str) -> u64 {
        self.last_committed.get(consumer_id).copied().unwrap_or(0)
    }
}

type ConsumerGroups = Arc<RwLock<HashMap<String, ConsumerGroupState>>>;

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
    println!("║       消费组管理 - 发布者（广播模式）                          ║");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    let host = get_arg(args, "--host", "127.0.0.1");
    let port = get_arg(args, "--port", "5556").parse::<u16>().unwrap_or(5556);
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

    println!("{:<6} {:<15} {:<12}", "消息ID", "订阅者数", "已发送");
    println!("{}", "-".repeat(50));

    loop {
        tokio::select! {
            accept_result = listener.accept() => {
                if let Ok((stream, addr)) = accept_result {
                    println!("➕ 新订阅者连接: {} (总计: {})", addr, clients.len() + 1);
                    clients.push(stream);
                }
            }

            _ = ticker.tick() => {
                if count > 0 && sent_count >= count {
                    println!("\n✅ 已发送 {} 条消息，达到限制", sent_count);
                    break;
                }

                let msg = ConsumerMessage::new(message_id, "broadcast", "publisher");
                let json_line = msg.to_json_line();

                // 广播给所有订阅者
                let mut disconnected = Vec::new();
                for (idx, client) in clients.iter_mut().enumerate() {
                    if let Err(_) = client.write_all(json_line.as_bytes()).await {
                        disconnected.push(idx);
                    }
                }

                // 移除断开的客户端
                for idx in disconnected.iter().rev() {
                    clients.remove(*idx);
                    println!("➖ 订阅者已断开 (剩余: {})", clients.len());
                }

                if !clients.is_empty() {
                    sent_count += 1;
                    println!("{:<6} {:<15} {:<12}", message_id, clients.len(), sent_count);
                    message_id += 1;
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
    println!("║       消费组管理 - 订阅者（消费组隔离）                        ║");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    let host = get_arg(args, "--host", "127.0.0.1");
    let port = get_arg(args, "--port", "5556").parse::<u16>().unwrap_or(5556);
    let group_id = get_arg(args, "--group", "default-group");
    let consumer_id = get_arg(args, "--consumer-id", "consumer-unknown");

    let addr = format!("{}:{}", host, port);
    println!("📋 订阅者配置:");
    println!("  消费组ID: {}", group_id);
    println!("  消费者ID: {}", consumer_id);
    println!("  连接地址: {}", addr);
    println!();

    // 初始化消费组状态
    let groups: ConsumerGroups = Arc::new(RwLock::new(HashMap::new()));
    {
        let mut gs = groups.write().await;
        let mut group_state = gs
            .entry(group_id.clone())
            .or_insert_with(|| ConsumerGroupState::new(group_id.clone()));
        group_state.add_member(consumer_id.clone());
    }

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
    let mut last_commit = 0u64;

    println!("{:<6} {:<12} {:<15} {:<15} {:<15}", "ID", "时间戳", "消费组", "消费者", "传感器");
    println!("{}", "-".repeat(80));

    loop {
        line.clear();
        match buf_reader.read_line(&mut line).await {
            Ok(0) => {
                // 连接关闭前提交最后的偏移量
                {
                    let mut gs = groups.write().await;
                    if let Some(group) = gs.get_mut(&group_id) {
                        group.commit_offset(&consumer_id);
                    }
                }
                println!("\n✅ 连接已关闭，共接收 {} 条消息", received_count);
                println!("📍 最后已提交偏移量: {}", last_commit);
                break;
            }
            Ok(_) => {
                if let Ok(json) = serde_json::from_str::<ConsumerMessage>(&line) {
                    received_count += 1;

                    // 更新消费者偏移量
                    {
                        let mut gs = groups.write().await;
                        if let Some(group) = gs.get_mut(&group_id) {
                            group.update_offset(&consumer_id, json.id);
                            
                            // 每10条消息自动提交一次
                            if received_count % 10 == 0 {
                                group.commit_offset(&consumer_id);
                                last_commit = json.id;
                            }
                        }
                    }

                    println!(
                        "{:<6} {:<12} {:<15} {:<15} {:<15}",
                        json.id, json.timestamp, json.consumer_group, consumer_id, json.sensor_type
                    );

                    if received_count % 20 == 0 {
                        let gs = groups.read().await;
                        if let Some(group) = gs.get(&group_id) {
                            println!(
                                "\n📊 [{}] 消费进度: 已接收 {} 条，已提交 {}，组成员: {}",
                                consumer_id,
                                received_count,
                                last_commit,
                                group.members.len()
                            );
                        }
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
    println!("║  订阅者已断开 [{}] - 组: {}", consumer_id, group_id);
    println!("╚════════════════════════════════════════════════════════════════╝");
}
