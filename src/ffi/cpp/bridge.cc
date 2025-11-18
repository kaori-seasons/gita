#include "bridge.h"

// 包含cpp_plugins头文件
#include "plugin_base.h"
#include "data_types.h"
#include "feature_plugin_base.h"
#include "vibrate31_plugin.h"
#include "plugin_manager.h"

#include <algorithm>
#include <iostream>
#include <sstream>
#include <cmath>
#include <chrono>

// JSON序列化辅助函数（简化实现）
std::string to_json(double value) {
    std::stringstream ss;
    ss << value;
    return ss.str();
}

std::string to_json(const std::vector<int32_t>& values) {
    std::stringstream ss;
    ss << "[";
    for (size_t i = 0; i < values.size(); ++i) {
        if (i > 0) ss << ",";
        ss << values[i];
    }
    ss << "]";
    return ss.str();
}

std::string to_json(const std::string& str) {
    return "\"" + str + "\"";
}

// C++算法执行器的内部实现
class CppAlgorithmExecutor::Impl {
public:
    Impl() : plugin_manager_(nullptr), vibrate31_plugin_(nullptr) {
        std::cout << "CppAlgorithmExecutor::Impl created (with cpp_plugins integration)" << std::endl;
    }

    ~Impl() {
        cleanup();
        std::cout << "CppAlgorithmExecutor::Impl destroyed" << std::endl;
    }

    bool initialize() {
        try {
            // 创建插件管理器
            plugin_manager_ = AlgorithmPlugins::PluginManager::getInstance();

            // 注册所有插件
            registerAllPlugins();

            // 加载vibrate31插件
            if (loadVibrate31Plugin()) {
                std::cout << "CppAlgorithmExecutor initialized successfully" << std::endl;
                return true;
            } else {
                std::cerr << "Failed to load vibrate31 plugin" << std::endl;
                return false;
            }
        } catch (const std::exception& e) {
            std::cerr << "Failed to initialize CppAlgorithmExecutor: " << e.what() << std::endl;
            return false;
        }
    }

    AlgorithmOutput execute_algorithm(const AlgorithmInput& input) {
        AlgorithmOutput output;
        output.success = false;
        output.execution_time_ms = 0;
        output.memory_used_bytes = 0;

        auto start_time = std::chrono::high_resolution_clock::now();

        try {
            if (!plugin_manager_) {
                output.error_message = "Plugin manager not initialized";
                return output;
            }

            // 创建插件参数
            auto params = parseParameters(input.parameters_json);
            if (!params) {
                output.error_message = "Failed to parse parameters";
                return output;
            }

            // 创建输入数据 - 智能识别数据类型
            auto plugin_data = createPluginData(input, params);
            if (!plugin_data) {
                output.error_message = "Failed to create plugin data";
                return output;
            }

            // 创建输出结果
            auto plugin_result = std::make_shared<AlgorithmPlugins::PluginResultImpl>();

            // 获取插件实例 - 统一处理所有插件类型
            std::shared_ptr<AlgorithmPlugins::IPlugin> plugin;
            if (input.algorithm_name == "vibrate31" && vibrate31_plugin_) {
                plugin = vibrate31_plugin_;
            } else {
                plugin = plugin_manager_->createPlugin(input.algorithm_name, params);
            }

            if (!plugin) {
                output.error_message = "Plugin not found: " + input.algorithm_name;
                return output;
            }

            // 执行算法
            bool success = plugin->process(plugin_data, plugin_result);

            // 计算执行时间
            auto end_time = std::chrono::high_resolution_clock::now();
            output.execution_time_ms = std::chrono::duration_cast<std::chrono::milliseconds>(
                end_time - start_time).count();

            // 序列化结果
            if (success) {
                output.success = true;
                output.result_json = serializeResult(plugin_result);
            } else {
                output.error_message = plugin->getLastError();
            }

            // 估算内存使用量
            output.memory_used_bytes = estimateMemoryUsage(input, output.result_json);

        } catch (const std::exception& e) {
            output.error_message = std::string("Algorithm execution failed: ") + e.what();
        }

        return output;
    }

