#pragma once

#include <memory>
#include <string>
#include <vector>
#include <map>
#include <chrono>
#include <functional>

namespace AlgorithmPlugins {

// 前向声明
class PluginData;
class PluginResult;
class PluginParameter;

/**
 * @brief 插件数据类型枚举
 */
enum class DataType {
    REAL_TIME,      // 实时数据（秒级）
    BATCH_DATA,     // 批次数据（如振动数据）
    FEATURE_DATA,   // 特征数据
    STATUS_DATA     // 状态数据
};

/**
 * @brief 插件类型枚举
 */
enum class PluginType {
    FEATURE,        // 特征提取插件
    DECISION,       // 状态识别插件
    EVALUATION,     // 健康评估插件
    EVENT,          // 事件处理插件
    OTHER,          // 综合算法插件
    SUMMARY         // 汇总算法插件
};

/**
 * @brief 插件数据基类
 */
class PluginData {
public:
    virtual ~PluginData() = default;
    
    // 获取数据类型
    virtual DataType getType() const = 0;
    
    // 获取时间戳
    virtual std::chrono::system_clock::time_point getTimestamp() const = 0;
    
    // 获取设备ID
    virtual std::string getDeviceId() const = 0;
    
    // 序列化/反序列化
    virtual std::string serialize() const = 0;
    virtual bool deserialize(const std::string& data) = 0;
};

/**
 * @brief 插件结果基类
 */
class PluginResult {
public:
    virtual ~PluginResult() = default;
    
    // 设置结果数据
    virtual void setData(const std::string& key, const std::string& value) = 0;
    virtual void setData(const std::string& key, double value) = 0;
    virtual void setData(const std::string& key, int value) = 0;
    
    // 获取结果数据
    virtual std::string getStringData(const std::string& key) const = 0;
    virtual double getDoubleData(const std::string& key) const = 0;
    virtual int getIntData(const std::string& key) const = 0;
    
    // 检查是否存在某个键
    virtual bool hasData(const std::string& key) const = 0;
    
    // 序列化/反序列化
    virtual std::string serialize() const = 0;
    virtual bool deserialize(const std::string& data) = 0;
};

/**
 * @brief 插件参数基类
 */
class PluginParameter {
public:
    virtual ~PluginParameter() = default;
    
    // 获取参数
    virtual std::string getString(const std::string& key, const std::string& defaultValue = "") const = 0;
    virtual double getDouble(const std::string& key, double defaultValue = 0.0) const = 0;
    virtual int getInt(const std::string& key, int defaultValue = 0) const = 0;
    virtual bool getBool(const std::string& key, bool defaultValue = false) const = 0;
    virtual std::vector<double> getDoubleArray(const std::string& key) const = 0;
    virtual std::vector<int> getIntArray(const std::string& key) const = 0;
    
    // 设置参数
    virtual void setString(const std::string& key, const std::string& value) = 0;
    virtual void setDouble(const std::string& key, double value) = 0;
    virtual void setInt(const std::string& key, int value) = 0;
    virtual void setBool(const std::string& key, bool value) = 0;
    virtual void setDoubleArray(const std::string& key, const std::vector<double>& value) = 0;
    virtual void setIntArray(const std::string& key, const std::vector<int>& value) = 0;
    
    // 序列化/反序列化
    virtual std::string serialize() const = 0;
    virtual bool deserialize(const std::string& data) = 0;
};

/**
 * @brief 插件基类接口
 */
class IPlugin {
public:
    virtual ~IPlugin() = default;
    
    // 插件基本信息
    virtual std::string getName() const = 0;
    virtual std::string getVersion() const = 0;
    virtual std::string getDescription() const = 0;
    virtual PluginType getType() const = 0;
    
    // 插件生命周期管理
    virtual bool initialize(std::shared_ptr<PluginParameter> params) = 0;
    virtual bool process(std::shared_ptr<PluginData> input, std::shared_ptr<PluginResult> output) = 0;
    virtual void cleanup() = 0;
    
    // 插件状态查询
    virtual bool isInitialized() const = 0;
    virtual std::string getLastError() const = 0;
    
    // 插件配置
    virtual std::vector<std::string> getRequiredParameters() const = 0;
    virtual std::vector<std::string> getOptionalParameters() const = 0;
};

/**
 * @brief 插件基类实现
 */
class AlgorithmWork : public IPlugin {
public:
    AlgorithmWork();
    virtual ~AlgorithmWork() = default;
    
    // IPlugin接口实现
    bool initialize(std::shared_ptr<PluginParameter> params) override;
    void cleanup() override;
    bool process(std::shared_ptr<PluginData> input, std::shared_ptr<PluginResult> output) override;
    bool isInitialized() const override { return initialized_; }
    std::string getLastError() const override { return last_error_; }
    
protected:
    bool initialized_ = false;
    std::string last_error_;
    std::shared_ptr<PluginParameter> parameters_;
    
    // 设置错误信息
    void setError(const std::string& error);
    
    // 参数验证
    virtual bool validateParameters();
    
    // 算法核心接口
    virtual bool algorithm(std::shared_ptr<PluginData> input, std::shared_ptr<PluginResult> output);
};

/**
 * @brief 插件工厂接口
 */
class IPluginFactory {
public:
    virtual ~IPluginFactory() = default;
    virtual std::shared_ptr<IPlugin> createPlugin() = 0;
    virtual std::string getPluginName() const = 0;
    virtual PluginType getPluginType() const = 0;
};

/**
 * @brief 插件注册宏
 */
#define REGISTER_PLUGIN(PluginClass, PluginName, PluginType) \
    static PluginClass##Factory g_##PluginClass##Factory; \
    extern "C" { \
        __declspec(dllexport) IPluginFactory* createPluginFactory() { \
            return &g_##PluginClass##Factory; \
        } \
    }

} // namespace AlgorithmPlugins
