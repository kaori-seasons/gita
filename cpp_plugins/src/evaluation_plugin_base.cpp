#include "evaluation_plugin_base.h"
#include <algorithm>
#include <numeric>
#include <cmath>

namespace AlgorithmPlugins {

// EvaluationPluginBase实现
EvaluationPluginBase::EvaluationPluginBase() = default;

bool EvaluationPluginBase::initialize(std::shared_ptr<PluginParameter> params) {
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

void EvaluationPluginBase::cleanup() {
    initialized_ = false;
    parameters_.reset();
}

bool EvaluationPluginBase::evaluateHealth(std::shared_ptr<PluginData> input, 
                                          std::shared_ptr<PluginResult> output) {
    if (!initialized_) {
        setError("插件未初始化");
        return false;
    }
    
    if (!input || !output) {
        setError("输入或输出数据为空");
        return false;
    }
    
    try {
        // 这里应该调用具体的健康评估实现
        // 由于这是基类，子类需要重写evaluateHealth方法
        setError("健康评估方法未实现");
        return false;
    } catch (const std::exception& e) {
        setError("健康评估异常: " + std::string(e.what()));
        return false;
    }
}

double EvaluationPluginBase::calculateScore(double value, double threshold_low, double threshold_high) {
    if (value <= threshold_low) return 100.0;
    if (value >= threshold_high) return 0.0;
    
    return 100.0 * (threshold_high - value) / (threshold_high - threshold_low);
}

double EvaluationPluginBase::calculateScore(double value, const std::vector<double>& thresholds) {
    if (thresholds.empty()) return 100.0;
    
    for (size_t i = 0; i < thresholds.size(); ++i) {
        if (value <= thresholds[i]) {
            return 100.0 - (i * 100.0 / thresholds.size());
        }
    }
    
    return 0.0;
}

std::map<std::string, double> EvaluationPluginBase::mergeHealthScores(const std::vector<std::map<std::string, double>>& scores) {
    std::map<std::string, double> merged_scores;
    
    if (scores.empty()) return merged_scores;
    
    // 获取所有健康度名称
    std::set<std::string> all_keys;
    for (const auto& score_map : scores) {
        for (const auto& [key, value] : score_map) {
            all_keys.insert(key);
        }
    }
    
    // 计算平均值
    for (const auto& key : all_keys) {
        double sum = 0.0;
        int count = 0;
        
        for (const auto& score_map : scores) {
            auto it = score_map.find(key);
            if (it != score_map.end()) {
                sum += it->second;
                count++;
            }
        }
        
        if (count > 0) {
            merged_scores[key] = sum / count;
        }
    }
    
    return merged_scores;
}

// RealtimeHealthPluginBase实现
RealtimeHealthPluginBase::RealtimeHealthPluginBase() = default;

bool RealtimeHealthPluginBase::evaluateHealth(std::shared_ptr<PluginData> input, 
                                              std::shared_ptr<PluginResult> output) {
    try {
        // 离线检测
        offlineCheck(std::chrono::system_clock::now());
        
        // 状态检查和数据缓存
        if (!statusCheckAndCacheData(input, std::chrono::system_clock::now())) {
            // 使用上次结果
            for (const auto& [key, value] : last_health_scores_) {
                output->setData(key, value);
            }
            return true;
        }
        
        // 各特征的统计量计算与分数评估
        std::map<std::string, double> stat_scores;
        for (const auto& stat : getFeatureStats()) {
            auto scores = calculateFeatureHealth(stat, feature_cache_[stat.analysis_features]);
            stat_scores.insert(scores.begin(), scores.end());
        }
        
        // 组合输出的健康度曲线
        for (const auto& config : getHealthConfigs()) {
            auto health_scores = calculateOverallHealth({config}, stat_scores);
            for (const auto& [key, value] : health_scores) {
                output->setData(key, value);
                last_health_scores_[key] = value;
            }
        }
        
        return true;
        
    } catch (const std::exception& e) {
        setError("实时健康度评估异常: " + std::string(e.what()));
        return false;
    }
}

void RealtimeHealthPluginBase::offlineCheck(std::chrono::system_clock::time_point current_time) {
    if (prev_time_ != std::chrono::system_clock::time_point{}) {
        auto duration = std::chrono::duration_cast<std::chrono::seconds>(current_time - prev_time_);
        if (duration.count() > offline_length_) {
            resetCache(true);
        }
    }
    prev_time_ = current_time;
}

bool RealtimeHealthPluginBase::statusCheckAndCacheData(std::shared_ptr<PluginData> input_data,
                                                      std::chrono::system_clock::time_point current_time) {
    try {
        // 获取状态数据
        int status = 0;
        std::map<std::string, double> features;
        
        if (auto status_data = std::dynamic_pointer_cast<StatusData>(input_data)) {
            status = status_data->getStatus();
        } else if (auto feature_data = std::dynamic_pointer_cast<FeatureData>(input_data)) {
            features = feature_data->getFeatures();
            // 从特征中提取状态
            auto status_it = features.find("status");
            if (status_it != features.end()) {
                status = static_cast<int>(status_it->second);
            }
        } else if (auto realtime_data = std::dynamic_pointer_cast<RealTimeData>(input_data)) {
            features["mean_hf"] = realtime_data->getMeanHF();
            features["mean_lf"] = realtime_data->getMeanLF();
            features["mean"] = realtime_data->getMean();
            features["std"] = realtime_data->getStd();
            features["temperature"] = realtime_data->getTemperature();
            features["speed"] = realtime_data->getSpeed();
        }
        
        // 状态检查
        if (status == 1) { // 运行状态
            run_count_++;
            close_count_ = 0;
            
            // 缓存特征数据
            for (const auto& stat : getFeatureStats()) {
                auto it = features.find(stat.analysis_features);
                if (it != features.end()) {
                    feature_cache_[stat.analysis_features].push_back(it->second);
                    time_cache_[stat.analysis_status].push_back(current_time);
                }
            }
            
            return true;
        } else {
            close_count_++;
            run_count_ = 0;
            
            // 检查是否超过非关注状态持续时长
            if (close_count_ >= close_width_) {
                resetCache(false);
                return false;
            }
            
            return false;
        }
        
    } catch (const std::exception& e) {
        setError("状态检查和数据缓存异常: " + std::string(e.what()));
        return false;
    }
}

void RealtimeHealthPluginBase::resetCache(bool all_cache) {
    if (all_cache) {
        feature_cache_.clear();
        time_cache_.clear();
    }
    
    close_count_ = 0;
    run_count_ = 0;
}

// CompRealtimeHealth34Plugin实现
CompRealtimeHealth34Plugin::CompRealtimeHealth34Plugin() = default;

bool CompRealtimeHealth34Plugin::validateParameters() {
    // 获取必需参数
    auto feature_stats_str = parameters_->getString("feature_stats", "");
    auto healths_str = parameters_->getString("healths", "");
    
    if (feature_stats_str.empty()) {
        setError("feature_stats参数不能为空");
        return false;
    }
    
    if (healths_str.empty()) {
        setError("healths参数不能为空");
        return false;
    }
    
    // 解析参数（简化实现）
    parseFeatureStats(feature_stats_str);
    parseHealthConfigs(healths_str);
    
    // 获取可选参数
    offline_length_ = parameters_->getInt("offline_length", 86400 * 15);
    minimum_quantity_ = parameters_->getInt("minimum_quantity", 30);
    close_width_ = parameters_->getInt("close_width", 1);
    
    // 获取健康度定义和默认分数
    auto health_def_array = parameters_->getStringArray("health_define");
    auto default_score_array = parameters_->getIntArray("default_score");
    
    health_definitions_ = health_def_array;
    default_scores_ = default_score_array;
    
    return true;
}

std::vector<std::string> CompRealtimeHealth34Plugin::getHealthDefinitions() const {
    return health_definitions_;
}

std::vector<int> CompRealtimeHealth34Plugin::getDefaultScores() const {
    return default_scores_;
}

std::vector<RealtimeHealthPluginBase::FeatureStat> CompRealtimeHealth34Plugin::getFeatureStats() const {
    return feature_stats_;
}

std::vector<RealtimeHealthPluginBase::HealthConfig> CompRealtimeHealth34Plugin::getHealthConfigs() const {
    return health_configs_;
}

std::map<std::string, double> CompRealtimeHealth34Plugin::calculateFeatureHealth(const FeatureStat& stat,
                                                                                 const std::vector<double>& data) {
    std::map<std::string, double> scores;
    
    if (data.size() < minimum_quantity_) {
        // 数据量不足，返回默认分数
        scores[stat.result_key] = 100.0;
        return scores;
    }
    
    try {
        // 数据清洗
        std::vector<double> cleaned_data = cleanData(data, stat.clean_formula);
        
        // 平滑处理
        std::vector<double> smoothed_data = smoothData(cleaned_data, stat.move_smooth_param);
        
        // 计算统计量
        for (const auto& method : stat.statistic) {
            double stat_value = calculateStatistic(smoothed_data, method);
            double score = convertToScore(stat_value, stat.thresholds, stat.upper_limit);
            scores[stat.result_key + "_" + method] = score;
        }
        
        // 计算综合分数
        double avg_score = 0.0;
        for (const auto& [key, value] : scores) {
            avg_score += value;
        }
        avg_score /= scores.size();
        scores[stat.result_key] = avg_score;
        
        return scores;
        
    } catch (const std::exception& e) {
        setError("特征健康度计算异常: " + std::string(e.what()));
        scores[stat.result_key] = 100.0; // 默认分数
        return scores;
    }
}

std::map<std::string, double> CompRealtimeHealth34Plugin::calculateOverallHealth(const std::vector<HealthConfig>& configs,
                                                                                 const std::map<std::string, double>& feature_scores) {
    std::map<std::string, double> health_scores;
    
    for (const auto& config : configs) {
        if (config.formula == "weighted_average") {
            double weighted_sum = 0.0;
            double total_weight = 0.0;
            
            for (size_t i = 0; i < config.dependencies.size() && i < config.weights.size(); ++i) {
                auto it = feature_scores.find(config.dependencies[i]);
                if (it != feature_scores.end()) {
                    weighted_sum += it->second * config.weights[i];
                    total_weight += config.weights[i];
                }
            }
            
            if (total_weight > 0) {
                health_scores[config.name] = weighted_sum / total_weight;
            } else {
                health_scores[config.name] = 100.0; // 默认分数
            }
        } else {
            // 其他计算公式
            health_scores[config.name] = 100.0; // 默认分数
        }
    }
    
    return health_scores;
}

double CompRealtimeHealth34Plugin::calculateStatistic(const std::vector<double>& data, const std::string& method) {
    if (data.empty()) return 0.0;
    
    if (method == "mean") {
        return std::accumulate(data.begin(), data.end(), 0.0) / data.size();
    } else if (method == "std") {
        double mean = std::accumulate(data.begin(), data.end(), 0.0) / data.size();
        double sum_sq_diff = 0.0;
        for (double value : data) {
            double diff = value - mean;
            sum_sq_diff += diff * diff;
        }
        return std::sqrt(sum_sq_diff / (data.size() - 1));
    } else if (method == "max") {
        return *std::max_element(data.begin(), data.end());
    } else if (method == "min") {
        return *std::min_element(data.begin(), data.end());
    } else if (method == "median") {
        std::vector<double> sorted_data = data;
        std::sort(sorted_data.begin(), sorted_data.end());
        size_t n = sorted_data.size();
        if (n % 2 == 0) {
            return (sorted_data[n/2 - 1] + sorted_data[n/2]) / 2.0;
        } else {
            return sorted_data[n/2];
        }
    }
    
    return 0.0;
}

std::vector<double> CompRealtimeHealth34Plugin::cleanData(const std::vector<double>& data, 
                                                          const std::map<std::string, std::string>& clean_formula) {
    std::vector<double> cleaned_data = data;
    
    for (const auto& [method, params] : clean_formula) {
        if (method == "remove_edges") {
            // 去头去尾
            if (cleaned_data.size() > 4) {
                cleaned_data.erase(cleaned_data.begin());
                cleaned_data.erase(cleaned_data.end() - 1);
            }
        } else if (method == "percentile_cleaning") {
            // 百分位清洗
            std::vector<double> sorted_data = cleaned_data;
            std::sort(sorted_data.begin(), sorted_data.end());
            
            size_t n = sorted_data.size();
            double lower_percentile = sorted_data[static_cast<size_t>(n * 0.05)];
            double upper_percentile = sorted_data[static_cast<size_t>(n * 0.95)];
            
            for (double& value : cleaned_data) {
                if (value < lower_percentile) value = lower_percentile;
                if (value > upper_percentile) value = upper_percentile;
            }
        }
    }
    
    return cleaned_data;
}

std::vector<double> CompRealtimeHealth34Plugin::smoothData(const std::vector<double>& data,
                                                           const std::map<std::string, std::string>& smooth_param) {
    std::vector<double> smoothed_data = data;
    
    auto win_length_it = smooth_param.find("win_length");
    auto func_it = smooth_param.find("func");
    
    if (win_length_it != smooth_param.end() && func_it != smooth_param.end()) {
        int win_length = std::stoi(win_length_it->second);
        std::string func = func_it->second;
        
        if (win_length > 1 && win_length < static_cast<int>(data.size())) {
            std::vector<double> temp_data = smoothed_data;
            
            for (size_t i = win_length/2; i < data.size() - win_length/2; ++i) {
                std::vector<double> window;
                for (int j = -win_length/2; j <= win_length/2; ++j) {
                    window.push_back(data[i + j]);
                }
                
                if (func == "mean") {
                    smoothed_data[i] = std::accumulate(window.begin(), window.end(), 0.0) / window.size();
                } else if (func == "min") {
                    smoothed_data[i] = *std::min_element(window.begin(), window.end());
                } else if (func == "max") {
                    smoothed_data[i] = *std::max_element(window.begin(), window.end());
                }
            }
        }
    }
    
    return smoothed_data;
}

double CompRealtimeHealth34Plugin::convertToScore(double value, const std::vector<double>& thresholds, double upper_limit) {
    if (thresholds.empty()) return 100.0;
    
    // 使用阈值计算分数
    for (size_t i = 0; i < thresholds.size(); ++i) {
        if (value <= thresholds[i]) {
            return 100.0 - (i * 100.0 / thresholds.size());
        }
    }
    
    // 超过所有阈值，使用上限计算
    if (upper_limit > 0 && value > upper_limit) {
        return 0.0;
    }
    
    return 0.0;
}

void CompRealtimeHealth34Plugin::parseFeatureStats(const std::string& stats_str) {
    // 简化的解析实现
    // 生产环境应使用JSON解析库
    FeatureStat stat;
    stat.analysis_features = "mean_hf";
    stat.analysis_status = "运行状态";
    stat.statistic = {"mean", "std", "max"};
    stat.result_key = "vibration_health";
    stat.thresholds = {100.0, 200.0, 300.0};
    stat.upper_limit = 500.0;
    
    feature_stats_.push_back(stat);
}

void CompRealtimeHealth34Plugin::parseHealthConfigs(const std::string& configs_str) {
    // 简化的解析实现
    // 生产环境应使用JSON解析库
    HealthConfig config;
    config.name = "overall_health";
    config.formula = "weighted_average";
    config.weights = {0.4, 0.3, 0.3};
    config.dependencies = {"vibration_health", "current_health", "temperature_health"};
    
    health_configs_.push_back(config);
}

// ErrorDetectionPluginBase实现
ErrorDetectionPluginBase::ErrorDetectionPluginBase() = default;

bool ErrorDetectionPluginBase::evaluateHealth(std::shared_ptr<PluginData> input, 
                                              std::shared_ptr<PluginResult> output) {
    try {
        // 获取特征数据
        std::map<std::string, double> features;
        
        if (auto feature_data = std::dynamic_pointer_cast<FeatureData>(input)) {
            features = feature_data->getFeatures();
        } else if (auto realtime_data = std::dynamic_pointer_cast<RealTimeData>(input)) {
            features["mean_hf"] = realtime_data->getMeanHF();
            features["mean_lf"] = realtime_data->getMeanLF();
            features["mean"] = realtime_data->getMean();
            features["std"] = realtime_data->getStd();
        }
        
        // 计算错误健康度
        for (const auto& config : getErrorConfigs()) {
            auto it = features.find(config.feature_name);
            if (it != features.end()) {
                auto scores = calculateErrorHealth(config, {it->second});
                for (const auto& [key, value] : scores) {
                    output->setData(key, value);
                    last_scores_[key] = value;
                }
            }
        }
        
        return true;
        
    } catch (const std::exception& e) {
        setError("错误检测异常: " + std::string(e.what()));
        return false;
    }
}

std::vector<double> ErrorDetectionPluginBase::smoothData(const std::vector<double>& data,
                                                         const std::map<std::string, std::string>& smooth_param) {
    std::vector<double> smoothed_data = data;
    
    auto win_length_it = smooth_param.find("win_length");
    auto func_it = smooth_param.find("func");
    
    if (win_length_it != smooth_param.end() && func_it != smooth_param.end()) {
        int win_length = std::stoi(win_length_it->second);
        std::string func = func_it->second;
        
        if (win_length > 1 && win_length < static_cast<int>(data.size())) {
            for (size_t i = win_length/2; i < data.size() - win_length/2; ++i) {
                std::vector<double> window;
                for (int j = -win_length/2; j <= win_length/2; ++j) {
                    window.push_back(data[i + j]);
                }
                
                if (func == "mean") {
                    smoothed_data[i] = std::accumulate(window.begin(), window.end(), 0.0) / window.size();
                } else if (func == "min") {
                    smoothed_data[i] = *std::min_element(window.begin(), window.end());
                } else if (func == "max") {
                    smoothed_data[i] = *std::max_element(window.begin(), window.end());
                }
            }
        }
    }
    
    return smoothed_data;
}

// Error18Plugin实现
Error18Plugin::Error18Plugin() = default;

bool Error18Plugin::validateParameters() {
    // 获取必需参数
    auto threshold_array = parameters_->getDoubleArray("threshold");
    auto upper_limit_array = parameters_->getDoubleArray("upper_limit");
    
    if (threshold_array.empty()) {
        setError("threshold参数不能为空");
        return false;
    }
    
    if (upper_limit_array.empty()) {
        setError("upper_limit参数不能为空");
        return false;
    }
    
    // 转换参数
    thresholds_ = threshold_array;
    upper_limits_ = upper_limit_array;
    
    // 获取可选参数
    auto move_smooth_str = parameters_->getString("move_smooth_param", "");
    auto long_smooth_str = parameters_->getString("long_smooth", "");
    
    parseSmoothParams(move_smooth_str, move_smooth_param_);
    parseSmoothParams(long_smooth_str, long_smooth_);
    
    auto_mode_ = parameters_->getBool("auto", false);
    error_width_ = parameters_->getInt("error_width", 30);
    
    // 获取特征名称
    auto feature_names_array = parameters_->getStringArray("feature_names");
    feature_names_ = feature_names_array;
    
    // 设置健康度定义和默认分数
    health_definitions_ = {"error"};
    default_scores_ = {100};
    
    return true;
}

std::vector<std::string> Error18Plugin::getHealthDefinitions() const {
    return health_definitions_;
}

std::vector<int> Error18Plugin::getDefaultScores() const {
    return default_scores_;
}

std::vector<ErrorDetectionPluginBase::ErrorConfig> Error18Plugin::getErrorConfigs() const {
    std::vector<ErrorConfig> configs;
    
    for (size_t i = 0; i < feature_names_.size() && i < thresholds_.size() && i < upper_limits_.size(); ++i) {
        ErrorConfig config;
        config.feature_name = feature_names_[i];
        config.thresholds = thresholds_[i];
        config.upper_limit = upper_limits_[i];
        config.smooth_param = move_smooth_param_;
        config.error_width = error_width_;
        
        configs.push_back(config);
    }
    
    return configs;
}

std::map<std::string, double> Error18Plugin::calculateErrorHealth(const ErrorConfig& config,
                                                                  const std::vector<double>& data) {
    std::map<std::string, double> scores;
    
    if (data.empty()) {
        scores[config.feature_name + "_error"] = 100.0;
        return scores;
    }
    
    try {
        // 平滑处理
        std::vector<double> smoothed_data = smoothData(data, config.smooth_param);
        
        // 计算错误分数
        double value = smoothed_data.back(); // 使用最新值
        double score = calculateScore(value, config.thresholds, config.upper_limit);
        
        scores[config.feature_name + "_error"] = score;
        
        return scores;
        
    } catch (const std::exception& e) {
        setError("错误健康度计算异常: " + std::string(e.what()));
        scores[config.feature_name + "_error"] = 100.0;
        return scores;
    }
}

void Error18Plugin::parseSmoothParams(const std::string& params_str, std::map<std::string, std::string>& smooth_param) {
    // 简化的解析实现
    // 生产环境应使用JSON解析库
    smooth_param["win_length"] = "10";
    smooth_param["func"] = "min";
}

} // namespace AlgorithmPlugins
