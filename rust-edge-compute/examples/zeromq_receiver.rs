//! 数据接收进程 - 接收一秒一条数据
//!
//! 使用方法:
//!   cargo run --example zeromq_receiver -- --host 127.0.0.1 --port 5555
//!
//! 参数说明:
//!   --host <addr>     监听地址（默认: 127.0.0.1）
//!   --port <port>     监听端口（默认: 5555）

use tokio::net::TcpListener;
use tokio::io::{AsyncBufReadExt, BufReader};
use serde_json::Value;

#[tokio::main]
async fn main() {
    println!("\n╔═══════════════════════════════════════════════════════════════╗");
    println!("║        数据接收进程 - 接收一秒一条数据                         ║");
    println!("╚═══════════════════════════════════════════════════════════════╝\n");

    // 解析命令行参数
    let mut host = "127.0.0.1".to_string();
    let mut port = 5555u16;

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
            _ => {
                eprintln!("❌ 未知的参数: {}", args[i]);
                std::process::exit(1);
            }
        }
    }

    let server_addr = format!("{}:{}", host, port);

    // 打印配置信息
    println!("📋 配置信息:");
    println!("  监听地址: {}", server_addr);
    println!();

    // 创建监听器
    println!("🔌 正在创建监听器...");
    let listener = match TcpListener::bind(&server_addr).await {
        Ok(l) => {
            println!("✅ 监听器创建成功，等待连接...");
            l
        }
        Err(e) => {
            eprintln!("❌ 创建监听器失败: {}", e);
            std::process::exit(1);
        }
    };

    loop {
        // 等待客户端连接
        let (socket, addr) = match listener.accept().await {
            Ok((s, a)) => (s, a),
            Err(e) => {
                eprintln!("❌ 接受连接失败: {}", e);
                continue;
            }
        };

        println!("\n✅ 客户端已连接: {}\n", addr);
        println!("{:<6} {:<12} {:<15} {:<20} {:<50}", "ID", "时间戳", "设备ID", "传感器类型", "数据值");
        println!("{}", "-".repeat(110));

        // 处理客户端
        tokio::spawn(async move {
            let (reader, _) = socket.into_split();
            let mut buf_reader = BufReader::new(reader);
            let mut line = String::new();
            let mut message_count = 0u64;

            loop {
                line.clear();
                match buf_reader.read_line(&mut line).await {
                    Ok(0) => {
                        println!("\n✅ 客户端已断开连接 ({}:{}), 共接收 {} 条消息\n", addr.ip(), addr.port(), message_count);
                        break;
                    }
                    Ok(_) => {
                        message_count += 1;
                        
                        // 解析 JSON
                        if let Ok(json) = serde_json::from_str::<Value>(&line) {
                            let id = json["id"].as_u64().unwrap_or(0);
                            let timestamp = json["timestamp"].as_u64().unwrap_or(0);
                            let device_id = json["device_id"].as_str().unwrap_or("unknown");
                            let sensor_type = json["sensor_type"].as_str().unwrap_or("unknown");
                            
                            let values_str = if let Some(values) = json["values"].as_array() {
                                values
                                    .iter()
                                    .filter_map(|v| v.as_f64())
                                    .map(|v| format!("{:.2}", v))
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            } else {
                                "[]".to_string()
                            };

                            println!(
                                "{:<6} {:<12} {:<15} {:<20} [{}]",
                                id, timestamp, device_id, sensor_type, values_str
                            );

                            // 每10条消息打印一次统计
                            if message_count % 10 == 0 {
                                println!("\n📊 已接收 {} 条消息", message_count);
                            }
                        } else {
                            eprintln!("⚠️  收到无效的 JSON: {}", line.trim());
                        }
                    }
                    Err(e) => {
                        eprintln!("\n❌ 读取数据失败: {}", e);
                        break;
                    }
                }
            }
        });
    }
}
