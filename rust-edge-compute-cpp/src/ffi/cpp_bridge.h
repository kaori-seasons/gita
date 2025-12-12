#pragma once

#include <cstdint>
#include <memory>
#include <string>
#include <chrono>

// 前向声明
struct AlgorithmOutput;

/// C++ 算法执行器接口
class CppAlgorithmExecutor {
public:
    CppAlgorithmExecutor();
    ~CppAlgorithmExecutor();

    /// 初始化执行器
    bool initialize() const;

private:
    // 空私有区域

public:
    // execute_algorithm will be provided as a free function wrapper below
};
std::unique_ptr<CppAlgorithmExecutor> new_cpp_executor();

// cpp_plugins 命名空间函数声明
namespace AlgorithmPlugins {
    void registerAllPlugins();
}
