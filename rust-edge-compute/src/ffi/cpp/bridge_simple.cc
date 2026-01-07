#include "rust-edge-compute/src/ffi/bridge.rs.h"  // CXX生成的头文件
#include "../../cpp_plugins/include/plugin_manager.h"
#include "../../cpp_plugins/include/vibrate31_plugin.h"
#include "../../cpp_plugins/include/evaluation_plugin_base.h"
#include "../../cpp_plugins/include/data_types.h"
#include <iostream>
#include <vector>
#include <string>
#include <sstream>
#include <chrono>
#include <map>
#include <memory>
#include <algorithm>
#include <numeric>
#include <cmath>

/*
 * ============================================================================
 * 三层架构设计（按照评估文档）
 * ============================================================================
 * 
 * Layer 1: Rust FFI Bridge (bridge.rs)
 *   - 接收 ComputeRequest
 *   - 类型转换：Rust types → C++ types
 *   - 生命周期管理
 * 
 * Layer 2: C++ Bridge Abstraction (此文件 - bridge_simple.cc)
 *   - PluginExecutor：统一的插件调用抽象层
 *   - execute_plugin(name, params)：通用插件执行接口
 *   - JSON ↔ PluginData 转换器
 *   - 不包含任何算法实现代码
 * 
 * Layer 3: Concrete Plugins (cpp_plugins/)
 *   - Vibrate31Plugin：振动特征提取
 *   - Error18Plugin：错误检测
 *   - CompRealtimeHealth34Plugin：健康评估
 * ============================================================================
 */

using namespace AlgorithmPlugins;

// 声明cpp_plugins提供的插件注册函数
extern "C" {
    void register_algorithm_plugins();
}

/**
 * @brief 插件执行器 - Bridge层的顶层抽象
 * 
 * 职责：
 * 1. 管理插件生命周期
 * 2. 提供统一的插件调用接口
 * 3. 数据格式转换（JSON ↔ PluginData）
 * 4. 不实现任何算法逻辑
 */
class PluginExecutor {
public:
    struct PluginRequest {
        std::string plugin_name;
        std::string params_json;
        std::string device_id;
        uint64_t timestamp_ms;
    };
    
    struct PluginResponse {
        bool success;
        std::string result_json;
        std::string error_message;
        uint64_t execution_time_ms;
    };
    
    PluginExecutor() {
        std::cout << "[PluginExecutor] Initializing plugin system..." << std::endl;
    }
    
    ~PluginExecutor() {
        cleanup();
    }
    
    bool initialize() {
        try {
            std::cout << "[PluginExecutor] About to test Vibrate31Plugin instantiation..." << std::endl;
            auto test_plugin = std::make_shared<Vibrate31Plugin>();
            std::cout << "[PluginExecutor] Vibrate31Plugin created successfully: " << test_plugin->getName() << std::endl;
            
            std::cout << "[PluginExecutor] About to call register_algorithm_plugins..." << std::endl;
            // 调用cpp_plugins提供的注册函数
            register_algorithm_plugins();
            std::cout << "[PluginExecutor] register_algorithm_plugins() completed" << std::endl;
            
            std::cout << "[PluginExecutor] Plugin system initialized successfully" << std::endl;
            return true;
        } catch (const std::exception& e) {
            std::cerr << "[PluginExecutor] Initialization failed: " << e.what() << std::endl;
            return false;
        }
    }
    
    void cleanup() {
        plugins_.clear();
        plugin_params_.clear();
        std::cout << "[PluginExecutor] Plugin system cleaned up" << std::endl;
    }
    
