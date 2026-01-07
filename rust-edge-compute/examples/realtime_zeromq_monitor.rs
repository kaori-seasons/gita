//! 实时 ZeroMQ 消息监控程序
//! 
//! 功能：
//! - 从 ZeroMQ 数据生产者实时订阅消息
//! - 执行完整的 DAG 数据流转换
//! - 调用真实的 C++ 插件链 (vibrate31 → error18 → evaluation)
//! - 实时监控内存使用情况
//! - 每 10 条消息打印一次性能报告
//! 
//! 使用方法：
//! ```bash
//! # 终端 1: 启动数据生产者
//! cargo run --features cpp --example zeromq_writer -- --port 5555 --interval 500
//! 
//! # 终端 2: 启动实时监控
//! DYLD_LIBRARY_PATH=./cpp_plugins/install/lib \
//! cargo run --features cpp --example realtime_zeromq_monitor -- --host 127.0.0.1 --port 5555
//! ```

use std::collections::HashMap;
use std::time::Instant;
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::mpsc;
use tokio::time::Duration;
use tokio::net::TcpStream;
use tokio::io::{AsyncBufReadExt, BufReader};
use clap::Parser;

// 导入真实的FFI bridge
use rust_edge_compute::ffi::bridge::CppAlgorithmExecutor;

// ============================================================================
// 命令行参数
// ============================================================================

#[derive(Parser, Debug)]
#[clap(name = "realtime_zeromq_monitor")]
#[clap(about = "实时 ZeroMQ 消息监控与 C++ 插件处理", long_about = None)]
struct Args {
    /// ZeroMQ 生产者主机地址
    #[clap(long, default_value = "127.0.0.1")]
    host: String,

    /// ZeroMQ 生产者端口
    #[clap(long, default_value = "5555")]
    port: u16,

    /// 内存报告间隔（每 N 条消息打印一次）
    #[clap(long, default_value = "10")]
    report_interval: u64,
}

// ============================================================================
// 内存监控模块
// ============================================================================

/// 插件内存使用情况统计
#[derive(Debug, Clone)]
struct PluginMemoryStats {
    plugin_name: String,
    call_count: u64,
    peak_memory_kb: f64,
    last_memory_kb: f64,
    // 记录每次调用前后的内存样本
    memory_samples: Vec<f64>,
}

/// 实时内存监控器
#[derive(Debug, Clone)]
struct RealtimeMemoryMonitor {
    stats: Arc<Mutex<HashMap<String, PluginMemoryStats>>>,
    process_start_memory_kb: f64,
    monitoring_start: Instant,
}

impl RealtimeMemoryMonitor {
    fn new() -> Self {
        let start_memory = Self::get_process_memory_kb();
        Self {
            stats: Arc::new(Mutex::new(HashMap::new())),
            process_start_memory_kb: start_memory,
            monitoring_start: Instant::now(),
        }
    }

    /// 获取当前进程内存使用量 (KB)
    fn get_process_memory_kb() -> f64 {
        #[cfg(target_os = "macos")]
        {
            use std::process::Command;
            let output = Command::new("ps")
                .args(&["-o", "rss=", "-p", &std::process::id().to_string()])
                .output();
            
            if let Ok(output) = output {
                if let Ok(s) = String::from_utf8(output.stdout) {
                    return s.trim().parse::<f64>().unwrap_or(0.0);
                }
            }
        }
        
        #[cfg(target_os = "linux")]
        {
            use std::fs;
            if let Ok(status) = fs::read_to_string("/proc/self/status") {
                for line in status.lines() {
                    if line.starts_with("VmRSS:") {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 2 {
                            return parts[1].parse::<f64>().unwrap_or(0.0);
                        }
                    }
                }
            }
        }
        
        0.0
    }

    /// 记录插件执行前后的内存变化
    fn record_plugin_execution(&self, plugin_name: &str, memory_before_kb: f64, memory_after_kb: f64) {
        let memory_used = memory_after_kb - memory_before_kb;

        let mut stats = self.stats.lock().unwrap();
        let entry = stats.entry(plugin_name.to_string()).or_insert(PluginMemoryStats {
            plugin_name: plugin_name.to_string(),
            call_count: 0,
            peak_memory_kb: 0.0,
            last_memory_kb: 0.0,
            memory_samples: Vec::new(),
        });

        entry.call_count += 1;
        entry.last_memory_kb = memory_used;
        entry.peak_memory_kb = entry.peak_memory_kb.max(memory_used);
        
        // 保留最近 100 次的样本用于计算平均值
        entry.memory_samples.push(memory_used);
        if entry.memory_samples.len() > 100 {
            entry.memory_samples.remove(0);
        }
    }

