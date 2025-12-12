//! ZeroMQ 模拟数据生产器
//!
//! 持续生成模拟的传感器数据并通过ZeroMQ发布
//! 可用于测试下游的数据处理流程

use std::time::{SystemTime, UNIX_EPOCH};
use tokio::time::{Duration, interval};
use serde::{Deserialize, Serialize};

/// ZeroMQ消息结构（模拟）
#[derive(Debug, Clone, Serialize, Deserialize)]
struct MockZeroMQMessage {
    measurement_point_id: String,
    sequence: u64,
    timestamp: u64,
    sensor_type: String,
    values: Vec<f64>,
}

/// 模拟 ZeroMQ 数据源（Publisher）
struct MockZeroMQPublisher {
    device_id: String,
    sequence_counter: u64,
}

impl MockZeroMQPublisher {
    fn new(device_id: String) -> Self {
        Self {
            device_id,
            sequence_counter: 0,
        }
    }

    /// 生成一批传感器消息（5个传感器）
    fn generate_sensor_messages(&mut self) -> Vec<MockZeroMQMessage> {
        let mut messages = Vec::new();
        
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        // 振动传感器 X轴
        messages.push(MockZeroMQMessage {
            measurement_point_id: format!("{}-vibration-x", self.device_id),
            sequence: self.sequence_counter,
            timestamp,
            sensor_type: "vibration_x".to_string(),
            values: vec![1.2, 1.5, 1.3, 1.6, 1.4],
        });

        // 振动传感器 Y轴
        messages.push(MockZeroMQMessage {
            measurement_point_id: format!("{}-vibration-y", self.device_id),
            sequence: self.sequence_counter,
            timestamp,
            sensor_type: "vibration_y".to_string(),
            values: vec![2.1, 2.4, 2.2, 2.5, 2.3],
        });

        // 振动传感器 Z轴
        messages.push(MockZeroMQMessage {
            measurement_point_id: format!("{}-vibration-z", self.device_id),
            sequence: self.sequence_counter,
            timestamp,
            sensor_type: "vibration_z".to_string(),
            values: vec![5.3, 6.2, 5.8, 6.5, 6.1],
        });

        // 温度传感器
        messages.push(MockZeroMQMessage {
            measurement_point_id: format!("{}-temperature", self.device_id),
            sequence: self.sequence_counter,
            timestamp,
            sensor_type: "temperature".to_string(),
            values: vec![65.5, 65.8, 66.2, 65.9, 66.1],
        });

        // 电流传感器
        messages.push(MockZeroMQMessage {
            measurement_point_id: format!("{}-current", self.device_id),
            sequence: self.sequence_counter,
            timestamp,
            sensor_type: "current".to_string(),
            values: vec![45.2, 45.5, 45.3, 45.6, 45.4],
        });

        self.sequence_counter += 1;
        messages
    }
}

#[tokio::main]
async fn main() {
    println!("\n╔════════════════════════════════════════════════════════════════╗");
    println!("║      ZeroMQ 模拟数据生产器 - 传感器数据发布                   ║");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    let device_id = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "edge-device-001".to_string());
    
    let interval_ms: u64 = std::env::args()
        .nth(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(100);

    let max_batches: usize = std::env::args()
        .nth(3)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0); // 0 表示无限循环

    println!("📊 配置信息:");
    println!("  设备ID: {}", device_id);
    println!("  发送间隔: {}ms", interval_ms);
    println!("  批次限制: {}", if max_batches == 0 { "无限".to_string() } else { max_batches.to_string() });
    println!();

    let mut publisher = MockZeroMQPublisher::new(device_id.clone());
    let mut ticker = interval(Duration::from_millis(interval_ms));
    let mut batch_count = 0;

    println!("🚀 开始生成传感器数据...\n");
    println!("{:<10} {:<20} {:<15} {:<30}", "批次", "序列号", "时间戳", "传感器类型");
    println!("{}", "-".repeat(80));

    loop {
        ticker.tick().await;

        let messages = publisher.generate_sensor_messages();
        batch_count += 1;

        // 打印消息信息
        for msg in &messages {
            println!("{:<10} {:<20} {:<15} {:<30}", 
                batch_count,
                msg.sequence,
                msg.timestamp,
                msg.sensor_type
            );
        }

        // 序列化为JSON并打印（可选）
        if std::env::var("ZEROMQ_VERBOSE").is_ok() {
            println!("\n📦 批次 {} JSON数据:", batch_count);
            for msg in &messages {
                println!("{}", serde_json::to_string_pretty(&msg).unwrap());
            }
            println!();
        }

        // 检查是否达到最大批次
        if max_batches > 0 && batch_count >= max_batches {
            println!("\n✅ 已完成 {} 批次数据生成", batch_count);
            break;
        }

        // 每10批打印一次统计
        if batch_count % 10 == 0 {
            println!("\n📈 已生成 {} 批次，{} 条消息", batch_count, batch_count * 5);
        }
    }

    println!("\n╔════════════════════════════════════════════════════════════════╗");
    println!("║  ZeroMQ数据生产器已停止                                        ║");
    println!("╚════════════════════════════════════════════════════════════════╝");
}
