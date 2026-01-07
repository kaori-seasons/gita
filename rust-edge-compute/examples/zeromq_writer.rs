//! 无限制数据发送进程 - 无消费者时丢弃消息
//!
//! 特点：
//! - 无限制发送（每秒一条）
//! - 如果没有消费者订阅，消息直接丢弃
//! - 支持多个消费者同时订阅
//!
//! 使用方法:
//!   cargo run --features cpp --example zeromq_writer -- --host 127.0.0.1 --port 5555
//!
//! 参数说明:
//!   --host <addr>     绑定地址（默认: 127.0.0.1）
//!   --port <port>     绑定端口（默认: 5555）
//!   --interval <ms>   发送间隔毫秒数（默认: 1000）

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::time::{Duration, interval};
use tokio::net::{TcpListener, TcpStream};
use tokio::io::AsyncWriteExt;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 数据消息结构 - 真实特征数据
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DataMessage {
    #[serde(rename = "feature")]
    feature: FeatureData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FeatureData {
    /// 设备唯一标识
    device_id: String,
    std: u32,
    meanLf: u64,
    customFeature: String,
    peakPowers: String,
    uuid: String,
    version: u32,
    feature2: String,
    extend: ExtendData,
    feature3: String,
    feature4: String,
    peakFreqs: String,
    mean: u64,
    feature1: String,
    bandSpectrum: String,
    temperature: u32,
    time: u64,
    nodeId: u32,
    meanHf: u64,
    seq: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExtendData {
    SerialData: String,
}

impl DataMessage {
    /// 生成新的数据消息
    fn new(id: u64, device_id: String, _sensor_type: String) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        // 构造真实的特征数据
        let feature = FeatureData {
            device_id: device_id.clone(),
            std: 657 + (id as u32 % 100),
            meanLf: 509550917,
            customFeature: "".to_string(),
            peakPowers: "13.82,14.05,13.93,14.45,14.85,15.39,14.94,14.92,14.69,14.97,14.74,14.7,14.63,14.28,14.24,14.16,13.79,13.87,13.67,13.56".to_string(),
            uuid: format!("0201B544{}DD-25", id % 1000),
            version: 0,
            feature2: "19.88,20.12,20.03,20.16,19.98,20.1,20.31,19.98,20.02,20.18,20.05,20.18,20.09,19.93,19.86,19.95,20.09,20.13,19.79,20.12,20.04,20.06,19.96,19.99".to_string(),
            extend: ExtendData {
                SerialData: r#"{"SerialData":""}"#.to_string(),
            },
            feature3: "16.57,16.54,16.54,16.62,16.49,16.46,16.54,16.41,16.46,16.52,16.46,16.52,16.44,16.51,16.39,16.48,16.54,16.49,16.42,16.58,16.52,16.42,16.41,16.38".to_string(),
            feature4: "6.64,6.58,6.63,6.54,6.5,6.48,6.59,6.33,6.48,6.55,6.48,6.49,6.41,6.42,6.33,6.46,6.48,6.53,6.37,6.57,6.52,6.41,6.4,6.36".to_string(),
            peakFreqs: "4.25,5.29,5.83,6.07,6.33,6.47,6.64,6.72,6.83,6.94,7.07,7.17,7.23,7.29,7.36,7.43,7.48,7.56,7.61,7.69".to_string(),
            mean: 14507460,
            feature1: "23.82,23.73,23.72,23.74,23.66,23.66,23.68,23.6,23.64,23.71,23.65,23.69,23.64,23.79,23.7,23.73,23.74,23.69,23.63,23.74,23.73,23.62,23.6,23.63".to_string(),
            bandSpectrum: "19.07,18.56,17.42,14.83,12.37,12.53,11.94,11.52,11.37,11.29,11.4,11.92,12.72,11.61,11.06,10.98,10.99,10.95,10.97,10.99".to_string(),
            temperature: 0,
            time: timestamp,
            nodeId: 2809,
            meanHf: 19450335060,
            seq: (id % 1000) as u32,
        };

        Self { feature }
    }

    /// 转换为 JSON 字符串（带换行符作为消息分隔符）
    fn to_json_line(&self) -> String {
        let json = serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string());
        format!("{}\n", json)
    }
}

