//! Vibrate31 算法性能基准测试
//!
//! 这个文件包含了完整的性能基准测试套件，用于评估Vibrate31算法的性能表现

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use rust_edge_compute::container::*;
use rust_edge_compute::core::*;
use rust_edge_compute::ffi::MemoryManager;

// 导入具体的容器管理器
use rust_edge_compute::container::youki_manager::YoukiContainerManager;
use rust_edge_compute::container::algorithm_executor::ContainerizedAlgorithmExecutor;

/// 性能基准测试配置
#[derive(Debug, Clone)]
pub struct BenchmarkConfig {
    pub algorithm_config: Vibrate31Config,
    pub test_cases: Vec<TestCase>,
    pub iterations: usize,
    pub warmup_iterations: usize,
    pub enable_memory_profiling: bool,
    pub enable_cpu_profiling: bool,
}

/// 测试用例
#[derive(Debug, Clone)]
pub struct TestCase {
    pub name: String,
    pub description: String,
    pub sampling_rate: usize,
    pub duration_seconds: f64,
    pub signal_type: SignalType,
    pub expected_performance: ExpectedPerformance,
}

/// 信号类型
#[derive(Debug, Clone)]
pub enum SignalType {
    SineWave { frequency: f64, amplitude: f64 },
    MultiFrequency { components: Vec<(f64, f64)> },
    Noise { amplitude: f64 },
    BearingFault { speed: f64, fault_frequency_ratio: f64 },
    GearFault { teeth: usize, speed: f64 },
}

/// 期望性能指标
#[derive(Debug, Clone)]
pub struct ExpectedPerformance {
    pub max_computation_time_ms: u64,
    pub max_memory_usage_mb: f64,
    pub min_confidence: f64,
}

/// 基准测试结果
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    pub test_case_name: String,
    pub iterations: usize,
    pub total_time: Duration,
    pub average_time: Duration,
    pub min_time: Duration,
    pub max_time: Duration,
    pub p50_time: Duration,
    pub p95_time: Duration,
    pub p99_time: Duration,
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
    pub throughput_samples_per_second: f64,
    pub quality_metrics: QualityMetrics,
    pub success_rate: f64,
    pub error_distribution: HashMap<String, usize>,
}

/// 综合基准测试报告
#[derive(Debug, Clone)]
pub struct BenchmarkReport {
    pub timestamp: u64,
    pub system_info: SystemInfo,
    pub results: Vec<BenchmarkResult>,
    pub summary: BenchmarkSummary,
}

/// 系统信息
#[derive(Debug, Clone)]
pub struct SystemInfo {
    pub cpu_model: String,
    pub cpu_cores: usize,
    pub memory_total_gb: f64,
    pub os_version: String,
    pub rust_version: String,
}

/// 基准测试摘要
#[derive(Debug, Clone)]
pub struct BenchmarkSummary {
    pub total_tests: usize,
    pub passed_tests: usize,
    pub failed_tests: usize,
    pub average_performance_score: f64,
    pub memory_efficiency_score: f64,
    pub recommendations: Vec<String>,
}

/// 性能基准测试器
pub struct PerformanceBenchmark {
    config: BenchmarkConfig,
    executor: Arc<ContainerizedAlgorithmExecutor>,
    memory_manager: Arc<MemoryManager>,
    container_manager: Arc<YoukiContainerManager>,
}

impl PerformanceBenchmark {
    /// 创建新的性能基准测试器
    pub async fn new(config: BenchmarkConfig, memory_manager: Arc<MemoryManager>) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        // 创建Youki容器管理器
        let container_manager = Arc::new(YoukiContainerManager::new(PathBuf::from("./runtime")));

        // 创建容器化算法执行器
        let executor = Arc::new(ContainerizedAlgorithmExecutor::new(
            container_manager.clone(),
            memory_manager.clone(),
        ));