    /**
     * @brief 核心抽象：执行任意插件
     */
    PluginResponse execute_plugin(const PluginRequest& request) {
        auto start_time = std::chrono::high_resolution_clock::now();
        PluginResponse response;
        
        try {
            // 查找插件
            auto it = plugins_.find(request.plugin_name);
            if (it == plugins_.end()) {
                response.success = false;
                response.error_message = "Plugin not found: " + request.plugin_name;
                return response;
            }
            
            auto& plugin = it->second;
            
            // 转换输入数据
            auto input_data = json_to_plugin_data(request.params_json, plugin->getType());
            if (!input_data) {
                response.success = false;
                response.error_message = "Failed to parse input data";
                return response;
            }
            
            // 创建输出容器
            auto output_result = create_plugin_result();
            
            // 执行插件
            bool success = false;
            if (plugin->getType() == PluginType::FEATURE) {
                auto feature_plugin = std::dynamic_pointer_cast<FeaturePluginBase>(plugin);
                success = feature_plugin->extractFeatures(input_data, output_result);
            } else if (plugin->getType() == PluginType::EVALUATION) {
                auto eval_plugin = std::dynamic_pointer_cast<EvaluationPluginBase>(plugin);
                success = eval_plugin->evaluateHealth(input_data, output_result);
            } else {
                // 使用通用的process方法
                success = plugin->process(input_data, output_result);
            }
            
            // 转换输出数据
            response.success = success;
            response.result_json = plugin_result_to_json(output_result, success);
            response.error_message = success ? "" : plugin->getLastError();
            
        } catch (const std::exception& e) {
            response.success = false;
            response.error_message = std::string("Exception: ") + e.what();
        }
        
        auto end_time = std::chrono::high_resolution_clock::now();
        response.execution_time_ms = std::chrono::duration_cast<std::chrono::milliseconds>(
            end_time - start_time).count();
        
        return response;
    }
    
    std::vector<std::string> get_available_plugins() const {
        std::vector<std::string> names;
        for (const auto& pair : plugins_) {
            names.push_back(pair.first);
        }
        return names;
    }

private:
    std::map<std::string, std::shared_ptr<IPlugin>> plugins_;
    std::map<std::string, std::shared_ptr<PluginParameter>> plugin_params_;
    
    /**
     * @brief 注册插件（Bridge层：纯粹的注册动作，不涉及插件实例化）
     * 
     * Bridge层职责：
     * - 调用 cpp_plugins 的工厂注册所有插件
     * - 不创建任何插件实例
     * - 不包含任何计算逻辑
     */
    /**
     * @brief JSON → PluginData 转换
     */
    std::shared_ptr<PluginData> json_to_plugin_data(const std::string& json, PluginType type) {
        try {
            auto timestamp = std::chrono::system_clock::now();
            
            if (type == PluginType::FEATURE) {
                // 振动特征提取需要批次数据
                auto batch_data = std::make_shared<BatchData>("device_001", timestamp);
                
                // 解析 wave_data
                std::vector<double> wave_data = parse_json_array(json, "wave_data");
                batch_data->setWaveData(wave_data);
                
                // 解析 sampling_rate
                int sampling_rate = parse_json_int(json, "sampling_rate", 1000);
                batch_data->setSamplingRate(sampling_rate);
                
                // 解析 speed_data（可选）
                std::vector<double> speed_data = parse_json_array(json, "speed_data");
                batch_data->setSpeedData(speed_data);
                
                return batch_data;
            } else {
                // 其他插件使用特征数据
                auto feature_data = std::make_shared<FeatureData>("device_001", timestamp);
                // TODO: 解析特征数据
                return feature_data;
            }
        } catch (const std::exception& e) {
            std::cerr << "[PluginExecutor] JSON parse error: " << e.what() << std::endl;
            return nullptr;
        }
    }
    
    /**
     * @brief PluginResult → JSON 转换
     */
    std::string plugin_result_to_json(std::shared_ptr<PluginResult> result, bool success) {
        std::ostringstream ss;
        ss << "{\n";
        ss << "  \"success\": " << (success ? "true" : "false") << ",\n";
        
        if (success && result) {
            ss << "  \"data\": " << result->serialize() << "\n";
        } else {
            ss << "  \"data\": {}\n";
        }
        
        ss << "}";
        return ss.str();
    }
    
    /**
     * @brief 创建默认参数
     */
    std::shared_ptr<PluginParameter> create_default_params(const std::string& plugin_name);
    
    /**
     * @brief 创建插件结果容器
     */
    std::shared_ptr<PluginResult> create_plugin_result();
    
    /**
     * @brief 解析JSON数组
     */
    std::vector<double> parse_json_array(const std::string& json, const std::string& key) {
        std::vector<double> result;
        size_t key_pos = json.find("\"" + key + "\"");
        if (key_pos == std::string::npos) return result;
        
        size_t array_start = json.find("[", key_pos);
        size_t array_end = json.find("]", array_start);
        if (array_start == std::string::npos || array_end == std::string::npos) return result;
        
        std::string array_str = json.substr(array_start + 1, array_end - array_start - 1);
        std::stringstream ss(array_str);
        std::string item;
        
        while (std::getline(ss, item, ',')) {
            try {
                result.push_back(std::stod(item));
            } catch (...) {}
        }
        
        return result;
    }
    