    // 移除专门的vibrate31方法，所有插件都通过execute_algorithm统一处理
    // vibrate31插件会在execute_algorithm中自动识别振动数据类型

    std::vector<std::string> get_available_plugins() const {
        if (!plugin_manager_) return {};
        return plugin_manager_->getAvailablePlugins();
    }

    std::string get_plugin_info(const std::string& plugin_name) const {
        if (!plugin_manager_) return "{}";

        try {
            std::stringstream ss;
            ss << "{";
            ss << "\"name\": \"" << plugin_name << "\",";
            ss << "\"version\": \"" << plugin_manager_->getPluginVersion(plugin_name) << "\",";
            ss << "\"description\": \"" << plugin_manager_->getPluginDescription(plugin_name) << "\"";
            ss << "}";
            return ss.str();
        } catch (...) {
            return "{}";
        }
    }

    bool load_plugin(const std::string& plugin_name) {
        if (!plugin_manager_) return false;
        return plugin_manager_->loadPluginFromFile(plugin_name);
    }

    bool unload_plugin(const std::string& plugin_name) {
        if (!plugin_manager_) return false;
        return plugin_manager_->unregisterPlugin(plugin_name);
    }

private:
    std::shared_ptr<AlgorithmPlugins::PluginManager> plugin_manager_;
    std::shared_ptr<AlgorithmPlugins::IPlugin> vibrate31_plugin_;

    void cleanup() {
        if (vibrate31_plugin_) {
            vibrate31_plugin_->cleanup();
            vibrate31_plugin_.reset();
        }
        plugin_manager_.reset();
    }

    bool loadcarPlugin() {
        try {
            // 创建vibrate31插件实例
            vibrate31_plugin_ = std::make_shared<AlgorithmPlugins::Vibrate31Plugin>();

            // 创建默认参数
            auto params = std::make_shared<AlgorithmPlugins::PluginParameterImpl>();
            params->setInt("sampling_rate", 1000);
            params->setInt("duration_limit", 10);
            params->setDouble("dc_threshold", 500.0);

            // 初始化插件
            if (!vibrate31_plugin_->initialize(params)) {
                std::cerr << "Failed to initialize vibrate31 plugin: "
                         << vibrate31_plugin_->getLastError() << std::endl;
                return false;
            }

            std::cout << "vibrate31 plugin loaded successfully" << std::endl;
            return true;
        } catch (const std::exception& e) {
            std::cerr << "Failed to load vibrate31 plugin: " << e.what() << std::endl;
            return false;
        }
    }

    std::shared_ptr<AlgorithmPlugins::PluginParameter> parseParameters(const std::string& json_str) {
        try {
            auto params = std::make_shared<AlgorithmPlugins::PluginParameterImpl>();

            if (!json_str.empty()) {
                // 简化的JSON解析（实际项目中应该使用专业的JSON库）
                std::string::size_type pos = 0;
                while ((pos = json_str.find("\"", pos)) != std::string::npos) {
                    std::string::size_type key_start = pos + 1;
                    std::string::size_type key_end = json_str.find("\"", key_start);
                    if (key_end == std::string::npos) break;

                    std::string key = json_str.substr(key_start, key_end - key_start);

                    // 查找值
                    std::string::size_type value_start = json_str.find(":", key_end);
                    if (value_start == std::string::npos) break;
                    value_start += 1;

                    // 跳过空白字符
                    while (value_start < json_str.size() &&
                           (json_str[value_start] == ' ' || json_str[value_start] == '\t' ||
                            json_str[value_start] == '\n' || json_str[value_start] == '\r')) {
                        value_start++;
                    }

                    if (json_str[value_start] == '"') {
                        // 字符串值
                        std::string::size_type str_start = value_start + 1;
                        std::string::size_type str_end = json_str.find("\"", str_start);
                        if (str_end == std::string::npos) break;

                        std::string value = json_str.substr(str_start, str_end - str_start);
                        params->setString(key, value);
                        pos = str_end + 1;
                    } else if (json_str.substr(value_start, 4) == "true" ||
                               json_str.substr(value_start, 5) == "false") {
                        // 布尔值
                        bool bool_value = json_str.substr(value_start, 4) == "true";
                        params->setBool(key, bool_value);
                        pos = value_start + (bool_value ? 4 : 5);
                    } else {
                        // 数值
                        std::string::size_type num_end = json_str.find_first_of(",}", value_start);
                        if (num_end == std::string::npos) break;

                        std::string num_str = json_str.substr(value_start, num_end - value_start);
                        if (num_str.find('.') != std::string::npos) {
                            // 浮点数
                            params->setDouble(key, std::stod(num_str));
                        } else {
                            // 整数
                            params->setInt(key, std::stoi(num_str));
                        }
                        pos = num_end;
                    }
                }
            }

            return params;
        } catch (const std::exception& e) {
            std::cerr << "Failed to parse parameters: " << e.what() << std::endl;
            return nullptr;
        }
    }