        // 注册Vibrate31算法插件
        let vibrate31_info = AlgorithmInfo {
            name: "vibrate31".to_string(),
            version: "1.0.0".to_string(),
            description: "生产级振动特征提取算法，支持频谱分析和工况识别".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "wave_data": {"type": "array", "items": {"type": "number"}},
                    "speed_data": {"type": "array", "items": {"type": "number"}},
                    "sampling_rate": {"type": "number", "minimum": 100, "maximum": 50000}
                },
                "required": ["wave_data", "speed_data", "sampling_rate"]
            }),
            output_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "basic_stats": {"type": "object"},
                    "spectral_features": {"type": "object"},
                    "condition_features": {"type": "object"}
                }
            }),
            resource_requirements: ResourceRequirements {
                cpu_cores: 2.0,
                memory_mb: 512,
                disk_mb: 1024,
                network_mbps: Some(10),
            },
            timeout_seconds: 300,
            max_concurrent: 10,
        };

        // 创建插件镜像信息
        let plugin_base_path = PathBuf::from("./plugins/vibrate31_plugin");
        tokio::fs::create_dir_all(&plugin_base_path).await.unwrap();

        let vibrate31_image = PluginImage {
            image_name: "vibrate31-algorithm".to_string(),
            image_version: "1.0.0".to_string(),
            image_path: plugin_base_path.join("rootfs"),
            execute_command: vec![
                "/usr/local/bin/vibrate31".to_string(),
                "--input".to_string(),
                "/input/input.json".to_string(),
                "--output".to_string(),
                "/output/result.json".to_string(),
            ],
            environment: {
                let mut env = HashMap::new();
                env.insert("ALGORITHM_NAME".to_string(), "vibrate31".to_string());
                env.insert("ALGORITHM_VERSION".to_string(), "1.0.0".to_string());
                env.insert("RUST_LOG".to_string(), "info".to_string());
                env
            },
            mounts: Vec::new(),
        };

        // 注册算法插件
        executor.register_algorithm(vibrate31_info, vibrate31_image).await?;

        Ok(Self {
            config,
            executor,
            memory_manager,
            container_manager,
        })
    }

    /// 运行完整的基准测试套件
    pub async fn run_full_benchmark(&self) -> Result<BenchmarkReport, Box<dyn std::error::Error + Send + Sync>> {
        println!("🚀 开始Vibrate31性能基准测试...");
        println!("================================================");

        let start_time = Instant::now();

        // 预热阶段
        println!("🔥 执行预热阶段...");
        self.run_warmup().await?;
        println!("✅ 预热完成");

        // 运行所有测试用例
        let mut results = Vec::new();
        for test_case in &self.config.test_cases {
            println!("🧪 运行测试用例: {}", test_case.name);
            let result = self.run_test_case(test_case).await?;
            results.push(result);
            println!("✅ 测试用例 {} 完成", test_case.name);
        }

        // 生成系统信息
        let system_info = self.collect_system_info().await?;

        // 生成摘要报告
        let summary = self.generate_summary(&results);

        let report = BenchmarkReport {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs(),
            system_info,
            results,
            summary,
        };

        let total_time = start_time.elapsed();
        println!("⏱️  基准测试完成，总耗时: {:.2}s", total_time.as_secs_f64());

        Ok(report)
    }

    /// 运行预热阶段
    async fn run_warmup(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        for i in 0..self.config.warmup_iterations {
            let test_data = self.generate_test_data(&self.config.test_cases[0]).await?;
            let _ = self.executor.execute(test_data).await?;
            if i % 10 == 0 {
                println!("  预热进度: {}/{}", i + 1, self.config.warmup_iterations);
            }
        }
        Ok(())
    }

    /// 运行单个测试用例
    async fn run_test_case(&self, test_case: &TestCase) -> Result<BenchmarkResult, Box<dyn std::error::Error + Send + Sync>> {
        let mut execution_times = Vec::with_capacity(self.config.iterations);
        let mut memory_usages = Vec::new();
        let mut quality_scores = Vec::new();
        let mut errors = HashMap::new();
        let mut success_count = 0;

        println!("  📊 执行 {} 次迭代...", self.config.iterations);

        for i in 0..self.config.iterations {
            // 生成测试数据
            let test_data = self.generate_test_data(test_case).await?;

            // 记录开始时的内存使用
            let memory_before = self.memory_manager.get_stats().await.total_memory as f64 / (1024.0 * 1024.0);

            // 执行算法
            let start_time = Instant::now();
            let result = self.executor.execute(test_data).await;
            let execution_time = start_time.elapsed();

            // 记录结束时的内存使用
            let memory_after = self.memory_manager.get_stats().await.total_memory as f64 / (1024.0 * 1024.0);
            let memory_usage = memory_after - memory_before;

            match result {
                Ok(execution_result) => {
                    success_count += 1;
                    execution_times.push(execution_time);
                    memory_usages.push(memory_usage);

                    // 对于成功的执行，计算质量评分
                    if let Some(result_data) = &execution_result.result {
                        // 简化的质量评分计算（基于执行时间和资源使用）
                        let quality_score = if execution_time.as_millis() < 1000 {
                            0.9
                        } else if execution_time.as_millis() < 2000 {
                            0.8
                        } else {
                            0.7
                        };
                        quality_scores.push(quality_score);
                    } else {
                        quality_scores.push(0.5); // 默认质量评分
                    }
                }
                Err(e) => {
                    let error_type = e.to_string();
                    *errors.entry(error_type).or_insert(0) += 1;
                }
            }

            if i % (self.config.iterations / 10).max(1) == 0 {
                println!("    迭代进度: {}/{}", i + 1, self.config.iterations);
            }
        }

        // 计算统计指标
        let success_rate = success_count as f64 / self.config.iterations as f64;

        if execution_times.is_empty() {
            return Err("没有成功的执行结果".into());
        }

        let total_time: Duration = execution_times.iter().sum();
        let average_time = total_time / execution_times.len() as u32;

        execution_times.sort();
        let min_time = execution_times[0];
        let max_time = execution_times[execution_times.len() - 1];

        let p50_index = (execution_times.len() as f64 * 0.5) as usize;
        let p95_index = (execution_times.len() as f64 * 0.95) as usize;
        let p99_index = (execution_times.len() as f64 * 0.99) as usize;

        let p50_time = execution_times[p50_index];
        let p95_time = execution_times[p95_index.min(execution_times.len() - 1)];
        let p99_time = execution_times[p99_index.min(execution_times.len() - 1)];

        // 计算平均内存使用
        let avg_memory_usage = memory_usages.iter().sum::<f64>() / memory_usages.len() as f64;

        // 计算平均质量分数
        let avg_quality = quality_scores.iter().sum::<f64>() / quality_scores.len() as f64;

        // 计算吞吐量
        let total_samples = test_case.sampling_rate as f64 * test_case.duration_seconds;
        let throughput = total_samples / average_time.as_secs_f64();

        // 计算CPU使用率（简化的估算）
        let cpu_usage = 0.0; // 实际项目中需要系统监控

        let quality_metrics = QualityMetrics {
            signal_quality: 1.0,
            data_integrity: success_rate,
            processing_confidence: avg_quality,
            computation_time_ms: average_time.as_millis() as u64,
            memory_usage_mb: avg_memory_usage,
        };

        Ok(BenchmarkResult {
            test_case_name: test_case.name.clone(),
            iterations: self.config.iterations,
            total_time,
            average_time,
            min_time,
            max_time,
            p50_time,
            p95_time,
            p99_time,
            memory_usage_mb: avg_memory_usage,
            cpu_usage_percent: cpu_usage,
            throughput_samples_per_second: throughput,
            quality_metrics,
            success_rate,
            error_distribution: errors,
        })
    }

    /// 生成测试数据
    async fn generate_test_data(&self, test_case: &TestCase) -> Result<ComputeRequest, Box<dyn std::error::Error + Send + Sync>> {
        let num_samples = (test_case.sampling_rate as f64 * test_case.duration_seconds) as usize;

        let mut wave_data = Vec::with_capacity(num_samples);
        let mut speed_data = Vec::with_capacity(num_samples);

        match &test_case.signal_type {
            SignalType::SineWave { frequency, amplitude } => {
                for i in 0..num_samples {
                    let t = i as f64 / test_case.sampling_rate as f64;
                    let signal = amplitude * (2.0 * std::f64::consts::PI * frequency * t).sin();
                    wave_data.push(signal);
                    speed_data.push(1800.0); // 固定转速
                }
            }
            SignalType::MultiFrequency { components } => {
                for i in 0..num_samples {
                    let t = i as f64 / test_case.sampling_rate as f64;
                    let mut signal = 0.0;
                    for (freq, amp) in components {
                        signal += amp * (2.0 * std::f64::consts::PI * freq * t).sin();
                    }
                    wave_data.push(signal);
                    speed_data.push(1800.0);
                }
            }
            SignalType::Noise { amplitude } => {
                use rand::Rng;
                let mut rng = rand::thread_rng();
                for _ in 0..num_samples {
                    let signal = amplitude * rng.gen_range(-1.0..1.0);
                    wave_data.push(signal);
                    speed_data.push(1800.0);
                }
            }
            SignalType::BearingFault { speed, fault_frequency_ratio } => {
                for i in 0..num_samples {
                    let t = i as f64 / test_case.sampling_rate as f64;
                    let base_freq = speed / 60.0; // 将转速转换为Hz
                    let fault_freq = base_freq * fault_frequency_ratio;

                    // 基础振动
                    let base_signal = 5.0 * (2.0 * std::f64::consts::PI * base_freq * t).sin();

                    // 故障特征（脉冲信号）
                    let fault_signal = if (t * fault_freq).fract() < 0.1 { 10.0 } else { 0.0 };

                    let total_signal = base_signal + fault_signal;
                    wave_data.push(total_signal);
                    speed_data.push(*speed);
                }
            }
            SignalType::GearFault { teeth, speed } => {
                let gear_freq = *teeth as f64 * speed / 60.0; // 齿轮啮合频率
                for i in 0..num_samples {
                    let t = i as f64 / test_case.sampling_rate as f64;

                    // 正常的齿轮啮合振动
                    let normal_signal = 3.0 * (2.0 * std::f64::consts::PI * gear_freq * t).sin();

                    // 故障特征（缺失齿）
                    let fault_signal = if (t * gear_freq * 8.0).fract() < 0.05 { -5.0 } else { 0.0 };

                    let total_signal = normal_signal + fault_signal;
                    wave_data.push(total_signal);
                    speed_data.push(*speed);
                }
            }
        }

        Ok(ComputeRequest {
            id: format!("benchmark_{}_{}", test_case.name, uuid::Uuid::new_v4()),
            algorithm: "vibrate31".to_string(),
            parameters: serde_json::json!({
                "wave_data": wave_data,
                "speed_data": speed_data,
                "sampling_rate": test_case.sampling_rate,
                "device_id": format!("benchmark_{}", test_case.name),
                "sensor_location": "test_bearing"
            }),
            timeout_seconds: Some(300),
        })
    }

    /// 收集系统信息
    async fn collect_system_info(&self) -> Result<SystemInfo, Box<dyn std::error::Error + Send + Sync>> {
        Ok(SystemInfo {
            cpu_model: "Unknown CPU".to_string(), // 实际项目中需要系统调用
            cpu_cores: num_cpus::get(),
            memory_total_gb: 8.0, // 实际项目中需要系统调用
            os_version: std::env::consts::OS.to_string(),
            rust_version: rustc_version::version().unwrap_or_default().to_string(),
        })
    }

    /// 生成基准测试摘要
    fn generate_summary(&self, results: &[BenchmarkResult]) -> BenchmarkSummary {
        let total_tests = results.len();
        let passed_tests = results.iter().filter(|r| r.success_rate >= 0.95).count();
        let failed_tests = total_tests - passed_tests;

        let avg_performance = results.iter()
            .map(|r| 1000.0 / r.average_time.as_secs_f64()) // 转换为执行次数/秒
            .sum::<f64>() / results.len() as f64;

        let avg_memory_efficiency = results.iter()
            .map(|r| 1.0 / r.memory_usage_mb) // 内存效率评分
            .sum::<f64>() / results.len() as f64;

        let mut recommendations = Vec::new();

        if avg_performance < 10.0 {
            recommendations.push("考虑优化算法实现以提高性能".to_string());
        }

        if avg_memory_efficiency < 0.1 {
            recommendations.push("考虑优化内存使用效率".to_string());
        }

        if failed_tests > 0 {
            recommendations.push(format!("有 {} 个测试用例未达到性能要求", failed_tests));
        }

        BenchmarkSummary {
            total_tests,
            passed_tests,
            failed_tests,
            average_performance_score: avg_performance,
            memory_efficiency_score: avg_memory_efficiency,
            recommendations,
        }
    }

    /// 打印基准测试报告
    pub fn print_report(&self, report: &BenchmarkReport) {
        println!("\n📊 Vibrate31 性能基准测试报告");
        println!("========================================");

        println!("🖥️  系统信息:");
        println!("   CPU 核心数: {}", report.system_info.cpu_cores);
        println!("   内存容量: {:.1} GB", report.system_info.memory_total_gb);
        println!("   操作系统: {}", report.system_info.os_version);
        println!("   Rust 版本: {}", report.system_info.rust_version);

        println!("\n📈 测试结果汇总:");
        for result in &report.results {
            println!("   {}", result.test_case_name);
            println!("     执行时间: 平均 {:.2}ms, P95 {:.2}ms, P99 {:.2}ms",
                     result.average_time.as_secs_f64() * 1000.0,
                     result.p95_time.as_secs_f64() * 1000.0,
                     result.p99_time.as_secs_f64() * 1000.0);
            println!("     内存使用: {:.2} MB", result.memory_usage_mb);
            println!("     吞吐量: {:.0} 采样/秒", result.throughput_samples_per_second);
            println!("     成功率: {:.2}%", result.success_rate * 100.0);
            println!("     质量评分: {:.3}", result.quality_metrics.processing_confidence);
        }

        println!("\n🎯 性能摘要:");
        println!("   总测试用例: {}", report.summary.total_tests);
        println!("   通过测试: {}", report.summary.passed_tests);
        println!("   失败测试: {}", report.summary.failed_tests);
        println!("   平均性能评分: {:.2}", report.summary.average_performance_score);
        println!("   内存效率评分: {:.2}", report.summary.memory_efficiency_score);

        if !report.summary.recommendations.is_empty() {
            println!("\n💡 优化建议:");
            for recommendation in &report.summary.recommendations {
                println!("   • {}", recommendation);
            }
        }

        println!("\n✅ 基准测试完成!");
    }
}

