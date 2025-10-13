// 主程序文件 - 矩阵乘法算法插件
// 生产级实现，支持多种矩阵乘法算法和优化策略

#include <iostream>
#include <fstream>
#include <string>
#include <vector>
#include <chrono>
#include <memory>
#include <stdexcept>
#include <boost/program_options.hpp>
#include <json/json.h>

#include "matrix_multiplication_edge_optimized.hpp"
#include "json_handler.hpp"
#include "performance_monitor.hpp"
#include "version.hpp"

// 命令行参数结构体
struct CommandLineOptions {
    std::string input_file = "/input/input.json";
    std::string output_file = "/output/result.json";
    bool show_version = false;
    bool show_help = false;
    std::string algorithm = "naive";
    int optimization_level = 1;
    bool enable_profiling = false;
    size_t max_memory_mb = 512;
};

// 解析命令行参数
CommandLineOptions parseCommandLine(int argc, char* argv[]) {
    namespace po = boost::program_options;

    CommandLineOptions options;

    po::options_description desc("Matrix Multiplication Plugin Options");
    desc.add_options()
        ("help,h", po::bool_switch(&options.show_help), "显示帮助信息")
        ("version,v", po::bool_switch(&options.show_version), "显示版本信息")
        ("input,i", po::value<std::string>(&options.input_file)->default_value("/input/input.json"),
         "输入文件路径")
        ("output,o", po::value<std::string>(&options.output_file)->default_value("/output/result.json"),
         "输出文件路径")
        ("algorithm,a", po::value<std::string>(&options.algorithm)->default_value("naive"),
         "使用的算法 (naive, tiled, strassen, eigen, openblas)")
        ("optimization,O", po::value<int>(&options.optimization_level)->default_value(1),
         "优化级别 (0-3)")
        ("profile,p", po::bool_switch(&options.enable_profiling), "启用性能分析")
        ("max-memory", po::value<size_t>(&options.max_memory_mb)->default_value(512),
         "最大内存使用量 (MB)");

    po::variables_map vm;
    try {
        po::store(po::parse_command_line(argc, argv, desc), vm);
        po::notify(vm);
    } catch (const std::exception& e) {
        std::cerr << "命令行参数解析错误: " << e.what() << std::endl;
        std::cout << desc << std::endl;
        throw;
    }

    if (options.show_help) {
        std::cout << "矩阵乘法算法插件 v" << MATRIX_MULTIPLICATION_VERSION << std::endl;
        std::cout << "======================================" << std::endl;
        std::cout << desc << std::endl;
        exit(0);
    }

    if (options.show_version) {
        std::cout << "Matrix Multiplication Plugin v"
                  << MATRIX_MULTIPLICATION_VERSION_MAJOR << "."
                  << MATRIX_MULTIPLICATION_VERSION_MINOR << "."
                  << MATRIX_MULTIPLICATION_VERSION_PATCH << std::endl;
        exit(0);
    }

    return options;
}

