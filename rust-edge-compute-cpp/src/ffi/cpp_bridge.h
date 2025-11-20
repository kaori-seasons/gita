#pragma once

#include <string>
#include <vector>
#include <memory>

// 算法输入结构
struct AlgorithmInput {
    std::string algorithm_name;
    std::string parameters_json;
    std::string device_id;
    uint64_t timestamp_ms;
};

// 算法输出结构
struct AlgorithmOutput {
    bool success;
    std::string result_json;
    std::string error_message;
    uint64_t execution_time_ms;
    uint64_t memory_used_bytes;
};

// C++算法执行器类（简化版本，实际应该从cpp_plugins集成）
class CppAlgorithmExecutor {
public:
    CppAlgorithmExecutor();
    ~CppAlgorithmExecutor();

    // 初始化插件管理器
    bool initialize();

    // 执行通用算法
    AlgorithmOutput execute_algorithm(const AlgorithmInput& input);

    // 获取插件信息
    std::vector<std::string> get_available_plugins() const;
    std::string get_plugin_info(const std::string& plugin_name) const;

private:
    bool initialized_;
};

// 简单数学函数（向后兼容）
AlgorithmOutput simple_math_add(double a, double b);
AlgorithmOutput simple_math_multiply(double a, double b);