#[tokio::main]
async fn main() {
    println!("\n╔═══════════════════════════════════════════════════════════════╗");
    println!("║      无限制数据发送 - 无消费者时丢弃消息                       ║");
    println!("╚═══════════════════════════════════════════════════════════════╝\n");

    // 解析命令行参数
    let mut host = "127.0.0.1".to_string();
    let mut port = 5555u16;
    let mut interval_ms: u64 = 1000; // 默认一秒

    let args: Vec<String> = std::env::args().collect();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--host" => {
                if i + 1 < args.len() {
                    host = args[i + 1].clone();
                    i += 2;
                } else {
                    eprintln!("❌ --host 需要一个值");
                    std::process::exit(1);
                }
            }
            "--port" => {
                if i + 1 < args.len() {
                    port = args[i + 1].parse().unwrap_or(5555);
                    i += 2;
                } else {
                    eprintln!("❌ --port 需要一个值");
                    std::process::exit(1);
                }
            }
            "--interval" => {
                if i + 1 < args.len() {
                    interval_ms = args[i + 1].parse().unwrap_or(1000);
                    i += 2;
                } else {
                    eprintln!("❌ --interval 需要一个值");
                    std::process::exit(1);
                }
            }
            _ => {
                eprintln!("❌ 未知的参数: {}", args[i]);
                std::process::exit(1);
            }
        }
    }

    let server_addr = format!("{}:{}", host, port);

    // 打印配置信息
    println!("📋 配置信息:");
    println!("  绑定地址: {}", server_addr);
    println!("  发送间隔: {}ms", interval_ms);
    println!("  模式: 无限制发送 + 无消费者时丢弃");
    println!();

    // 创建监听器
    println!("🔌 正在绑定到 {}...", server_addr);
    let listener = match TcpListener::bind(&server_addr).await {
        Ok(l) => {
            println!("✅ 绑定成功！等待订阅者连接...");
            l
        }
        Err(e) => {
            eprintln!("❌ 绑定失败: {}", e);
            std::process::exit(1);
        }
    };

    // 使用 Arc<RwLock> 存储客户端连接
    let clients: Arc<RwLock<Vec<TcpStream>>> = Arc::new(RwLock::new(Vec::new()));
    let clients_clone = clients.clone();

    // 启动客户端接收任务
    tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    println!("➕ 新订阅者连接: {}", addr);
                    let mut clients_lock = clients_clone.write().await;
                    clients_lock.push(stream);
                    println!("   当前订阅者数: {}", clients_lock.len());
                }
                Err(e) => {
                    eprintln!("❌ 接受连接失败: {}", e);
                }
            }
        }
    });

    println!("\n🚀 开始发送数据...\n");
    println!("{:<8} {:<20} {:<25} {:<15}", "ID", "UUID", "设备ID", "传感器");
    println!("{}", "-".repeat(75));

    let mut message_id: u64 = 1;
    let mut sent_count: u64 = 0;
    let mut discarded_count: u64 = 0;
    let device_id = "edge-device-001".to_string();
    let mut ticker = interval(Duration::from_millis(interval_ms));
    
    // 定义三种传感器类型
    let sensor_types = vec!["vibration", "temperature", "current"];

    loop {
        ticker.tick().await;

        // 生成数据消息（循环选择三种传感器之一）
        let sensor_type = sensor_types[(message_id as usize) % 3].to_string();
        let message = DataMessage::new(message_id, device_id.clone(), sensor_type.clone());
        let json_line = message.to_json_line();

        // 获取当前订阅者数
        let mut clients_lock = clients.write().await;
        let subscriber_count = clients_lock.len();

        if subscriber_count == 0 {
            // 没有订阅者，丢弃消息
            discarded_count += 1;
            println!(
                "{:<8} {:<20} {:<25} {:<15} [丢弃]",
                message_id, message.feature.uuid, message.feature.device_id, sensor_type
            );
        } else {
            // 有订阅者，广播消息给所有订阅者
            let mut disconnected_indices = Vec::new();
            for (idx, stream) in clients_lock.iter_mut().enumerate() {
                if let Err(_) = stream.write_all(json_line.as_bytes()).await {
                    disconnected_indices.push(idx);
                }
            }

            // 移除断开连接的客户端
            for idx in disconnected_indices.iter().rev() {
                clients_lock.remove(*idx);
                println!("➖ 订阅者已断开连接");
            }

            if disconnected_indices.is_empty() {
                sent_count += 1;
                println!(
                    "{:<8} {:<20} {:<25} {:<15} {}",
                    message_id, message.feature.uuid, message.feature.device_id, sensor_type, subscriber_count
                );
            }
        }

        message_id += 1;

        // 每20条消息打印一次统计
        if (sent_count + discarded_count) % 20 == 0 && (sent_count + discarded_count) > 0 {
            println!("\n📊 统计: 已发送 {} 条，已丢弃 {} 条，订阅者数: {}", sent_count, discarded_count, subscriber_count);
        }
    }
}