    /**
     * @brief 解析JSON整数
     */
    int parse_json_int(const std::string& json, const std::string& key, int default_value) {
        size_t key_pos = json.find("\"" + key + "\"");
        if (key_pos == std::string::npos) return default_value;
        
        size_t colon_pos = json.find(":", key_pos);
        if (colon_pos == std::string::npos) return default_value;
        
        size_t num_start = json.find_first_of("0123456789", colon_pos);
        if (num_start == std::string::npos) return default_value;
        
        size_t num_end = json.find_first_not_of("0123456789", num_start);
        std::string num_str = json.substr(num_start, num_end - num_start);
        
        try {
            return std::stoi(num_str);
        } catch (...) {
            return default_value;
        }
    }
};

// PluginParameter 和 PluginResult 的具体实现
class SimplePluginParameter : public PluginParameter {
public:
    std::string getString(const std::string& key, const std::string& defaultValue = "") const override {
        auto it = string_params_.find(key);
        return (it != string_params_.end()) ? it->second : defaultValue;
    }
    
    double getDouble(const std::string& key, double defaultValue = 0.0) const override {
        auto it = double_params_.find(key);
        return (it != double_params_.end()) ? it->second : defaultValue;
    }
    
    int getInt(const std::string& key, int defaultValue = 0) const override {
        auto it = int_params_.find(key);
        return (it != int_params_.end()) ? it->second : defaultValue;
    }
    
    bool getBool(const std::string& key, bool defaultValue = false) const override {
        auto it = bool_params_.find(key);
        return (it != bool_params_.end()) ? it->second : defaultValue;
    }
    
    std::vector<double> getDoubleArray(const std::string& key) const override {
        auto it = double_array_params_.find(key);
        return (it != double_array_params_.end()) ? it->second : std::vector<double>();
    }
    
    std::vector<int> getIntArray(const std::string& key) const override {
        auto it = int_array_params_.find(key);
        return (it != int_array_params_.end()) ? it->second : std::vector<int>();
    }
    
    std::vector<std::string> getStringArray(const std::string& key) const override {
        auto it = string_array_params_.find(key);
        return (it != string_array_params_.end()) ? it->second : std::vector<std::string>();
    }
    
    void setString(const std::string& key, const std::string& value) override { string_params_[key] = value; }
    void setDouble(const std::string& key, double value) override { double_params_[key] = value; }
    void setInt(const std::string& key, int value) override { int_params_[key] = value; }
    void setBool(const std::string& key, bool value) override { bool_params_[key] = value; }
    void setDoubleArray(const std::string& key, const std::vector<double>& value) override { double_array_params_[key] = value; }
    void setIntArray(const std::string& key, const std::vector<int>& value) override { int_array_params_[key] = value; }
    void setStringArray(const std::string& key, const std::vector<std::string>& value) override { string_array_params_[key] = value; }
    
    std::string serialize() const override { return "{}"; }
    bool deserialize(const std::string& data) override { return true; }
    
private:
    std::map<std::string, std::string> string_params_;
    std::map<std::string, double> double_params_;
    std::map<std::string, int> int_params_;
    std::map<std::string, bool> bool_params_;
    std::map<std::string, std::vector<double>> double_array_params_;
    std::map<std::string, std::vector<int>> int_array_params_;
    std::map<std::string, std::vector<std::string>> string_array_params_;
};

class SimplePluginResult : public PluginResult {
public:
    void setData(const std::string& key, const std::string& value) override { string_data_[key] = value; }
    void setData(const std::string& key, double value) override { double_data_[key] = value; }
    void setData(const std::string& key, int value) override { int_data_[key] = value; }
    
    std::string getStringData(const std::string& key) const override {
        auto it = string_data_.find(key);
        return (it != string_data_.end()) ? it->second : "";
    }
    
    double getDoubleData(const std::string& key) const override {
        auto it = double_data_.find(key);
        return (it != double_data_.end()) ? it->second : 0.0;
    }
    
    int getIntData(const std::string& key) const override {
        auto it = int_data_.find(key);
        return (it != int_data_.end()) ? it->second : 0;
    }
    
    bool hasData(const std::string& key) const override {
        return string_data_.find(key) != string_data_.end() ||
               double_data_.find(key) != double_data_.end() ||
               int_data_.find(key) != int_data_.end();
    }
    