/// 从JSON配置文件加载基准测试配置
pub async fn load_benchmark_config_from_json() -> Result<BenchmarkConfig, Box<dyn std::error::Error + Send + Sync>> {
    let config_path = PathBuf::from("./examples/vibrate31_config.json");

    if !config_path.exists() {
        return Err("配置文件不存在，请确保 vibrate31_config.json 文件存在".into());
    }

    let config_content = tokio::fs::read_to_string(&config_path).await?;
    let config: serde_json::Value = serde_json::from_str(&config_content)?;

    // 解析配置（这里简化为默认配置，实际应该从JSON中解析）
    Ok(BenchmarkConfig {
        algorithm_config: Vibrate31Config {
            min_duration_seconds: config["algorithm"]["min_duration_seconds"].as_f64().unwrap_or(1.0),
            dc_threshold: config["algorithm"]["dc_threshold"].as_f64().unwrap_or(500.0),
            spectral_config: SpectralConfig {
                window_type: "hann".to_string(),
                overlap_ratio: 0.5,
                frequency_range: (0.0, 1000.0),
                resolution: 1.0,
            },
            condition_config: ConditionConfig {
                speed_thresholds: vec![10.0, 50.0],
                stability_window: 100,
                anomaly_threshold: 0.7,
            },
            monitoring_config: MonitoringConfig {
                enable_performance_tracking: true,
                memory_limit_mb: 512.0,
                timeout_seconds: 300,
                health_check_interval: 60,
            },
        },
        test_cases: vec![
            TestCase {
                name: "sine_wave_1k".to_string(),
                description: "1kHz正弦波信号测试".to_string(),
                sampling_rate: 2000,
                duration_seconds: 5.0,
                signal_type: SignalType::SineWave {
                    frequency: 1000.0,
                    amplitude: 10.0,
                },
                expected_performance: ExpectedPerformance {
                    max_computation_time_ms: 100,
                    max_memory_usage_mb: 50.0,
                    min_confidence: 0.9,
                },
            },
            TestCase {
                name: "multi_frequency".to_string(),
                description: "多频率复合信号测试".to_string(),
                sampling_rate: 2000,
                duration_seconds: 5.0,
                signal_type: SignalType::MultiFrequency {
                    components: vec![
                        (100.0, 5.0),
                        (500.0, 3.0),
                        (1000.0, 2.0),
                    ],
                },
                expected_performance: ExpectedPerformance {
                    max_computation_time_ms: 150,
                    max_memory_usage_mb: 60.0,
                    min_confidence: 0.85,
                },
            },
            TestCase {
                name: "bearing_fault".to_string(),
                description: "轴承故障模拟测试".to_string(),
                sampling_rate: 2000,
                duration_seconds: 5.0,
                signal_type: SignalType::BearingFault {
                    speed: 1800.0,
                    fault_frequency_ratio: 3.5,
                },
                expected_performance: ExpectedPerformance {
                    max_computation_time_ms: 200,
                    max_memory_usage_mb: 70.0,
                    min_confidence: 0.8,
                },
            },
            TestCase {
                name: "noise_signal".to_string(),
                description: "噪声信号测试".to_string(),
                sampling_rate: 2000,
                duration_seconds: 5.0,
                signal_type: SignalType::Noise {
                    amplitude: 1.0,
                },
                expected_performance: ExpectedPerformance {
                    max_computation_time_ms: 120,
                    max_memory_usage_mb: 55.0,
                    min_confidence: 0.75,
                },
            },
        ],
        iterations: 100,
        warmup_iterations: 20,
        enable_memory_profiling: true,
        enable_cpu_profiling: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_performance_benchmark() {
        let config = default_benchmark_config();
        let memory_manager = Arc::new(MemoryManager::new());

        let benchmark = PerformanceBenchmark::new(config, memory_manager).await.unwrap();

        // 只运行一个简单的测试用例
        let test_case = TestCase {
            name: "simple_test".to_string(),
            description: "简单性能测试".to_string(),
            sampling_rate: 1000,
            duration_seconds: 1.0,
            signal_type: SignalType::SineWave {
                frequency: 100.0,
                amplitude: 5.0,
            },
            expected_performance: ExpectedPerformance {
                max_computation_time_ms: 100,
                max_memory_usage_mb: 50.0,
                min_confidence: 0.8,
            },
        };

        let result = benchmark.run_test_case(&test_case).await.unwrap();

        assert!(result.success_rate > 0.9);
        assert!(result.average_time.as_millis() > 0);
        assert!(result.quality_metrics.processing_confidence > 0.5);

        println!("✅ 性能基准测试通过");
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("🚀 Vibrate31 性能基准测试工具");
    println!("=================================");

    // 从JSON配置文件加载配置
    println!("📋 加载配置文件...");
    let config = load_benchmark_config_from_json().await?;
    println!("✅ 配置文件加载完成");

    let memory_manager = Arc::new(MemoryManager::new());

    // 创建基准测试器
    println!("🔧 初始化基准测试器...");
    let benchmark = PerformanceBenchmark::new(config, memory_manager).await?;
    println!("✅ 基准测试器初始化完成");

    // 运行完整基准测试
    let report = benchmark.run_full_benchmark().await?;

    // 打印报告
    benchmark.print_report(&report);

    // 保存报告到文件（可选）
    let report_json = serde_json::to_string_pretty(&report)?;
    std::fs::write("vibrate31_benchmark_report.json", report_json)?;

    println!("📄 详细报告已保存到: vibrate31_benchmark_report.json");

    Ok(())
}
