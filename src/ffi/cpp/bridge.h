#pragma once

#include <cstdint>
#include <memory>
#include <vector>
#include <string>

struct AlgorithmInput;
struct AlgorithmOutput;

// 前向声明
class PluginExecutor;

// C++算法执行器接口（三层架构实现）
class CppAlgorithmExecutor {
public:
    CppAlgorithmExecutor();
    ~CppAlgorithmExecutor();

    // 初始化
    bool initialize() const;

    // 执行通用算法
    AlgorithmOutput execute_algorithm(const AlgorithmInput& input) const;

    // 获取可用插件列表
    std::vector<std::string> get_available_plugins() const;

    // 获取插件信息
    std::string get_plugin_info(const std::string& plugin_name) const;

    // 加载插件
    bool load_plugin(const std::string& plugin_name);

    // 卸载插件
    bool unload_plugin(const std::string& plugin_name);

private:
    // 插件执行器（顶层抽象）
    mutable std::unique_ptr<PluginExecutor> plugin_executor_;
};

// 创建C++算法执行器的工厂函数
std::unique_ptr<CppAlgorithmExecutor> new_cpp_executor();