    std::string serialize() const override {
        std::ostringstream ss;
        ss << "{";
        bool first = true;
        for (const auto& pair : double_data_) {
            if (!first) ss << ",";
            ss << "\"" << pair.first << "\":" << pair.second;
            first = false;
        }
        ss << "}";
        return ss.str();
    }
    
    bool deserialize(const std::string& data) override { return true; }
    
private:
    std::map<std::string, std::string> string_data_;
    std::map<std::string, double> double_data_;
    std::map<std::string, int> int_data_;
};

std::shared_ptr<PluginParameter> PluginExecutor::create_default_params(const std::string& plugin_name) {
    auto params = std::make_shared<SimplePluginParameter>();
    
    if (plugin_name == "vibrate31") {
        params->setInt("sampling_rate", 1000);
        params->setInt("duration_limit", 10);
        params->setDouble("dc_threshold", 500.0);
    } else if (plugin_name == "error18") {
        params->setBool("auto", false);
        params->setInt("error_width", 30);
    } else if (plugin_name == "evaluation") {
        params->setInt("offline_length", 86400 * 15);
        params->setInt("minimum_quantity", 30);
    }
    
    return params;
}

std::shared_ptr<PluginResult> PluginExecutor::create_plugin_result() {
    return std::make_shared<SimplePluginResult>();
}

// CppAlgorithmExecutor 实现 - 使用 PluginExecutor 抽象层

CppAlgorithmExecutor::CppAlgorithmExecutor() 
    : plugin_executor_(std::make_unique<PluginExecutor>()) {
    std::cout << "[CppAlgorithmExecutor] Created with plugin-based architecture" << std::endl;
}

CppAlgorithmExecutor::~CppAlgorithmExecutor() {
    std::cout << "[CppAlgorithmExecutor] Destroyed" << std::endl;
}

bool CppAlgorithmExecutor::initialize() const {
    bool success = plugin_executor_->initialize();
    std::cout << "[CppAlgorithmExecutor] Initialized: " << (success ? "success" : "failed") << std::endl;
    return success;
}

AlgorithmOutput CppAlgorithmExecutor::execute_algorithm(const AlgorithmInput& input) const {
    // 使用 PluginExecutor 抽象层执行插件
    std::string algo_name(input.algorithm_name.data(), input.algorithm_name.size());
    std::string params_json(input.parameters_json.data(), input.parameters_json.size());
    std::string device_id(input.device_id.data(), input.device_id.size());
    
    std::cout << "[CppAlgorithmExecutor] Executing plugin: " << algo_name << std::endl;
    
    // 构建插件请求
    PluginExecutor::PluginRequest req{
        .plugin_name = algo_name,
        .params_json = params_json,
        .device_id = device_id,
        .timestamp_ms = input.timestamp_ms
    };
    
    // 执行插件
    auto resp = plugin_executor_->execute_plugin(req);
    
    // 构建输出
    AlgorithmOutput output;
    output.success = resp.success;
    output.result_json = resp.result_json;
    output.error_message = resp.error_message;
    output.execution_time_ms = resp.execution_time_ms;
    output.memory_used_bytes = 1024 * 1024; // 1MB 基础内存
    
    return output;
}

std::vector<std::string> CppAlgorithmExecutor::get_available_plugins() const {
    return plugin_executor_->get_available_plugins();
}

std::string CppAlgorithmExecutor::get_plugin_info(const std::string& plugin_name) const {
    auto plugins = plugin_executor_->get_available_plugins();
    bool found = std::find(plugins.begin(), plugins.end(), plugin_name) != plugins.end();
    
    std::ostringstream ss;
    ss << "{";
    ss << "\"name\": \"" << plugin_name << "\",";
    ss << "\"version\": \"1.0.0\",";
    ss << "\"available\": " << (found ? "true" : "false") << ",";
    ss << "\"description\": \"Plugin-based algorithm implementation\"";
    ss << "}";
    return ss.str();
}

bool CppAlgorithmExecutor::load_plugin(const std::string& plugin_name) {
    std::cout << "[CppAlgorithmExecutor] Load plugin: " << plugin_name << std::endl;
    return true;
}

bool CppAlgorithmExecutor::unload_plugin(const std::string& plugin_name) {
    std::cout << "[CppAlgorithmExecutor] Unload plugin: " << plugin_name << std::endl;
    return true;
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