    /// 打印实时内存报告
    fn print_realtime_report(&self, processed_count: u64) {
        let stats = self.stats.lock().unwrap();
        let elapsed = self.monitoring_start.elapsed().as_secs();
        let current_memory = Self::get_process_memory_kb();
        let total_memory_growth = current_memory - self.process_start_memory_kb;

        println!("\n┌─────────────────────────────────────────────────────────────────┐");
        println!("│  📊 实时内存监控报告 (运行时长: {}s, 已处理: {} 条消息)", elapsed, processed_count);
        println!("├─────────────────────────────────────────────────────────────────┤");
        println!("│  进程总内存增长: {:.2} MB", total_memory_growth / 1024.0);
        println!("│  当前进程内存: {:.2} MB", current_memory / 1024.0);
        println!("│  进程启动内存: {:.2} MB", self.process_start_memory_kb / 1024.0);
        println!("├─────────────────────────────────────────────────────────────────┤");
        
        if stats.is_empty() {
            println!("│  (暂无插件调用记录)");
        } else {
            for (_plugin_name, stat) in stats.iter() {
                let avg_memory = if stat.memory_samples.is_empty() {
                    0.0
                } else {
                    stat.memory_samples.iter().sum::<f64>() / stat.memory_samples.len() as f64
                };
                
                println!("│  🔌 插件: {}", stat.plugin_name);
                println!("│     调用次数: {}", stat.call_count);
                println!("│     平均内存增量: {:.2} KB/次 (最近{}次样本)", avg_memory, stat.memory_samples.len());
                println!("│     峰值内存增量: {:.2} KB", stat.peak_memory_kb);
                println!("│     最近内存增量: {:.2} KB", stat.last_memory_kb);
                println!("│  ────────────────────────────────────────────────────────────");
            }
        }
        
        println!("└─────────────────────────────────────────────────────────────────┘");
    }
}

