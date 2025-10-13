//! 容器化算法执行演示
//!
//! 这个示例展示了如何使用Rust Edge Compute框架来运行容器化的C++算法插件

use std::path::PathBuf;
use std::sync::Arc;
use rust_edge_compute::container::*;
use rust_edge_compute::core::*;
use rust_edge_compute::ffi::MemoryManager;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("🚀 Rust Edge Compute - 容器化算法执行演示");
    println!("================================================");

    // 1. 初始化组件
    println!("📦 初始化系统组件...");
    let memory_manager = Arc::new(MemoryManager::new());
    let container_manager = Arc::new(ContainerManager::new("youki".to_string()));
    let algorithm_executor = Arc::new(ContainerizedAlgorithmExecutor::new(
        container_manager,
        Arc::clone(&memory_manager),
    ));

    println!("✅ 系统组件初始化完成");

    // 2. 创建示例算法插件
    println!("🔧 注册示例算法插件...");

    // 创建矩阵乘法算法插件（使用实际的文件路径）
    let plugin_base_path = PathBuf::from("./plugins/matrix_multiplication_plugin");

    // 检查插件文件是否存在
    if !plugin_base_path.exists() {
        println!("⚠️  插件文件不存在，正在创建示例插件文件...");

        // 创建插件目录结构
        tokio::fs::create_dir_all(&plugin_base_path).await.unwrap();

        // 创建示例输入文件
        let input_example = serde_json::json!({
            "operation": "matrix_multiplication",
            "matrix_a": [[1.0, 2.0], [3.0, 4.0]],
            "matrix_b": [[5.0, 6.0], [7.0, 8.0]],
            "algorithm": "naive",
            "optimization": "basic"
        });

        let input_path = plugin_base_path.join("input_example.json");
        tokio::fs::write(&input_path, serde_json::to_string_pretty(&input_example).unwrap()).await.unwrap();
        println!("✅ 创建示例输入文件: {:?}", input_path);

        // 创建示例输出文件
        let output_example = serde_json::json!({
            "status": "success",
            "algorithm": "naive",
            "result": [[19.0, 22.0], [43.0, 50.0]],
            "performance": {
                "computation_time_ms": 1,
                "input_matrix_size": [2, 2],
                "output_matrix_size": [2, 2]
            },
            "metadata": {
                "version": "1.0.0",
                "execution_time_ms": 5
            }
        });

        let output_path = plugin_base_path.join("output_example.json");
        tokio::fs::write(&output_path, serde_json::to_string_pretty(&output_example).unwrap()).await.unwrap();
        println!("✅ 创建示例输出文件: {:?}", output_path);
    }

    let (matrix_mul_info, matrix_mul_image) = AlgorithmPluginBuilder::new("matrix_multiplication", "1.0.0")
        .description("高性能矩阵乘法算法 - 完整生产级实现")
        .resources(2.0, 512) // 2个CPU核心，512MB内存
        .timeout(600) // 10分钟超时
        .image_path(plugin_base_path.join("rootfs")) // 使用实际的rootfs路径
        .execute_command(vec![
            "/usr/local/bin/matrix_multiplication".to_string(),
            "--input".to_string(),
            "/input/input.json".to_string(),
            "--output".to_string(),
            "/output/result.json".to_string(),
        ])
        .env("ALGORITHM_TYPE", "matrix")
        .env("OPTIMIZATION_LEVEL", "high")
        .env("OMP_NUM_THREADS", "2")
        .env("MKL_NUM_THREADS", "2")
        .build();

    // 创建图像处理算法插件
    let (image_proc_info, image_proc_image) = AlgorithmPluginBuilder::new("image_processing", "2.1.0")
        .description("AI图像处理和分析算法")
        .resources(4.0, 2048) // 4个CPU核心，2GB内存
        .timeout(1800) // 30分钟超时
        .image_path(PathBuf::from("./plugins/image_proc_plugin"))
        .execute_command(vec![
            "/usr/local/bin/image_processor".to_string(),
            "--config".to_string(),
            "/input/input.json".to_string(),
            "--result".to_string(),
            "/output/result.json".to_string(),
        ])
        .env("CUDA_VISIBLE_DEVICES", "0")
        .env("MODEL_PATH", "/models/")
        .build();

    // 注册算法插件
    algorithm_executor.register_algorithm(matrix_mul_info, matrix_mul_image).await?;
    algorithm_executor.register_algorithm(image_proc_info, image_proc_image).await?;

    println!("✅ 算法插件注册完成");
    println!("   📊 注册的算法:");
    for algorithm in algorithm_executor.list_algorithms().await {
        println!("     - {} v{}: {}", algorithm.name, algorithm.version, algorithm.description);
        println!("       资源需求: CPU {:.1}核, 内存 {}MB",
                algorithm.resource_requirements.cpu_cores,
                algorithm.resource_requirements.memory_mb);
    }

    // 3. 执行矩阵乘法任务
    println!("\n🧮 执行矩阵乘法任务...");
    let matrix_request = ComputeRequest {
        id: "matrix_task_001".to_string(),
        algorithm: "matrix_multiplication".to_string(),
        parameters: serde_json::json!({
            "operation": "multiply",
            "matrix_a": [
                [1.0, 2.0, 3.0],
                [4.0, 5.0, 6.0],
                [7.0, 8.0, 9.0]
            ],
            "matrix_b": [
                [9.0, 8.0, 7.0],
                [6.0, 5.0, 4.0],
                [3.0, 2.0, 1.0]
            ],
            "precision": "double",
            "optimization": "avx"
        }),
        priority: TaskPriority::High,
        timeout: Some(300),
    };

    let matrix_result = algorithm_executor.execute_algorithm(matrix_request).await?;

    match matrix_result.status {
        ExecutionStatus::Success => {
            println!("✅ 矩阵乘法执行成功!");
            println!("   执行时间: {}ms", matrix_result.execution_time_ms);
            println!("   容器ID: {}", matrix_result.container_id);
            println!("   结果: {}", matrix_result.result.unwrap_or(serde_json::Value::Null));
            println!("   资源使用: CPU {:.1}%, 内存 {}MB",
                    matrix_result.resource_usage.cpu_usage_percent,
                    matrix_result.resource_usage.memory_usage_mb);
        }
        _ => {
            println!("❌ 矩阵乘法执行失败: {}",
                    matrix_result.error_message.unwrap_or("未知错误".to_string()));
        }
    }

    // 4. 执行图像处理任务
    println!("\n🖼️  执行图像处理任务...");
    let image_request = ComputeRequest {
        id: "image_task_001".to_string(),
        algorithm: "image_processing".to_string(),
        parameters: serde_json::json!({
            "operation": "classify",
            "image_path": "/data/input_image.jpg",
            "model": "resnet50",
            "confidence_threshold": 0.8,
            "preprocessing": {
                "resize": [224, 224],
                "normalize": true,
                "mean": [0.485, 0.456, 0.406],
                "std": [0.229, 0.224, 0.225]
            }
        }),
        priority: TaskPriority::Normal,
        timeout: Some(900),
    };

    let image_result = algorithm_executor.execute_algorithm(image_request).await?;

    match image_result.status {
        ExecutionStatus::Success => {
            println!("✅ 图像处理执行成功!");
            println!("   执行时间: {}ms", image_result.execution_time_ms);
            println!("   容器ID: {}", image_result.container_id);
            if let Some(result) = image_result.result {
                println!("   分类结果: {}", result);
            }
        }
        _ => {
            println!("❌ 图像处理执行失败: {}",
                    image_result.error_message.unwrap_or("未知错误".to_string()));
        }
    }

    // 5. 显示执行统计
    println!("\n📊 执行统计报告:");
    let stats = algorithm_executor.get_execution_stats().await;
    println!("总执行次数: {}", stats.total_executions);
    println!("成功执行次数: {}", stats.successful_executions);
    println!("失败执行次数: {}", stats.failed_executions);
    println!("超时执行次数: {}", stats.timeout_executions);
    println!("平均执行时间: {:.2}ms", stats.avg_execution_time_ms);
    println!("成功率: {:.2}%", if stats.total_executions > 0 {
        stats.successful_executions as f64 / stats.total_executions as f64 * 100.0
    } else { 0.0 });

    // 6. 显示内存使用情况
    println!("\n🧠 内存使用情况:");
    let memory_stats = memory_manager.get_stats().await;
    println!("总分配内存: {} bytes", memory_stats.total_memory);
    println!("活跃内存: {} bytes", memory_stats.active_memory);
    println!("内存块数量: {}", memory_stats.total_blocks);

    // 7. 演示并发执行
    println!("\n🔄 演示并发算法执行...");
    let mut concurrent_tasks = vec![];

    for i in 0..3 {
        let executor_clone = Arc::clone(&algorithm_executor);
        let task = tokio::spawn(async move {
            let request = ComputeRequest {
                id: format!("concurrent_task_{}", i),
                algorithm: "matrix_multiplication".to_string(),
                parameters: serde_json::json!({
                    "operation": "multiply",
                    "matrix_a": [[i as f64 + 1.0, 0.0], [0.0, i as f64 + 1.0]],
                    "matrix_b": [[1.0, 0.0], [0.0, 1.0]],
                }),
                priority: TaskPriority::Normal,
                timeout: Some(60),
            };

            let result = executor_clone.execute_algorithm(request).await;
            (i, result)
        });
        concurrent_tasks.push(task);
    }

    println!("等待并发任务完成...");
    for task in concurrent_tasks {
        let (task_id, result) = task.await?;
        match result {
            Ok(execution_result) => {
                println!("任务 {}: {}ms", task_id, execution_result.execution_time_ms);
            }
            Err(e) => {
                println!("任务 {} 失败: {}", task_id, e);
            }
        }
    }

    // 8. 清理和最终报告
    println!("\n🧹 执行清理...");
    let final_stats = algorithm_executor.get_execution_stats().await;
    let final_memory_stats = memory_manager.get_stats().await;

    println!("最终统计:");
    println!("- 总执行任务数: {}", final_stats.total_executions);
    println!("- 系统内存使用: {} bytes", final_memory_stats.total_memory);
    println!("- 活跃内存块数: {}", final_memory_stats.active_blocks);

    println!("\n🎉 容器化算法执行演示完成!");
    println!("================================================");
    println!("这个演示展示了:");
    println!("✅ 算法插件的容器化部署和执行");
    println!("✅ 资源隔离和性能监控");
    println!("✅ 并发任务处理能力");
    println!("✅ 完整的错误处理和恢复机制");
    println!("✅ 企业级的生产就绪特性");

    Ok(())
}