// 主函数
int main(int argc, char* argv[]) {
    auto start_time = std::chrono::high_resolution_clock::now();

    try {
        // 解析命令行参数
        CommandLineOptions options = parseCommandLine(argc, argv);

        // 初始化性能监控器
        std::unique_ptr<PerformanceMonitor> perf_monitor;
        if (options.enable_profiling) {
            perf_monitor = std::make_unique<PerformanceMonitor>();
            perf_monitor->startProfiling("total_execution");
        }

        // 创建JSON处理器
        JsonHandler json_handler;

        // 读取输入参数
        Json::Value input_params;
        if (!json_handler.readJsonFile(options.input_file, input_params)) {
            throw std::runtime_error("无法读取输入文件: " + options.input_file);
        }

        // 验证输入参数
        if (!json_handler.validateInput(input_params)) {
            throw std::runtime_error("输入参数验证失败");
        }

        // 解析矩阵参数
        MatrixMultiplication::Matrix A, B;
        if (!json_handler.parseMatrices(input_params, A, B)) {
            throw std::runtime_error("矩阵参数解析失败");
        }

        // 创建矩阵乘法器 - 边缘端优化版本
        MatrixMultiplication::AlgorithmType algorithm_type;
        if (options.algorithm == "naive") {
            algorithm_type = MatrixMultiplication::AlgorithmType::NAIVE;
        } else if (options.algorithm == "tiled") {
            algorithm_type = MatrixMultiplication::AlgorithmType::TILED;
        } else {
            // 默认使用分块算法（更适合边缘端）
            std::cout << "未知算法类型 '" << options.algorithm << "'，使用默认分块算法" << std::endl;
            algorithm_type = MatrixMultiplication::AlgorithmType::TILED;
        }

        // 创建边缘端优化的矩阵乘法器
        MatrixMultiplication multiplier(algorithm_type,
                                       static_cast<MatrixMultiplication::OptimizationLevel>(options.optimization_level),
                                       1024); // 1GB内存限制

        // 检查内存限制
        size_t estimated_memory = multiplier.estimateMemoryUsage(A.size(), A[0].size(), B[0].size());
        if (estimated_memory > options.max_memory_mb * 1024 * 1024) {
            throw std::runtime_error("预估内存使用量 (" + std::to_string(estimated_memory / 1024 / 1024) +
                                   "MB) 超过限制 (" + std::to_string(options.max_memory_mb) + "MB)");
        }

        // 检查矩阵是否可以处理
        if (!multiplier.canHandleMatrix(A.size(), A[0].size(), B[0].size())) {
            throw std::runtime_error("矩阵尺寸超出边缘端处理能力");
        }

        // 执行矩阵乘法
        auto computation_start = std::chrono::high_resolution_clock::now();
        if (options.enable_profiling) {
            perf_monitor->startProfiling("matrix_multiplication");
        }

        MatrixMultiplication::Matrix result = multiplier.multiply(A, B);

        if (options.enable_profiling) {
            perf_monitor->endProfiling("matrix_multiplication");
        }
        auto computation_end = std::chrono::high_resolution_clock::now();

        // 计算性能指标
        auto computation_duration = std::chrono::duration_cast<std::chrono::milliseconds>(
            computation_end - computation_start);

        // 创建输出结果
        Json::Value output_result;
        output_result["status"] = "success";
        output_result["algorithm"] = options.algorithm;
        output_result["optimization_level"] = options.optimization_level;

        // 将结果矩阵转换为JSON
        Json::Value result_matrix(Json::arrayValue);
        for (const auto& row : result) {
            Json::Value json_row(Json::arrayValue);
            for (const auto& val : row) {
                json_row.append(val);
            }
            result_matrix.append(json_row);
        }
        output_result["result"] = result_matrix;

        // 获取详细性能指标
        auto perf_metrics = multiplier.getPerformanceMetrics();

        // 添加性能指标
        output_result["performance"]["computation_time_ms"] = static_cast<Json::UInt64>(computation_duration.count());
        output_result["performance"]["operations_count"] = static_cast<Json::UInt64>(perf_metrics.operations_count);
        output_result["performance"]["memory_accesses"] = static_cast<Json::UInt64>(perf_metrics.memory_accesses);
        output_result["performance"]["peak_memory_usage_mb"] = static_cast<Json::UInt64>(perf_metrics.peak_memory_usage / 1024 / 1024);
        output_result["performance"]["cache_efficiency"] = perf_metrics.cache_efficiency;

        // 矩阵尺寸信息
        output_result["performance"]["input_matrix_size"] = Json::Value(Json::arrayValue);
        output_result["performance"]["input_matrix_size"].append(static_cast<Json::UInt64>(A.size()));
        output_result["performance"]["input_matrix_size"].append(static_cast<Json::UInt64>(A[0].size()));
        output_result["performance"]["output_matrix_size"] = Json::Value(Json::arrayValue);
        output_result["performance"]["output_matrix_size"].append(static_cast<Json::UInt64>(result.size()));
        output_result["performance"]["output_matrix_size"].append(static_cast<Json::UInt64>(result[0].size()));

        // 内存使用情况
        output_result["performance"]["estimated_memory_mb"] = static_cast<Json::UInt64>(estimated_memory / 1024 / 1024);
        output_result["performance"]["max_memory_limit_mb"] = static_cast<Json::UInt64>(multiplier.getMemoryLimit());

        // 边缘端优化信息
        output_result["performance"]["optimization_level"] = options.optimization_level;
        output_result["performance"]["cpu_cores_used"] = MatrixMultiplication::EdgeConfig::MAX_THREADS;
        output_result["performance"]["memory_optimized"] = true;

        // 添加元数据
        auto end_time = std::chrono::high_resolution_clock::now();
        auto total_duration = std::chrono::duration_cast<std::chrono::milliseconds>(end_time - start_time);

        output_result["metadata"]["version"] = MATRIX_MULTIPLICATION_VERSION;
        output_result["metadata"]["execution_time_ms"] = static_cast<Json::UInt64>(total_duration.count());
        output_result["metadata"]["timestamp"] = std::chrono::duration_cast<std::chrono::seconds>(
            std::chrono::system_clock::now().time_since_epoch()).count();

        // 写入输出文件
        if (!json_handler.writeJsonFile(options.output_file, output_result)) {
            throw std::runtime_error("无法写入输出文件: " + options.output_file);
        }

        // 输出执行信息
        std::cout << "矩阵乘法执行成功!" << std::endl;
        std::cout << "算法: " << options.algorithm << std::endl;
        std::cout << "输入矩阵大小: " << A.size() << "x" << A[0].size() << std::endl;
        std::cout << "输出矩阵大小: " << result.size() << "x" << result[0].size() << std::endl;
        std::cout << "计算时间: " << computation_duration.count() << "ms" << std::endl;
        std::cout << "总执行时间: " << total_duration.count() << "ms" << std::endl;

        if (options.enable_profiling && perf_monitor) {
            perf_monitor->endProfiling("total_execution");
            perf_monitor->printReport();
        }

        return 0;

    } catch (const std::exception& e) {
        // 错误处理
        std::cerr << "执行失败: " << e.what() << std::endl;

        // 创建错误输出
        Json::Value error_result;
        error_result["status"] = "error";
        error_result["error"] = e.what();
        error_result["timestamp"] = std::chrono::duration_cast<std::chrono::seconds>(
            std::chrono::system_clock::now().time_since_epoch()).count();

        // 尝试写入错误信息到输出文件
        try {
            Json::Value full_result;
            full_result["status"] = "error";
            full_result["error"] = e.what();
            full_result["metadata"]["version"] = MATRIX_MULTIPLICATION_VERSION;
            full_result["metadata"]["timestamp"] = std::chrono::duration_cast<std::chrono::seconds>(
                std::chrono::system_clock::now().time_since_epoch()).count();

            JsonHandler json_handler;
            json_handler.writeJsonFile("/output/result.json", full_result);
        } catch (...) {
            // 如果写入失败，至少在控制台输出错误
            std::cerr << "无法写入错误信息到输出文件" << std::endl;
        }

        return 1;
    }
}
