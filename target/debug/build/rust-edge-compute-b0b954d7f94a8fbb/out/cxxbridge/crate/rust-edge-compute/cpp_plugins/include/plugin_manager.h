#pragma once

#include "plugin_base.h"
#include <memory>
#include <map>
#include <vector>
#include <string>
#include <mutex>

namespace AlgorithmPlugins {

/**
 * @brief 插件管理器类
 * 
 * 负责插件的加载、注册、创建和管理
 */
class PluginManager {
public:
    static PluginManager& getInstance();
    
    // 禁用拷贝构造和赋值
    PluginManager(const PluginManager&) = delete;
    PluginManager& operator=(const PluginManager&) = delete;
    
    // 插件注册
    bool registerPluginFactory(std::shared_ptr<IPluginFactory> factory);
    bool registerPluginFactory(const std::string& plugin_name, 
                              std::shared_ptr<IPluginFactory> factory);
    
    // 插件创建
    std::shared_ptr<IPlugin> createPlugin(const std::string& plugin_name);
    std::shared_ptr<IPlugin> createPlugin(const std::string& plugin_name, 
                                        std::shared_ptr<PluginParameter> params);
    
    // 插件查询
    std::vector<std::string> getAvailablePlugins() const;
    std::vector<std::string> getPluginsByType(PluginType type) const;
    bool isPluginAvailable(const std::string& plugin_name) const;
    PluginType getPluginType(const std::string& plugin_name) const;
    
    // 插件信息查询
    std::string getPluginVersion(const std::string& plugin_name) const;
    std::string getPluginDescription(const std::string& plugin_name) const;
    std::vector<std::string> getRequiredParameters(const std::string& plugin_name) const;
    std::vector<std::string> getOptionalParameters(const std::string& plugin_name) const;
    
    // 插件加载
    bool loadPluginFromFile(const std::string& file_path);
    bool loadPluginsFromDirectory(const std::string& directory_path);
    
    // 插件卸载
    bool unregisterPlugin(const std::string& plugin_name);
    void clearAllPlugins();
    
    // 插件状态查询
    std::map<std::string, std::string> getPluginStatus() const;
    
private:
    PluginManager() = default;
    ~PluginManager() = default;
    
    // 插件工厂映射
    std::map<std::string, std::shared_ptr<IPluginFactory>> plugin_factories_;
    
    // 线程安全
    mutable std::mutex mutex_;
    
    // 内部方法
    std::shared_ptr<IPluginFactory> getPluginFactory(const std::string& plugin_name) const;
};

/**
 * @brief 插件链管理器
 * 
 * 负责管理插件链的执行顺序和数据流
 */
class PluginChainManager {
public:
    PluginChainManager();
    ~PluginChainManager();
    
    // 插件链配置
    struct ChainConfig {
        std::string chain_name;
        std::vector<std::string> plugin_names;
        std::vector<std::shared_ptr<PluginParameter>> plugin_params;
        std::map<std::string, std::string> data_mappings; // 数据映射关系
    };
    
    // 插件链管理
    bool createChain(const ChainConfig& config);
    bool addPluginToChain(const std::string& chain_name, 
                          const std::string& plugin_name,
                          std::shared_ptr<PluginParameter> params);
    bool removePluginFromChain(const std::string& chain_name, 
                               const std::string& plugin_name);
    bool clearChain(const std::string& chain_name);
    
    // 插件链执行
    bool executeChain(const std::string& chain_name, 
                     std::shared_ptr<PluginData> input_data,
                     std::shared_ptr<PluginResult> output_result);
    
    // 插件链查询
    std::vector<std::string> getAvailableChains() const;
    std::vector<std::string> getChainPlugins(const std::string& chain_name) const;
    bool isChainAvailable(const std::string& chain_name) const;
    
    // 数据流管理
    bool setDataMapping(const std::string& chain_name,
                       const std::string& source_plugin,
                       const std::string& target_plugin,
                       const std::string& data_key);
    
private:
    // 插件链映射
    std::map<std::string, ChainConfig> plugin_chains_;
    
    // 插件实例缓存
    std::map<std::string, std::map<std::string, std::shared_ptr<IPlugin>>> plugin_instances_;
    
    // 线程安全
    mutable std::mutex mutex_;
    