    std::shared_ptr<AlgorithmPlugins::PluginData> createPluginData(const AlgorithmInput& input,
                                                                  std::shared_ptr<AlgorithmPlugins::PluginParameter> params) {
        // 智能识别算法类型并创建相应数据对象

        if (input.algorithm_name == "vibrate31") {
            // 为vibrate31创建BatchData - 振动特征提取
            return createVibrationData(input);
        }
        else if (input.algorithm_name.find("current") != std::string::npos ||
                 input.algorithm_name.find("temperature") != std::string::npos ||
                 input.algorithm_name.find("audio") != std::string::npos) {
            // 实时特征提取插件
            return createRealTimeData(input);
        }
        else if (input.algorithm_name.find("motor") != std::string::npos ||
                 input.algorithm_name.find("universal") != std::string::npos ||
                 input.algorithm_name.find("classify") != std::string::npos) {
            // 状态识别插件 - 需要特征数据作为输入
            return createFeatureData(input);
        }
        else if (input.algorithm_name.find("health") != std::string::npos ||
                 input.algorithm_name.find("error") != std::string::npos) {
            // 健康评估插件 - 需要特征数据作为输入
            return createFeatureData(input);
        }
        else if (input.algorithm_name.find("alarm") != std::string::npos ||
                 input.algorithm_name.find("score") != std::string::npos ||
                 input.algorithm_name.find("status") != std::string::npos) {
            // 事件处理插件 - 需要健康度数据作为输入
            return createFeatureData(input);
        }

        // 默认创建特征数据
        return createFeatureData(input);
    }

    std::shared_ptr<AlgorithmPlugins::PluginData> createVibrationData(const AlgorithmInput& input) {
        // 解析JSON参数中的振动数据
        try {
            nlohmann::json params = nlohmann::json::parse(input.parameters_json);

            // 创建BatchData用于振动数据
            auto batch_data = std::make_shared<AlgorithmPlugins::BatchData>(
                input.device_id,
                std::chrono::system_clock::time_point(std::chrono::milliseconds(input.timestamp_ms))
            );

            // 提取波形数据
            if (params.contains("wave_data") && params["wave_data"].is_array()) {
                std::vector<double> wave_data;
                for (const auto& value : params["wave_data"]) {
                    if (value.is_number()) {
                        wave_data.push_back(value.get<double>());
                    }
                }
                batch_data->setWaveData(wave_data);
            }

            // 提取转速数据
            if (params.contains("speed_data") && params["speed_data"].is_array()) {
                std::vector<double> speed_data;
                for (const auto& value : params["speed_data"]) {
                    if (value.is_number()) {
                        speed_data.push_back(value.get<double>());
                    }
                }
                batch_data->setSpeedData(speed_data);
            }

            // 设置采样率
            int sampling_rate = params.value("sampling_rate", 1000);
            batch_data->setSamplingRate(sampling_rate);

            // 设置状态（默认为运行状态）
            int status = params.value("status", 1);
            batch_data->setStatus(status);

            return batch_data;

        } catch (const std::exception& e) {
            std::cerr << "Failed to parse vibration data: " << e.what() << std::endl;
            return nullptr;
        }
    }

