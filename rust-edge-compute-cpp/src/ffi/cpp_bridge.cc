#include "cpp_bridge.h"
#include "json_parser.h"
#include <sstream>
#include <cmath>
#include <chrono>

CppAlgorithmExecutor::CppAlgorithmExecutor() : initialized_(false) {}

CppAlgorithmExecutor::~CppAlgorithmExecutor() {}

bool CppAlgorithmExecutor::initialize() {
    // 初始化cpp_plugins插件管理器
    // 注意：这里需要根据实际的插件管理器接口进行调整
    // 当前实现提供框架，实际使用时需要调用PluginManager::initialize()
    
    try {
        // TODO: 实际集成时，应该调用：
        // plugin_manager_ = std::make_shared<AlgorithmPlugins::PluginManager>();
        // if (!plugin_manager_->initialize()) {
        //     return false;
        // }
        
        initialized_ = true;
        return true;
    } catch (const std::exception& e) {
        // 记录错误（实际应该使用日志系统）
        return false;
    }
}

AlgorithmOutput CppAlgorithmExecutor::execute_algorithm(const AlgorithmInput& input) {
    AlgorithmOutput output;
    output.success = false;
    output.execution_time_ms = 0;
    output.memory_used_bytes = 0;

    if (!initialized_) {
        output.error_message = "Executor not initialized";
        return output;
    }

    auto start_time = std::chrono::high_resolution_clock::now();

    // 使用JSON解析器解析参数
    auto params = SimpleJsonParser::parse(input.parameters_json);
    
    // 根据算法名称执行
    if (input.algorithm_name == "add") {
        // 解析参数
        double a = SimpleJsonParser::get_double(input.parameters_json, "a", 0.0);
        double b = SimpleJsonParser::get_double(input.parameters_json, "b", 0.0);
        
        double result = a + b;
        
        // 生成JSON结果
        std::map<std::string, double> result_data;
        result_data["result"] = result;
        output.result_json = SimpleJsonParser::to_json_object(result_data);
        output.success = true;
    } else if (input.algorithm_name == "multiply") {
        // 解析参数
        double a = SimpleJsonParser::get_double(input.parameters_json, "a", 1.0);
        double b = SimpleJsonParser::get_double(input.parameters_json, "b", 1.0);
        
        double result = a * b;
        
        // 生成JSON结果
        std::map<std::string, double> result_data;
        result_data["result"] = result;
        output.result_json = SimpleJsonParser::to_json_object(result_data);
        output.success = true;
    } else {
        // 尝试调用cpp_plugins中的插件
        // 注意：这里需要根据实际的插件管理器接口进行调整
        // 当前实现提供框架，实际使用时需要集成cpp_plugins
        
        // 检查是否是已知的插件
        std::vector<std::string> available_plugins = get_available_plugins();
        bool is_plugin = false;
        for (const auto& plugin : available_plugins) {
            if (plugin == input.algorithm_name) {
                is_plugin = true;
                break;
            }
        }
        
        if (is_plugin) {
            // 插件执行逻辑（占位实现）
            // 实际应该调用插件管理器的执行方法
            std::map<std::string, std::string> result_data;
            result_data["message"] = "Plugin executed: " + input.algorithm_name;
            result_data["status"] = "success";
            output.result_json = SimpleJsonParser::to_json(result_data);
            output.success = true;
        } else {
            output.error_message = "Algorithm not found: " + input.algorithm_name;
        }
    }

    // 计算执行时间
    auto end_time = std::chrono::high_resolution_clock::now();
    auto duration = std::chrono::duration_cast<std::chrono::milliseconds>(end_time - start_time);
    output.execution_time_ms = duration.count();

    return output;
}

std::vector<std::string> CppAlgorithmExecutor::get_available_plugins() const {
    // 从cpp_plugins获取可用插件列表
    // 注意：这里需要根据实际的插件管理器接口进行调整
    // 当前实现提供基础插件列表，实际使用时需要集成cpp_plugins的PluginManager
    
    std::vector<std::string> plugins;
    
    // 内置算法
    plugins.push_back("add");
    plugins.push_back("multiply");
    
    // 已知的cpp_plugins插件
    // 实际应该从PluginManager获取
    plugins.push_back("vibrate31");
    plugins.push_back("current_feature_extractor");
    plugins.push_back("temperature_feature_extractor");
    plugins.push_back("audio_feature_extractor");
    plugins.push_back("motor97");
    plugins.push_back("universal_classify1");
    plugins.push_back("comp_realtime_health34");
    plugins.push_back("error18");
    plugins.push_back("score_alarm5");
    plugins.push_back("status_alarm4");
    
    return plugins;
}

std::string CppAlgorithmExecutor::get_plugin_info(const std::string& plugin_name) const {
    // 从cpp_plugins获取插件信息
    // 注意：这里需要根据实际的插件管理器接口进行调整
    // 当前实现提供基础信息，实际使用时需要从PluginManager获取详细信息
    
    std::map<std::string, std::string> info;
    info["name"] = plugin_name;
    info["version"] = "1.0.0";
    info["type"] = "algorithm";
    
    // 根据插件名称设置特定信息
    if (plugin_name == "vibrate31") {
        info["description"] = "Vibration feature extraction plugin";
        info["input_type"] = "vibration_data";
        info["output_type"] = "vibration_features";
    } else if (plugin_name.find("feature_extractor") != std::string::npos) {
        info["description"] = "Feature extraction plugin";
        info["input_type"] = "time_series_data";
        info["output_type"] = "features";
    } else if (plugin_name.find("classify") != std::string::npos) {
        info["description"] = "Classification plugin";
        info["input_type"] = "features";
        info["output_type"] = "classification_result";
    } else if (plugin_name.find("alarm") != std::string::npos) {
        info["description"] = "Alarm plugin";
        info["input_type"] = "evaluation_result";
        info["output_type"] = "alarm_event";
    }
    
    return SimpleJsonParser::to_json(info);
}

// 简单数学函数实现
AlgorithmOutput simple_math_add(double a, double b) {
    AlgorithmOutput output;
    double result = a + b;
    
    // 简单的JSON生成（占位实现）
    output.result_json = "{\"result\":" + std::to_string(result) + "}";
    output.success = true;
    output.execution_time_ms = 0;
    output.memory_used_bytes = 0;
    
    return output;
}

AlgorithmOutput simple_math_multiply(double a, double b) {
    AlgorithmOutput output;
    double result = a * b;
    
    // 简单的JSON生成（占位实现）
    output.result_json = "{\"result\":" + std::to_string(result) + "}";
    output.success = true;
    output.execution_time_ms = 0;
    output.memory_used_bytes = 0;
    
    return output;
}