    // 内部方法
    bool initializeChainPlugins(const std::string& chain_name);
    bool executePluginInChain(const std::string& chain_name,
                            const std::string& plugin_name,
                            std::shared_ptr<PluginData> input_data,
                            std::shared_ptr<PluginResult> output_result);
    
    // 数据转换
    std::shared_ptr<PluginData> convertDataForPlugin(std::shared_ptr<PluginData> input_data,
                                                    std::shared_ptr<IPlugin> target_plugin);
    std::shared_ptr<PluginResult> convertResultForPlugin(std::shared_ptr<PluginResult> input_result,
                                                        std::shared_ptr<IPlugin> source_plugin);
};

/**
 * @brief 插件配置管理器
 * 
 * 负责管理插件的配置参数和场景参数
 */
class PluginConfigManager {
public:
    PluginConfigManager();
    ~PluginConfigManager();
    
    // 配置文件管理
    bool loadConfigFromFile(const std::string& file_path);
    bool saveConfigToFile(const std::string& file_path) const;
    bool loadConfigFromJson(const std::string& json_string);
    std::string saveConfigToJson() const;
    
    // 插件配置管理
    bool setPluginConfig(const std::string& plugin_name, 
                        std::shared_ptr<PluginParameter> config);
    std::shared_ptr<PluginParameter> getPluginConfig(const std::string& plugin_name) const;
    bool removePluginConfig(const std::string& plugin_name);
    
    // 场景配置管理
    bool setSceneConfig(const std::string& scene_name, 
                       const std::map<std::string, std::string>& config);
    std::map<std::string, std::string> getSceneConfig(const std::string& scene_name) const;
    bool removeSceneConfig(const std::string& scene_name);
    
    // 全局配置管理
    bool setGlobalConfig(const std::string& key, const std::string& value);
    std::string getGlobalConfig(const std::string& key, const std::string& defaultValue = "") const;
    bool removeGlobalConfig(const std::string& key);
    
    // 配置查询
    std::vector<std::string> getConfiguredPlugins() const;
    std::vector<std::string> getConfiguredScenes() const;
    std::map<std::string, std::string> getAllGlobalConfigs() const;
    
private:
    // 配置存储
    std::map<std::string, std::shared_ptr<PluginParameter>> plugin_configs_;
    std::map<std::string, std::map<std::string, std::string>> scene_configs_;
    std::map<std::string, std::string> global_configs_;
    
    // 线程安全
    mutable std::mutex mutex_;
};

/**
 * @brief 插件监控管理器
 * 
 * 负责监控插件的执行状态和性能
 */
class PluginMonitorManager {
public:
    PluginMonitorManager();
    ~PluginMonitorManager();
    
    // 监控数据结构
    struct PluginMetrics {
        std::string plugin_name;
        uint64_t execution_count = 0;
        uint64_t success_count = 0;
        uint64_t error_count = 0;
        double avg_execution_time_ms = 0.0;
        double max_execution_time_ms = 0.0;
        double min_execution_time_ms = 0.0;
        std::chrono::system_clock::time_point last_execution_time;
        std::string last_error_message;
    };
    
    // 监控管理
    void startMonitoring(const std::string& plugin_name);
    void stopMonitoring(const std::string& plugin_name);
    void recordExecution(const std::string& plugin_name, 
                        bool success, 
                        double execution_time_ms,
                        const std::string& error_message = "");
    
    // 监控数据查询
    PluginMetrics getPluginMetrics(const std::string& plugin_name) const;
    std::vector<std::string> getMonitoredPlugins() const;
    std::map<std::string, PluginMetrics> getAllMetrics() const;
    
    // 性能统计
    double getAverageExecutionTime(const std::string& plugin_name) const;
    double getSuccessRate(const std::string& plugin_name) const;
    uint64_t getExecutionCount(const std::string& plugin_name) const;
    
    // 监控配置
    void setMonitoringEnabled(bool enabled) { monitoring_enabled_ = enabled; }
    bool isMonitoringEnabled() const { return monitoring_enabled_; }
    
private:
    // 监控数据
    std::map<std::string, PluginMetrics> plugin_metrics_;
    std::set<std::string> monitored_plugins_;
    
    // 监控状态
    bool monitoring_enabled_ = true;
    
    // 线程安全
    mutable std::mutex mutex_;
};

} // namespace AlgorithmPlugins
