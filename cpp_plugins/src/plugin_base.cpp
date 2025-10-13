#include "plugin_base.h"
#include <stdexcept>
#include <sstream>

namespace AlgorithmPlugins {

// PluginData基类实现
PluginData::~PluginData() = default;

// PluginResult基类实现
PluginResult::~PluginResult() = default;

// PluginParameter基类实现
PluginParameter::~PluginParameter() = default;

// AlgorithmWork基类实现
AlgorithmWork::AlgorithmWork() {
    // 初始化基础变量
    initialized_ = false;
    last_error_ = "";
}

bool AlgorithmWork::initialize(std::shared_ptr<PluginParameter> params) {
    try {
        parameters_ = params;
        initialized_ = validateParameters();
        
        if (!initialized_) {
            setError("参数验证失败");
            return false;
        }
        
        return true;
    } catch (const std::exception& e) {
        setError("初始化异常: " + std::string(e.what()));
        return false;
    }
}

void AlgorithmWork::cleanup() {
    initialized_ = false;
    parameters_.reset();
}

bool AlgorithmWork::process(std::shared_ptr<PluginData> input, std::shared_ptr<PluginResult> output) {
    if (!initialized_) {
        setError("插件未初始化");
        return false;
    }
    
    if (!input || !output) {
        setError("输入或输出数据为空");
        return false;
    }
    
    try {
        return algorithm(input, output);
    } catch (const std::exception& e) {
        setError("算法执行异常: " + std::string(e.what()));
        return false;
    }
}

void AlgorithmWork::setError(const std::string& error) {
    last_error_ = error;
    // 这里可以添加日志记录
}

bool AlgorithmWork::validateParameters() {
    // 默认实现，子类可以重写
    return true;
}

bool AlgorithmWork::algorithm(std::shared_ptr<PluginData> input, std::shared_ptr<PluginResult> output) {
    // 默认实现，子类必须重写
    setError("算法未实现");
    return false;
}

} // namespace AlgorithmPlugins
