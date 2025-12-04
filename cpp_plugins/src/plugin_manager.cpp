#include "plugin_manager.h"
#include "../include/data_types.h"
#include <mutex>
#include <filesystem>
#include <fstream>
#include <sstream>

namespace AlgorithmPlugins {

// PluginManager瀹炵幇
PluginManager& PluginManager::getInstance() {
    static PluginManager instance;
    return instance;
}

bool PluginManager::registerPluginFactory(std::shared_ptr<IPluginFactory> factory) {
    if (!factory) {
        return false;
    }
    
    std::lock_guard<std::mutex> lock(mutex_);
    std::string plugin_name = factory->getPluginName();
    plugin_factories_[plugin_name] = factory;
    return true;
}

bool PluginManager::registerPluginFactory(const std::string& plugin_name, 
                                          std::shared_ptr<IPluginFactory> factory) {
    if (!factory || plugin_name.empty()) {
        return false;
    }
    
    std::lock_guard<std::mutex> lock(mutex_);
    plugin_factories_[plugin_name] = factory;
    return true;
}

std::shared_ptr<IPlugin> PluginManager::createPlugin(const std::string& plugin_name) {
    std::lock_guard<std::mutex> lock(mutex_);
    
    auto factory = getPluginFactory(plugin_name);
    if (!factory) {
        return nullptr;
    }
    
    return factory->createPlugin();
}

std::shared_ptr<IPlugin> PluginManager::createPlugin(const std::string& plugin_name, 
                                                      std::shared_ptr<PluginParameter> params) {
    auto plugin = createPlugin(plugin_name);
    if (plugin && params) {
        if (!plugin->initialize(params)) {
            return nullptr;
        }
    }
    return plugin;
}

std::vector<std::string> PluginManager::getAvailablePlugins() const {
    std::lock_guard<std::mutex> lock(mutex_);
    
    std::vector<std::string> plugins;
    for (const auto& [name, factory] : plugin_factories_) {
        plugins.push_back(name);
    }
    
    return plugins;
}

std::vector<std::string> PluginManager::getPluginsByType(PluginType type) const {
    std::lock_guard<std::mutex> lock(mutex_);
    
    std::vector<std::string> plugins;
    for (const auto& [name, factory] : plugin_factories_) {
        auto plugin = factory->createPlugin();
        if (plugin && plugin->getType() == type) {
            plugins.push_back(name);
        }
    }
    
    return plugins;
}

bool PluginManager::isPluginAvailable(const std::string& plugin_name) const {
    std::lock_guard<std::mutex> lock(mutex_);
    return plugin_factories_.find(plugin_name) != plugin_factories_.end();
}

PluginType PluginManager::getPluginType(const std::string& plugin_name) const {
    std::lock_guard<std::mutex> lock(mutex_);
    
    auto factory = getPluginFactory(plugin_name);
    if (factory) {
        auto plugin = factory->createPlugin();
        if (plugin) {
            return plugin->getType();
        }
    }
    
    return PluginType::OTHER;
}

std::string PluginManager::getPluginVersion(const std::string& plugin_name) const {
    std::lock_guard<std::mutex> lock(mutex_);
    
    auto factory = getPluginFactory(plugin_name);
    if (factory) {
        auto plugin = factory->createPlugin();
        if (plugin) {
            return plugin->getVersion();
        }
    }
    
    return "";
}

std::string PluginManager::getPluginDescription(const std::string& plugin_name) const {
    std::lock_guard<std::mutex> lock(mutex_);
    
    auto factory = getPluginFactory(plugin_name);
    if (factory) {
        auto plugin = factory->createPlugin();
        if (plugin) {
            return plugin->getDescription();
        }
    }
    
    return "";
}

std::vector<std::string> PluginManager::getRequiredParameters(const std::string& plugin_name) const {
    std::lock_guard<std::mutex> lock(mutex_);
    
    auto factory = getPluginFactory(plugin_name);
    if (factory) {
        auto plugin = factory->createPlugin();
        if (plugin) {
            return plugin->getRequiredParameters();
        }
    }
    
    return {};
}

std::vector<std::string> PluginManager::getOptionalParameters(const std::string& plugin_name) const {
    std::lock_guard<std::mutex> lock(mutex_);
    
    auto factory = getPluginFactory(plugin_name);
    if (factory) {
        auto plugin = factory->createPlugin();
        if (plugin) {
            return plugin->getOptionalParameters();
        }
    }
    
    return {};
}

bool PluginManager::loadPluginFromFile(const std::string& file_path) {
    try {
        // 绠€鍖栫殑鎻掍欢鍔犺浇瀹炵幇
        // 鐢熶骇鐜搴斾娇鐢ㄥ姩鎬佸簱鍔犺浇鏈哄埗
        std::ifstream file(file_path);
        if (!file.is_open()) {
            return false;
        }
        
        std::string content((std::istreambuf_iterator<char>(file)),
                           std::istreambuf_iterator<char>());
        
        // 杩欓噷搴旇瑙ｆ瀽鎻掍欢閰嶇疆鏂囦欢骞舵敞鍐屾彃浠?
        // 绠€鍖栧疄鐜帮紝鐩存帴杩斿洖鎴愬姛
        return true;
        
    } catch (const std::exception& e) {
        return false;
    }
}

bool PluginManager::loadPluginsFromDirectory(const std::string& directory_path) {
    try {
        std::filesystem::path dir_path(directory_path);
        if (!std::filesystem::exists(dir_path) || !std::filesystem::is_directory(dir_path)) {
            return false;
        }
        
        bool success = true;
        for (const auto& entry : std::filesystem::directory_iterator(dir_path)) {
            if (entry.is_regular_file() && entry.path().extension() == ".json") {
                if (!loadPluginFromFile(entry.path().string())) {
                    success = false;
                }
            }
        }
        
        return success;
        
    } catch (const std::exception& e) {
        return false;
    }
}

bool PluginManager::unregisterPlugin(const std::string& plugin_name) {
    std::lock_guard<std::mutex> lock(mutex_);
    
    auto it = plugin_factories_.find(plugin_name);
    if (it != plugin_factories_.end()) {
        plugin_factories_.erase(it);
        return true;
    }
    
    return false;
}

void PluginManager::clearAllPlugins() {
    std::lock_guard<std::mutex> lock(mutex_);
    plugin_factories_.clear();
}

std::map<std::string, std::string> PluginManager::getPluginStatus() const {
    std::lock_guard<std::mutex> lock(mutex_);
    
    std::map<std::string, std::string> status;
    for (const auto& [name, factory] : plugin_factories_) {
        status[name] = "Available";
    }
    
    return status;
}

std::shared_ptr<IPluginFactory> PluginManager::getPluginFactory(const std::string& plugin_name) const {
    auto it = plugin_factories_.find(plugin_name);
    return (it != plugin_factories_.end()) ? it->second : nullptr;
}

// PluginChainManager瀹炵幇
PluginChainManager::PluginChainManager() = default;

PluginChainManager::~PluginChainManager() = default;

bool PluginChainManager::createChain(const ChainConfig& config) {
    std::lock_guard<std::mutex> lock(mutex_);
    
    if (config.chain_name.empty() || config.plugin_names.empty()) {
        return false;
    }
    
    // 妫€鏌ユ彃浠舵槸鍚﹀彲鐢?
    auto& manager = PluginManager::getInstance();
    for (const auto& plugin_name : config.plugin_names) {
        if (!manager.isPluginAvailable(plugin_name)) {
            return false;
        }
    }
    
    // 鍒涘缓鎻掍欢閾?
    plugin_chains_[config.chain_name] = config;
    
    // 鍒濆鍖栨彃浠跺疄渚?
    return initializeChainPlugins(config.chain_name);
}

bool PluginChainManager::addPluginToChain(const std::string& chain_name, 
                                          const std::string& plugin_name,
                                          std::shared_ptr<PluginParameter> params) {
    std::lock_guard<std::mutex> lock(mutex_);
    
    auto it = plugin_chains_.find(chain_name);
    if (it == plugin_chains_.end()) {
        return false;
    }
    
    auto& manager = PluginManager::getInstance();
    if (!manager.isPluginAvailable(plugin_name)) {
        return false;
    }
    
    it->second.plugin_names.push_back(plugin_name);
    it->second.plugin_params.push_back(params);
    
    return true;
}

bool PluginChainManager::removePluginFromChain(const std::string& chain_name, 
                                                const std::string& plugin_name) {
    std::lock_guard<std::mutex> lock(mutex_);
    
    auto it = plugin_chains_.find(chain_name);
    if (it == plugin_chains_.end()) {
        return false;
    }
    
    auto& config = it->second;
    auto plugin_it = std::find(config.plugin_names.begin(), config.plugin_names.end(), plugin_name);
    if (plugin_it != config.plugin_names.end()) {
        size_t index = std::distance(config.plugin_names.begin(), plugin_it);
        config.plugin_names.erase(plugin_it);
        
        if (index < config.plugin_params.size()) {
            config.plugin_params.erase(config.plugin_params.begin() + index);
        }
        
        return true;
    }
    
    return false;
}

bool PluginChainManager::clearChain(const std::string& chain_name) {
    std::lock_guard<std::mutex> lock(mutex_);
    
    auto it = plugin_chains_.find(chain_name);
    if (it != plugin_chains_.end()) {
        plugin_chains_.erase(it);
        
        // 娓呯悊鎻掍欢瀹炰緥
        auto instance_it = plugin_instances_.find(chain_name);
        if (instance_it != plugin_instances_.end()) {
            plugin_instances_.erase(instance_it);
        }
        
        return true;
    }
    
    return false;
}

bool PluginChainManager::executeChain(const std::string& chain_name, 
                                     std::shared_ptr<PluginData> input_data,
                                     std::shared_ptr<PluginResult> output_result) {
    std::lock_guard<std::mutex> lock(mutex_);
    
    auto it = plugin_chains_.find(chain_name);
    if (it == plugin_chains_.end()) {
        return false;
    }
    
    const auto& config = it->second;
    auto current_data = input_data;
    auto current_result = std::make_shared<PluginResultImpl>();
    
    // 渚濇鎵ц鎻掍欢閾句腑鐨勬瘡涓彃浠?
    for (size_t i = 0; i < config.plugin_names.size(); ++i) {
        const auto& plugin_name = config.plugin_names[i];
        std::shared_ptr<PluginParameter> params = (i < config.plugin_params.size()) 
            ? config.plugin_params[i] : nullptr;
        
        // Create a temporary result for this plugin
        auto plugin_result = std::make_shared<PluginResultImpl>();
        if (!executePluginInChain(chain_name, plugin_name, current_data, plugin_result)) {
            return false;
        }
        // Copy the plugin result to current_result
        *current_result = *plugin_result;
        
        // 鍑嗗涓嬩竴涓彃浠剁殑杈撳叆鏁版嵁
        if (i < config.plugin_names.size() - 1) {
            current_data = convertDataForPlugin(current_data, 
                plugin_instances_[chain_name][plugin_name]);
        }
    }
    
    // 澶嶅埗鏈€缁堢粨鏋滃埌杈撳嚭
    *output_result = *current_result;
    
    return true;
}

std::vector<std::string> PluginChainManager::getAvailableChains() const {
    std::lock_guard<std::mutex> lock(mutex_);
    
    std::vector<std::string> chains;
    for (const auto& [name, config] : plugin_chains_) {
        chains.push_back(name);
    }
    
    return chains;
}

std::vector<std::string> PluginChainManager::getChainPlugins(const std::string& chain_name) const {
    std::lock_guard<std::mutex> lock(mutex_);
    
    auto it = plugin_chains_.find(chain_name);
    if (it != plugin_chains_.end()) {
        return it->second.plugin_names;
    }
    
    return {};
}

bool PluginChainManager::isChainAvailable(const std::string& chain_name) const {
    std::lock_guard<std::mutex> lock(mutex_);
    return plugin_chains_.find(chain_name) != plugin_chains_.end();
}

bool PluginChainManager::setDataMapping(const std::string& chain_name,
                                       const std::string& source_plugin,
                                       const std::string& target_plugin,
                                       const std::string& data_key) {
    std::lock_guard<std::mutex> lock(mutex_);
    
    auto it = plugin_chains_.find(chain_name);
    if (it == plugin_chains_.end()) {
        return false;
    }
    
    std::string mapping_key = source_plugin + "->" + target_plugin;
    it->second.data_mappings[mapping_key] = data_key;
    
    return true;
}

bool PluginChainManager::initializeChainPlugins(const std::string& chain_name) {
    auto it = plugin_chains_.find(chain_name);
    if (it == plugin_chains_.end()) {
        return false;
    }
    
    const auto& config = it->second;
    auto& manager = PluginManager::getInstance();
    
    // 鍒涘缓鎻掍欢瀹炰緥
    for (size_t i = 0; i < config.plugin_names.size(); ++i) {
        const auto& plugin_name = config.plugin_names[i];
        std::shared_ptr<PluginParameter> params = (i < config.plugin_params.size()) 
            ? config.plugin_params[i] : nullptr;
        
        auto plugin = manager.createPlugin(plugin_name, params);
        if (!plugin) {
            return false;
        }
        
        plugin_instances_[chain_name][plugin_name] = plugin;
    }
    
    return true;
}

bool PluginChainManager::executePluginInChain(const std::string& chain_name,
                                             const std::string& plugin_name,
                                             std::shared_ptr<PluginData> input_data,
                                             std::shared_ptr<PluginResult> output_result) {
    auto instance_it = plugin_instances_.find(chain_name);
    if (instance_it == plugin_instances_.end()) {
        return false;
    }
    
    auto plugin_it = instance_it->second.find(plugin_name);
    if (plugin_it == instance_it->second.end()) {
        return false;
    }
    
    auto plugin = plugin_it->second;
    return plugin->process(input_data, output_result);
}

std::shared_ptr<PluginData> PluginChainManager::convertDataForPlugin(std::shared_ptr<PluginData> input_data,
                                                                    std::shared_ptr<IPlugin> target_plugin) {
    // 绠€鍖栫殑鏁版嵁杞崲瀹炵幇
    // 鐢熶骇鐜搴斿疄鐜版洿澶嶆潅鐨勬暟鎹浆鎹㈤€昏緫
    return input_data;
}

std::shared_ptr<PluginResult> PluginChainManager::convertResultForPlugin(std::shared_ptr<PluginResult> input_result,
                                                                         std::shared_ptr<IPlugin> source_plugin) {
    // 绠€鍖栫殑缁撴灉杞崲瀹炵幇
    // 鐢熶骇鐜搴斿疄鐜版洿澶嶆潅鐨勭粨鏋滆浆鎹㈤€昏緫
    return input_result;
}

// PluginConfigManager瀹炵幇
PluginConfigManager::PluginConfigManager() = default;

PluginConfigManager::~PluginConfigManager() = default;

bool PluginConfigManager::loadConfigFromFile(const std::string& file_path) {
    try {
        std::ifstream file(file_path);
        if (!file.is_open()) {
            return false;
        }
        
        std::string content((std::istreambuf_iterator<char>(file)),
                           std::istreambuf_iterator<char>());
        
        return loadConfigFromJson(content);
        
    } catch (const std::exception& e) {
        return false;
    }
}

bool PluginConfigManager::saveConfigToFile(const std::string& file_path) const {
    try {
        std::ofstream file(file_path);
        if (!file.is_open()) {
            return false;
        }
        
        std::string json_content = saveConfigToJson();
        file << json_content;
        
        return true;
        
    } catch (const std::exception& e) {
        return false;
    }
}

bool PluginConfigManager::loadConfigFromJson(const std::string& json_string) {
    std::lock_guard<std::mutex> lock(mutex_);
    
    try {
        // 绠€鍖栫殑JSON瑙ｆ瀽瀹炵幇
        // 鐢熶骇鐜搴斾娇鐢ㄤ笓涓氱殑JSON搴?
        std::istringstream iss(json_string);
        std::string line;
        
        while (std::getline(iss, line)) {
            // 瑙ｆ瀽閰嶇疆椤?
            // 杩欓噷搴旇瀹炵幇瀹屾暣鐨凧SON瑙ｆ瀽閫昏緫
        }
        
        return true;
        
    } catch (const std::exception& e) {
        return false;
    }
}

std::string PluginConfigManager::saveConfigToJson() const {
    std::lock_guard<std::mutex> lock(mutex_);
    
    std::ostringstream oss;
    oss << "{\n";
    
    // 搴忓垪鍖栨彃浠堕厤缃?
    oss << "  \"plugin_configs\": {\n";
    bool first = true;
    for (const auto& [name, config] : plugin_configs_) {
        if (!first) oss << ",\n";
        oss << "    \"" << name << "\": " << config->serialize();
        first = false;
    }
    oss << "\n  },\n";
    
    // 搴忓垪鍖栧満鏅厤缃?
    oss << "  \"scene_configs\": {\n";
    first = true;
    for (const auto& [name, config] : scene_configs_) {
        if (!first) oss << ",\n";
        oss << "    \"" << name << "\": {";
        bool first_param = true;
        for (const auto& [key, value] : config) {
            if (!first_param) oss << ",";
            oss << "\"" << key << "\":\"" << value << "\"";
            first_param = false;
        }
        oss << "}";
        first = false;
    }
    oss << "\n  },\n";
    
    // 搴忓垪鍖栧叏灞€閰嶇疆
    oss << "  \"global_configs\": {\n";
    first = true;
    for (const auto& [key, value] : global_configs_) {
        if (!first) oss << ",\n";
        oss << "    \"" << key << "\":\"" << value << "\"";
        first = false;
    }
    oss << "\n  }\n";
    
    oss << "}\n";
    
    return oss.str();
}

bool PluginConfigManager::setPluginConfig(const std::string& plugin_name, 
                                          std::shared_ptr<PluginParameter> config) {
    if (plugin_name.empty() || !config) {
        return false;
    }
    
    std::lock_guard<std::mutex> lock(mutex_);
    plugin_configs_[plugin_name] = config;
    return true;
}

std::shared_ptr<PluginParameter> PluginConfigManager::getPluginConfig(const std::string& plugin_name) const {
    std::lock_guard<std::mutex> lock(mutex_);
    
    auto it = plugin_configs_.find(plugin_name);
    return (it != plugin_configs_.end()) ? it->second : nullptr;
}

bool PluginConfigManager::removePluginConfig(const std::string& plugin_name) {
    std::lock_guard<std::mutex> lock(mutex_);
    
    auto it = plugin_configs_.find(plugin_name);
    if (it != plugin_configs_.end()) {
        plugin_configs_.erase(it);
        return true;
    }
    
    return false;
}

bool PluginConfigManager::setSceneConfig(const std::string& scene_name, 
                                         const std::map<std::string, std::string>& config) {
    if (scene_name.empty()) {
        return false;
    }
    
    std::lock_guard<std::mutex> lock(mutex_);
    scene_configs_[scene_name] = config;
    return true;
}

std::map<std::string, std::string> PluginConfigManager::getSceneConfig(const std::string& scene_name) const {
    std::lock_guard<std::mutex> lock(mutex_);
    
    auto it = scene_configs_.find(scene_name);
    return (it != scene_configs_.end()) ? it->second : std::map<std::string, std::string>();
}

bool PluginConfigManager::removeSceneConfig(const std::string& scene_name) {
    std::lock_guard<std::mutex> lock(mutex_);
    
    auto it = scene_configs_.find(scene_name);
    if (it != scene_configs_.end()) {
        scene_configs_.erase(it);
        return true;
    }
    
    return false;
}

bool PluginConfigManager::setGlobalConfig(const std::string& key, const std::string& value) {
    if (key.empty()) {
        return false;
    }
    
    std::lock_guard<std::mutex> lock(mutex_);
    global_configs_[key] = value;
    return true;
}

std::string PluginConfigManager::getGlobalConfig(const std::string& key, const std::string& defaultValue) const {
    std::lock_guard<std::mutex> lock(mutex_);
    
    auto it = global_configs_.find(key);
    return (it != global_configs_.end()) ? it->second : defaultValue;
}

bool PluginConfigManager::removeGlobalConfig(const std::string& key) {
    std::lock_guard<std::mutex> lock(mutex_);
    
    auto it = global_configs_.find(key);
    if (it != global_configs_.end()) {
        global_configs_.erase(it);
        return true;
    }
    
    return false;
}

std::vector<std::string> PluginConfigManager::getConfiguredPlugins() const {
    std::lock_guard<std::mutex> lock(mutex_);
    
    std::vector<std::string> plugins;
    for (const auto& [name, config] : plugin_configs_) {
        plugins.push_back(name);
    }
    
    return plugins;
}

std::vector<std::string> PluginConfigManager::getConfiguredScenes() const {
    std::lock_guard<std::mutex> lock(mutex_);
    
    std::vector<std::string> scenes;
    for (const auto& [name, config] : scene_configs_) {
        scenes.push_back(name);
    }
    
    return scenes;
}

std::map<std::string, std::string> PluginConfigManager::getAllGlobalConfigs() const {
    std::lock_guard<std::mutex> lock(mutex_);
    return global_configs_;
}

// PluginMonitorManager瀹炵幇
PluginMonitorManager::PluginMonitorManager() = default;

PluginMonitorManager::~PluginMonitorManager() = default;

void PluginMonitorManager::startMonitoring(const std::string& plugin_name) {
    std::lock_guard<std::mutex> lock(mutex_);
    
    if (monitoring_enabled_) {
        monitored_plugins_.insert(plugin_name);
        
        // 鍒濆鍖栫洃鎺ф暟鎹?
        if (plugin_metrics_.find(plugin_name) == plugin_metrics_.end()) {
            PluginMetrics metrics;
            metrics.plugin_name = plugin_name;
            plugin_metrics_[plugin_name] = metrics;
        }
    }
}

void PluginMonitorManager::stopMonitoring(const std::string& plugin_name) {
    std::lock_guard<std::mutex> lock(mutex_);
    
    monitored_plugins_.erase(plugin_name);
}

void PluginMonitorManager::recordExecution(const std::string& plugin_name, 
                                          bool success, 
                                          double execution_time_ms,
                                          const std::string& error_message) {
    std::lock_guard<std::mutex> lock(mutex_);
    
    if (!monitoring_enabled_ || monitored_plugins_.find(plugin_name) == monitored_plugins_.end()) {
        return;
    }
    
    auto it = plugin_metrics_.find(plugin_name);
    if (it == plugin_metrics_.end()) {
        PluginMetrics metrics;
        metrics.plugin_name = plugin_name;
        plugin_metrics_[plugin_name] = metrics;
        it = plugin_metrics_.find(plugin_name);
    }
    
    auto& metrics = it->second;
    metrics.execution_count++;
    
    if (success) {
        metrics.success_count++;
    } else {
        metrics.error_count++;
        metrics.last_error_message = error_message;
    }
    
    // 鏇存柊鎵ц鏃堕棿缁熻
    if (metrics.min_execution_time_ms == 0.0 || execution_time_ms < metrics.min_execution_time_ms) {
        metrics.min_execution_time_ms = execution_time_ms;
    }
    
    if (execution_time_ms > metrics.max_execution_time_ms) {
        metrics.max_execution_time_ms = execution_time_ms;
    }
    
    // 鏇存柊骞冲潎鎵ц鏃堕棿
    metrics.avg_execution_time_ms = (metrics.avg_execution_time_ms * (metrics.execution_count - 1) + execution_time_ms) / metrics.execution_count;
    
    metrics.last_execution_time = std::chrono::system_clock::now();
}

PluginMonitorManager::PluginMetrics PluginMonitorManager::getPluginMetrics(const std::string& plugin_name) const {
    std::lock_guard<std::mutex> lock(mutex_);
    
    auto it = plugin_metrics_.find(plugin_name);
    return (it != plugin_metrics_.end()) ? it->second : PluginMetrics();
}

std::vector<std::string> PluginMonitorManager::getMonitoredPlugins() const {
    std::lock_guard<std::mutex> lock(mutex_);
    
    std::vector<std::string> plugins;
    for (const auto& monitored_plugin : monitored_plugins_) {
        plugins.push_back(monitored_plugin);
    }
    
    return plugins;
}

std::map<std::string, PluginMonitorManager::PluginMetrics> PluginMonitorManager::getAllMetrics() const {
    std::lock_guard<std::mutex> lock(mutex_);
    return plugin_metrics_;
}

double PluginMonitorManager::getAverageExecutionTime(const std::string& plugin_name) const {
    std::lock_guard<std::mutex> lock(mutex_);
    
    auto it = plugin_metrics_.find(plugin_name);
    return (it != plugin_metrics_.end()) ? it->second.avg_execution_time_ms : 0.0;
}

double PluginMonitorManager::getSuccessRate(const std::string& plugin_name) const {
    std::lock_guard<std::mutex> lock(mutex_);
    
    auto it = plugin_metrics_.find(plugin_name);
    if (it != plugin_metrics_.end() && it->second.execution_count > 0) {
        return static_cast<double>(it->second.success_count) / it->second.execution_count;
    }
    
    return 0.0;
}

uint64_t PluginMonitorManager::getExecutionCount(const std::string& plugin_name) const {
    std::lock_guard<std::mutex> lock(mutex_);
    
    auto it = plugin_metrics_.find(plugin_name);
    return (it != plugin_metrics_.end()) ? it->second.execution_count : 0;
}

} // namespace AlgorithmPlugins