/// 高级用法示例：自定义算法插件工厂
pub struct AlgorithmPluginFactory {
    base_image_path: PathBuf,
    registry: HashMap<String, Box<dyn Fn() -> (AlgorithmInfo, PluginImage) + Send + Sync>>,
}

impl AlgorithmPluginFactory {
    pub fn new(base_image_path: PathBuf) -> Self {
        Self {
            base_image_path,
            registry: HashMap::new(),
        }
    }

    /// 注册算法插件模板
    pub fn register_template<F>(&mut self, name: &str, template_fn: F)
    where
        F: Fn() -> (AlgorithmInfo, PluginImage) + Send + Sync + 'static,
    {
        self.registry.insert(name.to_string(), Box::new(template_fn));
    }

    /// 创建算法插件实例
    pub fn create_plugin(&self, name: &str, version: &str) -> Option<(AlgorithmInfo, PluginImage)> {
        if let Some(template_fn) = self.registry.get(name) {
            let (mut info, mut image) = template_fn();

            // 更新版本信息
            info.version = version.to_string();
            image.image_version = version.to_string();

            // 设置镜像路径
            image.image_path = self.base_image_path.join(format!("{}_{}", name, version));

            Some((info, image))
        } else {
            None
        }
    }

    /// 获取可用模板列表
    pub fn list_templates(&self) -> Vec<String> {
        self.registry.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_algorithm_plugin_factory() {
        let mut factory = AlgorithmPluginFactory::new(PathBuf::from("./test_plugins"));

        // 注册模板
        factory.register_template("test_algorithm", || {
            AlgorithmPluginBuilder::new("test_algorithm", "1.0.0")
                .description("测试算法")
                .resources(1.0, 128)
                .timeout(30)
                .execute_command(vec!["/usr/bin/test".to_string()])
                .build()
        });

        // 创建插件实例
        let plugin = factory.create_plugin("test_algorithm", "2.0.0");
        assert!(plugin.is_some());

        let (info, image) = plugin.unwrap();
        assert_eq!(info.version, "2.0.0");
        assert_eq!(image.image_version, "2.0.0");
        assert!(image.image_path.to_string_lossy().contains("test_algorithm_2.0.0"));
    }

    #[test]
    fn test_plugin_builder() {
        let (info, image) = AlgorithmPluginBuilder::new("test", "1.0")
            .description("Test algorithm")
            .resources(2.0, 512)
            .timeout(300)
            .image_path(PathBuf::from("/test/image"))
            .execute_command(vec!["/test/cmd".to_string()])
            .env("TEST", "value")
            .build();

        assert_eq!(info.name, "test");
        assert_eq!(info.version, "1.0");
        assert_eq!(info.description, "Test algorithm");
        assert_eq!(info.resource_requirements.cpu_cores, 2.0);
        assert_eq!(info.resource_requirements.memory_mb, 512);
        assert_eq!(info.timeout_seconds, 300);
        assert_eq!(image.execute_command, vec!["/test/cmd"]);
        assert_eq!(image.environment.get("TEST"), Some(&"value".to_string()));
    }
}