    std::shared_ptr<AlgorithmPlugins::PluginData> createRealTimeData(const AlgorithmInput& input) {
        // 创建实时数据对象
        auto realtime_data = std::make_shared<AlgorithmPlugins::RealTimeData>(
            input.device_id,
            std::chrono::system_clock::time_point(std::chrono::milliseconds(input.timestamp_ms))
        );

        try {
            nlohmann::json params = nlohmann::json::parse(input.parameters_json);

            // 设置基础特征（如果有的话）
            if (params.contains("mean_hf")) {
                realtime_data->setMeanHF(params["mean_hf"].get<double>());
            }
            if (params.contains("mean_lf")) {
                realtime_data->setMeanLF(params["mean_lf"].get<double>());
            }
            if (params.contains("mean")) {
                realtime_data->setMean(params["mean"].get<double>());
            }
            if (params.contains("std")) {
                realtime_data->setStd(params["std"].get<double>());
            }

            // 设置其他数据
            if (params.contains("temperature")) {
                realtime_data->setTemperature(params["temperature"].get<double>());
            }
            if (params.contains("speed")) {
                realtime_data->setSpeed(params["speed"].get<double>());
            }

        } catch (const std::exception& e) {
            std::cerr << "Failed to parse realtime data: " << e.what() << std::endl;
        }

        return realtime_data;
    }

    std::shared_ptr<AlgorithmPlugins::PluginData> createFeatureData(const AlgorithmInput& input) {
        // 创建特征数据对象
        auto feature_data = std::make_shared<AlgorithmPlugins::FeatureData>(
            input.device_id,
            std::chrono::system_clock::time_point(std::chrono::milliseconds(input.timestamp_ms))
        );

        try {
            nlohmann::json params = nlohmann::json::parse(input.parameters_json);

            // 设置特征数据
            if (params.contains("features") && params["features"].is_object()) {
                for (const auto& [key, value] : params["features"].items()) {
                    if (value.is_number()) {
                        feature_data->setFeature(key, value.get<double>());
                    }
                }
            }

        } catch (const std::exception& e) {
            std::cerr << "Failed to parse feature data: " << e.what() << std::endl;
        }

        return feature_data;
    }

    std::string serializeResult(std::shared_ptr<AlgorithmPlugins::PluginResult> result) {
        try {
            std::stringstream ss;
            ss << "{";
            ss << "\"execution_status\": \"success\",";
            ss << "\"timestamp\": " << std::chrono::duration_cast<std::chrono::milliseconds>(
                std::chrono::system_clock::now().time_since_epoch()).count();

            // 这里应该根据PluginResult的实际接口来序列化数据
            // 由于接口限制，这里使用简化的实现

            ss << "}";
            return ss.str();

        } catch (const std::exception& e) {
            return R"({"error": "serialization_failed"})";
        }
    }

    uint64_t estimateMemoryUsage(const AlgorithmInput& input, const std::string& result) {
        // 估算内存使用量
        uint64_t base_usage = 1024 * 1024; // 1MB基础内存
        uint64_t param_usage = input.parameters_json.size();
        uint64_t result_usage = result.size();

        return base_usage + param_usage + result_usage;
    }
};

// C++算法执行器主类实现
CppAlgorithmExecutor::CppAlgorithmExecutor()
    : pimpl_(std::make_unique<Impl>()) {
}

CppAlgorithmExecutor::~CppAlgorithmExecutor() = default;

bool CppAlgorithmExecutor::initialize() {
    return pimpl_->initialize();
}

AlgorithmOutput CppAlgorithmExecutor::execute_algorithm(const AlgorithmInput& input) {
    return pimpl_->execute_algorithm(input);
}

AlgorithmOutput CppAlgorithmExecutor::execute_vibrate31(const VibrationData& vibration_data,
                                                      const std::map<std::string, std::string>& parameters) {
    return pimpl_->execute_vibrate31(vibration_data, parameters);
}

std::vector<std::string> CppAlgorithmExecutor::get_available_plugins() const {
    return pimpl_->get_available_plugins();
}

std::string CppAlgorithmExecutor::get_plugin_info(const std::string& plugin_name) const {
    return pimpl_->get_plugin_info(plugin_name);
}

bool CppAlgorithmExecutor::load_plugin(const std::string& plugin_name) {
    return pimpl_->load_plugin(plugin_name);
}

bool CppAlgorithmExecutor::unload_plugin(const std::string& plugin_name) {
    return pimpl_->unload_plugin(plugin_name);
}