// ============================================================================
// ZeroMQ 消息数据结构
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FeatureMessage {
    #[serde(rename = "feature")]
    feature: FeatureData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FeatureData {
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

// ============================================================================
// 传感器数据结构
// ============================================================================

#[derive(Debug, Clone)]
struct CombinedSensorData {
    timestamp_ms: u64,
    device_id: String,
    vibration_xyz: (Vec<f64>, Vec<f64>, Vec<f64>),
    temperature_c: f64,
    current_a: f64,
}

// ============================================================================
// C++ 插件结果
// ============================================================================

#[derive(Debug, Clone)]
struct PluginResult {
    plugin_name: String,
    execution_time_ms: f64,
    result: Value,
}

// ============================================================================
// ZeroMQ 订阅器
// ============================================================================

/// 从真实 ZeroMQ socket 订阅消息（无限制接收）
async fn subscribe_zeromq_real_socket(
    sender: mpsc::Sender<FeatureMessage>,
    host: &str,
    port: u16,
) -> Result<(), String> {
    let addr = format!("{}:{}", host, port);
    println!("🔌 连接到 ZeroMQ 生产者: {}", addr);

    // 重试连接（等待生产者启动）
    let stream = match tokio::time::timeout(
        Duration::from_secs(10),
        async {
            for attempt in 1..=20 {
                match TcpStream::connect(&addr).await {
                    Ok(s) => {
                        println!("✅ 连接成功 (第 {} 次尝试)", attempt);
                        return Ok(s);
                    }
                    Err(_) => {
                        if attempt < 20 {
                            println!("⏳ 连接失败 (第 {} 次尝试)，{:.1} 秒后重试...", attempt, 0.5);
                            tokio::time::sleep(Duration::from_millis(500)).await;
                        } else {
                            return Err("连接失败（20次重试）".to_string());
                        }
                    }
                }
            }
            Err("超时".to_string())
        }
    ).await
    {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => return Err(format!("连接失败: {}", e)),
        Err(_) => return Err("连接超时".to_string()),
    };

    let (reader, _) = stream.into_split();
    let mut buf_reader = BufReader::new(reader);
    let mut line = String::new();
    let mut received_count = 0;

    println!("\n📥 开始无限制消费 ZeroMQ 消息...");
    println!("   (将持续接收消息，直到生产者断开连接或按 Ctrl+C 停止)\n");

    // 无限循环：持续接收消息直到连接关闭
    loop {
        line.clear();
        match tokio::time::timeout(
            Duration::from_secs(10),
            buf_reader.read_line(&mut line)
        ).await {
            Ok(Ok(0)) => {
                println!("\n✅ 生产者已断开连接，共接收 {} 条消息", received_count);
                break;
            }
            Ok(Ok(_)) => {
                match serde_json::from_str::<FeatureMessage>(&line) {
                    Ok(msg) => {
                        received_count += 1;
                        if sender.send(msg).await.is_err() {
                            println!("\n⚠️  发送通道已关闭 (共接收 {} 条消息)", received_count);
                            break;
                        }
                    }
                    Err(e) => {
                        eprintln!("  [{}] ⚠️  JSON 解析失败: {}", received_count + 1, e);
                    }
                }
            }
            Ok(Err(e)) => {
                return Err(format!("读取失败: {}", e));
            }
            Err(_) => {
                // 超时但不退出，继续等待
            }
        }
    }

    println!("\n✅ ZeroMQ 消息订阅完成 (总共接收 {} 条消息)\n", received_count);
    Ok(())
}

// ============================================================================
// 数据处理流程
// ============================================================================

/// 从 ZeroMQ 消息聚合传感器数据
fn aggregate_sensor_data_from_zeromq(msg: FeatureMessage) -> CombinedSensorData {
    let feature = &msg.feature;
    
    let parse_feature_string = |s: &str| -> Vec<f64> {
        s.split(',')
            .filter_map(|v| v.trim().parse::<f64>().ok())
            .collect()
    };
    
    let vibration_x = parse_feature_string(&feature.feature1);
    let vibration_y = parse_feature_string(&feature.feature2);
    let vibration_z = parse_feature_string(&feature.feature3);
    
    CombinedSensorData {
        timestamp_ms: feature.time,
        device_id: feature.device_id.clone(),
        vibration_xyz: (vibration_x, vibration_y, vibration_z),
        temperature_c: feature.temperature as f64 / 100.0,
        current_a: feature.meanLf as f64 / 1000.0,
    }
}

/// 执行 DAG 转换
fn execute_dag_pipeline(sensor_data: &CombinedSensorData) -> Value {
    json!({
        "timestamp_ms": sensor_data.timestamp_ms,
        "device_id": sensor_data.device_id,
        "vibration_x": sensor_data.vibration_xyz.0,
        "vibration_y": sensor_data.vibration_xyz.1,
        "vibration_z": sensor_data.vibration_xyz.2,
        "temperature": sensor_data.temperature_c,
        "current": sensor_data.current_a,
    })
}

/// 执行 C++ 插件链
async fn execute_cpp_plugin_chain(input: &Value) -> Result<(PluginResult, PluginResult, PluginResult), String> {
    // 创建并初始化执行器
    let mut executor = CppAlgorithmExecutor::new()
        .map_err(|e| format!("创建执行器失败: {}", e))?;
    executor.initialize()
        .map_err(|e| format!("初始化失败: {}", e))?;
    
    let empty_params = HashMap::new();

    // Vibrate31 插件
    let start = Instant::now();
    let vibrate31_result = executor.execute_plugin("vibrate31", input.clone(), empty_params.clone()).await
        .map_err(|e| format!("Vibrate31 执行失败: {}", e))?;
    let vibrate31_time = start.elapsed().as_secs_f64() * 1000.0;
    let vibrate31 = PluginResult {
        plugin_name: "vibrate31".to_string(),
        execution_time_ms: vibrate31_time,
        result: vibrate31_result.clone(),
    };

    // Error18 插件
    let start = Instant::now();
    let error18_result = executor.execute_plugin("error18", vibrate31_result, empty_params.clone()).await
        .map_err(|e| format!("Error18 执行失败: {}", e))?;
    let error18_time = start.elapsed().as_secs_f64() * 1000.0;
    let error18 = PluginResult {
        plugin_name: "error18".to_string(),
        execution_time_ms: error18_time,
        result: error18_result.clone(),
    };

    // Evaluation 插件
    let start = Instant::now();
    let evaluation_result = executor.execute_plugin("evaluation", error18_result, empty_params).await
        .map_err(|e| format!("Evaluation 执行失败: {}", e))?;
    let evaluation_time = start.elapsed().as_secs_f64() * 1000.0;
    let evaluation = PluginResult {
        plugin_name: "evaluation".to_string(),
        execution_time_ms: evaluation_time,
        result: evaluation_result,
    };

    Ok((vibrate31, error18, evaluation))
}

// ============================================================================
// 主程序
// ============================================================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    println!("\n╭──────────────────────────────────────────────────────────────────────────────╮");
    println!("│  🔴 实时监控: ZeroMQ → DAG → C++插件链                                  │");
    println!("│  数据流: ZeroMQ消费 → 特征解析 → DAG → vibrate31 → error18 → evaluation  │");
    println!("│  监控: 实时内存跟踪 + 插件性能分析                                  │");
    println!("╰──────────────────────────────────────────────────────────────────────────────╯");

    // 初始化内存监控器
    let memory_monitor = RealtimeMemoryMonitor::new();
    let monitor_clone = memory_monitor.clone();
    
    let start_total = Instant::now();

    println!("\n【步骤 1】连接到 ZeroMQ 生产者");
    println!("  地址: {}:{}", args.host, args.port);
    println!("  报告间隔: 每 {} 条消息\n", args.report_interval);

    let (tx, mut rx) = mpsc::channel::<FeatureMessage>(100);

    // 在后台启动 ZeroMQ 订阅任务
    let host = args.host.clone();
    let port = args.port;
    tokio::spawn(async move {
        if let Err(e) = subscribe_zeromq_real_socket(tx, &host, port).await {
            eprintln!("❌ ZeroMQ 订阅错误: {}", e);
        }
    });

    println!("【步骤 2】开始实时处理消息流\n");
    
    let mut processed_count = 0u64;
    let mut total_plugin_time = 0.0;

    // 实时消费循环
    while let Some(zmq_msg) = rx.recv().await {
        processed_count += 1;
        
        // 简洁的单条消息输出
        if processed_count % args.report_interval == 1 {
            println!("\n┌─── 消息 #{} ─────────────────────────────────────────────┐", processed_count);
        }
        println!("│  📦 Device: {}, Seq: {}, UUID: {}", 
            zmq_msg.feature.device_id, zmq_msg.feature.seq, &zmq_msg.feature.uuid[..8]);

        // 聚合传感器数据
        let sensor_data = aggregate_sensor_data_from_zeromq(zmq_msg);

        // 执行 DAG 转换
        let plugin_input = execute_dag_pipeline(&sensor_data);
        println!("│  ✅ DAG 转换完成");

        // 执行 C++ 插件链
        let memory_before = RealtimeMemoryMonitor::get_process_memory_kb();
        let plugin_start = Instant::now();
        match execute_cpp_plugin_chain(&plugin_input).await {
            Ok((vibrate31, error18, evaluation)) => {
                let plugin_time = plugin_start.elapsed().as_secs_f64() * 1000.0;
                total_plugin_time += plugin_time;
                
                let memory_after = RealtimeMemoryMonitor::get_process_memory_kb();

                // 记录内存使用
                monitor_clone.record_plugin_execution(
                    "vibrate31+error18+evaluation",
                    memory_before,
                    memory_after
                );

                println!("│  ✅ C++插件链执行成功 ({:.2}ms)", plugin_time);
                println!("│     ├─ Vibrate31: {:.2}ms", vibrate31.execution_time_ms);
                println!("│     ├─ Error18: {:.2}ms", error18.execution_time_ms);
                println!("│     └─ Evaluation: {:.2}ms", evaluation.execution_time_ms);
            }
            Err(e) => {
                println!("│  ❌ C++插件执行失败: {}", e);
            }
        }
        
        if processed_count % args.report_interval == 1 {
            println!("└────────────────────────────────────────────────────────┘");
        }

        // 定时打印内存监控报告
        if processed_count % args.report_interval == 0 {
            monitor_clone.print_realtime_report(processed_count);
            println!("\n  📊 性能统计：平均插件耗时 {:.2}ms, 总耗时 {:.2}s\n", 
                total_plugin_time / processed_count as f64,
                start_total.elapsed().as_secs_f64());
        }
    }

    // 打印最终报告
    let total_time = start_total.elapsed();
    println!("\n\n");
    println!("╭──────────────────────────────────────────────────────────────────────────────╮");
    println!("│  ✅ 实时监控程序结束                                                   │");
    println!("├──────────────────────────────────────────────────────────────────────────────┤");
    println!("│  📊 最终统计:                                                        │");
    println!("│     总处理消息: {} 条", processed_count);
    println!("│     总运行时长: {:.2}s", total_time.as_secs_f64());
    println!("│     平均插件链耗时: {:.2}ms", total_plugin_time / processed_count.max(1) as f64);
    println!("│     消息处理速率: {:.2} msg/s", processed_count as f64 / total_time.as_secs_f64());
    println!("├──────────────────────────────────────────────────────────────────────────────┤");
    memory_monitor.print_realtime_report(processed_count);
    println!("╰──────────────────────────────────────────────────────────────────────────────╯");

    Ok(())
}
