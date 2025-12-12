#pragma once

#include <cstdint>
#include <memory>
#include <vector>
#include <string>

// 前向声明
class PluginExecutor;

// 算法输入结构 - 与Rust侧共享
struct AlgorithmInput {
    std::string algorithm_name;
    std::string parameters_json;
    std::string device_id;
    uint64_t timestamp_ms;
};

// 算法输出结构 - 与Rust侧共享
struct AlgorithmOutput {
    bool success;
    std::string result_json;
    std::string error_message;
    uint64_t execution_time_ms;
    uint64_t memory_used_bytes;
};

// C++ 算法执行器接口
class CppAlgorithmExecutor {
public:
    CppAlgorithmExecutor();
    ~CppAlgorithmExecutor();

    // 初始化
    bool initialize() const;
    
    // 执行算法 - 生产级实现
    AlgorithmOutput execute_algorithm(const AlgorithmInput& input) const;
    
    // 获取可用插件列表
    std::vector<std::string> get_available_plugins() const;
    
    // 获取插件信息
    std::string get_plugin_info(const std::string& plugin_name) const;

private:
    // 插件执行器（顶层抽象）
    mutable PluginExecutor* plugin_executor_;
};

// 创建C++算法执行器的工厂函数
std::unique_ptr<CppAlgorithmExecutor> new_cpp_executor();
