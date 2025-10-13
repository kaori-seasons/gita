#pragma once

#include <cstdint>
#include <string>
#include <vector>
#include <memory>
#include <map>

// 集成cpp_plugins架构的桥接头文件

// 前向声明cpp_plugins类型
namespace AlgorithmPlugins {
    class IPlugin;
    class PluginManager;
    class PluginParameter;
    class PluginResult;
    class BatchData;
    class FeatureData;
}

// Rust-C++桥接数据结构
struct AlgorithmInput {
    std::string algorithm_name;
    std::string parameters_json;
    std::string device_id;
    uint64_t timestamp_ms;
};

struct AlgorithmOutput {
    bool success;
    std::string result_json;
    std::string error_message;
    uint64_t execution_time_ms;
    uint64_t memory_used_bytes;
};

// 振动数据结构（专门为vibrate31_plugin设计）
struct VibrationData {
    std::vector<double> wave_data;
    std::vector<double> speed_data;
    int sampling_rate;
    std::string device_id;
};

struct VibrationFeatures {
    double mean_hf = 0.0;
    double mean_lf = 0.0;
    double mean = 0.0;
    double std_dev = 0.0;
    double peak_freq = 0.0;
    double peak_power = 0.0;
    double spectrum_energy = 0.0;
    int status = 0;
    double load = 0.0;
};

// C++算法执行器接口（集成cpp_plugins）
class CppAlgorithmExecutor {
public:
    CppAlgorithmExecutor();
    ~CppAlgorithmExecutor();

    // 初始化插件管理器
    bool initialize();

    // 执行通用算法
    AlgorithmOutput execute_algorithm(const AlgorithmInput& input);

    // 统一插件执行接口（所有插件都通过此接口调用）
    // vibrate31等插件会自动识别输入数据类型并处理

    // 获取插件信息
    std::vector<std::string> get_available_plugins() const;
    std::string get_plugin_info(const std::string& plugin_name) const;

    // 插件管理
    bool load_plugin(const std::string& plugin_name);
    bool unload_plugin(const std::string& plugin_name);

private:
    // cpp_plugins集成
    std::shared_ptr<AlgorithmPlugins::PluginManager> plugin_manager_;
    std::shared_ptr<AlgorithmPlugins::IPlugin> vibrate31_plugin_;

    // 内部实现
    class Impl;
    std::unique_ptr<Impl> pimpl_;

    // 辅助方法
    std::shared_ptr<AlgorithmPlugins::PluginParameter> parse_parameters(
        const std::string& parameters_json);

    std::string serialize_result(std::shared_ptr<AlgorithmPlugins::PluginResult> result);

    std::shared_ptr<AlgorithmPlugins::BatchData> create_batch_data(
        const VibrationData& vibration_data);

    std::shared_ptr<AlgorithmPlugins::PluginResult> create_plugin_result();

    // vibrate31专用方法
    AlgorithmOutput execute_vibrate31_internal(
        std::shared_ptr<AlgorithmPlugins::BatchData> batch_data,
        std::shared_ptr<AlgorithmPlugins::PluginParameter> params);
};

// 创建C++算法执行器的工厂函数
std::unique_ptr<CppAlgorithmExecutor> new_cpp_executor();

// 兼容性函数（保留原有接口）
AlgorithmOutput simple_math_add(double a, double b);
AlgorithmOutput simple_math_multiply(double a, double b);
AlgorithmOutput string_reverse(const std::string& input);
AlgorithmOutput data_sort_integers(const std::vector<int32_t>& input);

// 新的生产级API
namespace ProductionAPI {

    // 插件状态信息
    struct PluginStatus {
        std::string plugin_name;
        bool loaded;
        bool initialized;
        std::string version;
        std::string last_error;
        uint64_t execution_count;
        double avg_execution_time_ms;
    };

    // 系统状态信息
    struct SystemStatus {
        uint64_t total_memory_bytes;
        uint64_t used_memory_bytes;
        uint32_t active_plugins;
        uint32_t total_plugins;
        std::string system_health;
    };

    // 获取插件状态
    std::vector<PluginStatus> get_plugin_status();

    // 获取系统状态
    SystemStatus get_system_status();

    // 健康检查
    bool health_check();

    // 性能监控
    struct PerformanceMetrics {
        double cpu_usage_percent;
        uint64_t memory_usage_bytes;
        uint32_t active_threads;
        uint64_t uptime_seconds;
    };

    PerformanceMetrics get_performance_metrics();

} // namespace ProductionAPI