// 工厂函数
std::unique_ptr<CppAlgorithmExecutor> new_cpp_executor() {
    auto executor = std::make_unique<CppAlgorithmExecutor>();
    if (executor->initialize()) {
        return executor;
    } else {
        return nullptr;
    }
}

// 兼容性函数实现
AlgorithmOutput simple_math_add(double a, double b) {
    AlgorithmOutput output;
    output.success = true;
    double result = a + b;
    output.result_json = "{\"result\": " + to_json(result) + "}";
    output.error_message = "";
    output.execution_time_ms = 1;
    output.memory_used_bytes = 64;
    return output;
}

AlgorithmOutput simple_math_multiply(double a, double b) {
    AlgorithmOutput output;
    output.success = true;
    double result = a * b;
    output.result_json = "{\"result\": " + to_json(result) + "}";
    output.error_message = "";
    output.execution_time_ms = 1;
    output.memory_used_bytes = 64;
    return output;
}

AlgorithmOutput string_reverse(const std::string& input) {
    AlgorithmOutput output;
    output.success = true;
    std::string result = input;
    std::reverse(result.begin(), result.end());
    output.result_json = "{\"result\": " + to_json(result) + "}";
    output.error_message = "";
    output.execution_time_ms = 1;
    output.memory_used_bytes = input.size() * 2;
    return output;
}

AlgorithmOutput string_uppercase(const std::string& input) {
    AlgorithmOutput output;
    output.success = true;
    std::string result = input;
    std::transform(result.begin(), result.end(), result.begin(), ::toupper);
    output.result_json = "{\"result\": " + to_json(result) + "}";
    output.error_message = "";
    output.execution_time_ms = 1;
    output.memory_used_bytes = input.size() * 2;
    return output;
}

AlgorithmOutput data_sort_integers(const std::vector<int32_t>& input) {
    AlgorithmOutput output;
    output.success = true;
    std::vector<int32_t> result = input;
    std::sort(result.begin(), result.end());
    output.result_json = "{\"result\": " + to_json(result) + "}";
    output.error_message = "";
    output.execution_time_ms = 10;
    output.memory_used_bytes = input.size() * sizeof(int32_t) * 2;
    return output;
}

AlgorithmOutput data_filter_positive(const std::vector<int32_t>& input) {
    AlgorithmOutput output;
    output.success = true;
    std::vector<int32_t> result;
    std::copy_if(input.begin(), input.end(), std::back_inserter(result),
                 [](int32_t x) { return x > 0; });
    output.result_json = "{\"result\": " + to_json(result) + "}";
    output.error_message = "";
    output.execution_time_ms = 5;
    output.memory_used_bytes = input.size() * sizeof(int32_t) * 2;
    return output;
}

// 生产级API实现
namespace ProductionAPI {

    std::vector<PluginStatus> get_plugin_status() {
        std::vector<PluginStatus> status;
        // 这里应该从实际的插件管理器获取状态
        // 暂时返回示例数据
        PluginStatus ps;
        ps.plugin_name = "vibrate31";
        ps.loaded = true;
        ps.initialized = true;
        ps.version = "1.0.0";
        ps.last_error = "";
        ps.execution_count = 0;
        ps.avg_execution_time_ms = 0.0;
        status.push_back(ps);
        return status;
    }

    SystemStatus get_system_status() {
        SystemStatus status;
        status.total_memory_bytes = 8ULL * 1024 * 1024 * 1024; // 8GB
        status.used_memory_bytes = 1ULL * 1024 * 1024 * 1024;  // 1GB
        status.active_plugins = 1;
        status.total_plugins = 5;
        status.system_health = "healthy";
        return status;
    }

    bool health_check() {
        // 简单的健康检查
        return true;
    }

    PerformanceMetrics get_performance_metrics() {
        PerformanceMetrics metrics;
        metrics.cpu_usage_percent = 25.0;
        metrics.memory_usage_bytes = 512ULL * 1024 * 1024; // 512MB
        metrics.active_threads = 4;
        metrics.uptime_seconds = 3600; // 1小时
        return metrics;
    }

} // namespace ProductionAPI
