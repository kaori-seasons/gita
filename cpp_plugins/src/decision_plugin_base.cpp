#include "decision_plugin_base.h"
#include <algorithm>
#include <numeric>
#include <cmath>

namespace AlgorithmPlugins {

// DecisionPluginBase实现
DecisionPluginBase::DecisionPluginBase() = default;

bool DecisionPluginBase::initialize(std::shared_ptr<PluginParameter> params) {
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

void DecisionPluginBase::cleanup() {
    initialized_ = false;
    parameters_.reset();
}

// DecisionPluginBase的process()实现
bool DecisionPluginBase::process(std::shared_ptr<PluginData> input, 
                                std::shared_ptr<PluginResult> output) {
    return classifyStatus(input, output);
}

void DecisionPluginBase::addStatusToHistory(int status) {
    status_history_.push_back(status);
    if (status_history_.size() > max_history_size_) {
        status_history_.pop_front();
    }
}

int DecisionPluginBase::getMostFrequentStatus() const {
    if (status_history_.empty()) return -1;
    
    std::map<int, int> status_count;
    for (int status : status_history_) {
        status_count[status]++;
    }
    
    auto max_it = std::max_element(status_count.begin(), status_count.end(),
        [](const std::pair<int, int>& a, const std::pair<int, int>& b) {
            return a.second < b.second;
        });
    
    return max_it->first;
}

bool DecisionPluginBase::isStatusTransition(int current_status, int previous_status) const {
    return current_status != previous_status;
}

// UniversalClassifyPluginBase实现
UniversalClassifyPluginBase::UniversalClassifyPluginBase() = default;

bool UniversalClassifyPluginBase::classifyStatus(std::shared_ptr<PluginData> input, 
                                                std::shared_ptr<PluginResult> output) {
    auto feature_data = std::dynamic_pointer_cast<FeatureData>(input);
    if (!feature_data) {
        setError("输入数据类型错误，期望FeatureData");
        return false;
    }
    
    try {
        // 离线检查
        offlineCheck(std::chrono::system_clock::now());
        
        // 获取特征值
        std::map<std::string, double> features = feature_data->getFeatures();
        
        // 计算各特征状态
        std::vector<int> feature_statuses;
        const auto& thresholds = getThresholds();
        for (size_t i = 0; i < getSelectFeatures().size(); ++i) {
            const auto& feature_name = getSelectFeatures()[i];
            auto it = features.find(feature_name);
            if (it != features.end()) {
                if (i < thresholds.size()) {
                    int status = calculateFeatureStatus(it->second, thresholds[i]);
                    feature_statuses.push_back(status);
                }
            } else {
                setError("缺少特征: " + feature_name);
                return false;
            }
        }
        
        // 计算综合状态
        int overall_status = calculateOverallStatus(feature_statuses);
        
        // 处理过渡状态
        if (prev_status_ != -1 && overall_status != prev_status_) {
            if (handleTransition(overall_status, prev_status_)) {
                overall_status = transition_status_;
            }
            
            if (handleTimeSeriesTransition(overall_status, prev_status_)) {
                overall_status = time_series_status_;
            }
        }
        
        // 更新状态历史
        addStatusToHistory(overall_status);
        prev_status_ = overall_status;
        
        // 输出结果
        output->setData("status", overall_status);
        output->setData("status_name", getStatusMapping().at(overall_status));
        output->setData("confidence", calculateConfidence(feature_statuses));
        
        return true;
        
    } catch (const std::exception& e) {
        setError("状态分类异常: " + std::string(e.what()));
        return false;
    }
}

void UniversalClassifyPluginBase::offlineCheck(std::chrono::system_clock::time_point current_time) {
    if (prev_time_ != std::chrono::system_clock::time_point{}) {
        auto duration = std::chrono::duration_cast<std::chrono::seconds>(current_time - prev_time_);
        if (duration.count() > offline_length_) {
            resetState();
        }
    }
    prev_time_ = current_time;
}

void UniversalClassifyPluginBase::resetState() {
    transition_counter_ = 0;
    close_counter_ = 0;
    time_series_counter_ = 0;
    prev_status_ = -1;
    time_point_[0] = std::chrono::system_clock::time_point{};
    time_point_[1] = std::chrono::system_clock::time_point{};
}

int UniversalClassifyPluginBase::calculateFeatureStatus(double feature_value, const std::vector<double>& threshold) {
    for (size_t i = 0; i < threshold.size(); ++i) {
        if (feature_value <= threshold[i]) {
            return static_cast<int>(i);
        }
    }
    return static_cast<int>(threshold.size());
}

int UniversalClassifyPluginBase::calculateOverallStatus(const std::vector<int>& feature_statuses) {
    if (feature_statuses.empty()) return 0;
    
    // 检查一票否决权
    if (veto_index_ >= 0 && veto_index_ < static_cast<int>(feature_statuses.size())) {
        if (feature_statuses[veto_index_] == 0) {
            return 0; // 停机状态
        }
    }
    
    // 计算运行转特征数量
    int run_count = 0;
    for (int status : feature_statuses) {
        if (status > 0) run_count++;
    }
    
    // 判断综合状态
    if (run_count >= run_feature_num_) {
        return 1; // 运行状态
    } else {
        return 0; // 停机状态
    }
}

double UniversalClassifyPluginBase::calculateConfidence(const std::vector<int>& feature_statuses) {
    if (feature_statuses.empty()) return 0.0;
    
    // 简化的置信度计算（可根据需要改进）
    int run_count = 0;
    for (int status : feature_statuses) {
        if (status > 0) run_count++;
    }
    
    return static_cast<double>(run_count) / feature_statuses.size();
}

// Motor97Plugin实现
Motor97Plugin::Motor97Plugin() = default;

bool Motor97Plugin::validateParameters() {
    // 获取必需参数
    auto select_features_array = parameters_->getStringArray("select_features");
    auto threshold_array = parameters_->getDoubleArray("threshold");
    
    if (select_features_array.empty()) {
        setError("select_features参数不能为空");
        return false;
    }
    
    if (threshold_array.empty()) {
        setError("threshold参数不能为空");
        return false;
    }
    
    // 转换参数
    select_features_ = select_features_array;
    thresholds_.clear();
    for (const auto& threshold : threshold_array) {
        thresholds_.push_back({threshold});
    }
    
    // 获取可选参数
    transition_status_ = parameters_->getInt("transition_status", 2);
    
    // 获取状态映射（简化实现）
    status_mapping_[0] = "Shutdown";
    status_mapping_[1] = "Running";
    status_mapping_[2] = "Transition";
    
    return true;
}

std::vector<std::string> Motor97Plugin::getSelectFeatures() const {
    return select_features_;
}

std::vector<std::vector<double>> Motor97Plugin::getThresholds() const {
    return thresholds_;
}

int Motor97Plugin::classifyByFeatures(const std::map<std::string, double>& features) {
    // 简化的分类逻辑
    return 1;
}

bool Motor97Plugin::handleTransition(int current_status, int previous_status) {
    // 简化的过渡处理
    return false;
}

bool Motor97Plugin::handleTimeSeriesTransition(int current_status, int previous_status) {
    // 简化的时序过渡处理
    return false;
}

void Motor97Plugin::parseAlarmRules(const std::string& rules_str) {
    // 简化的报警规则解析
}

// UniversalClassify1Plugin实现
UniversalClassify1Plugin::UniversalClassify1Plugin() = default;

bool UniversalClassify1Plugin::validateParameters() {
    // 获取必需参数
    auto select_features_array = parameters_->getStringArray("select_features");
    auto threshold_array = parameters_->getDoubleArray("threshold");
    
    if (select_features_array.empty()) {
        setError("select_features参数不能为空");
        return false;
    }
    
    if (threshold_array.empty()) {
        setError("threshold参数不能为空");
        return false;
    }
    
    // 转换参数
    select_features_ = select_features_array;
    thresholds_.clear();
    for (const auto& threshold : threshold_array) {
        thresholds_.push_back({threshold});
    }
    
    // 获取可选参数
    auto statistic_array = parameters_->getStringArray("statistic");
    statistics_ = statistic_array;
    
    auto window_width_array = parameters_->getIntArray("window_width");
    for (int width : window_width_array) {
        window_widths_.push_back({width});
    }
    
    // 初始化滑动窗口
    sliding_windows_.resize(select_features_.size());
    for (auto& window : sliding_windows_) {
        window.resize(statistics_.size());
    }
    
    // 获取状态映射（简化实现）
    status_mapping_[0] = "Shutdown";
    status_mapping_[1] = "Running";
    status_mapping_[2] = "Transition";
    
    return true;
}

std::vector<std::string> UniversalClassify1Plugin::getSelectFeatures() const {
    return select_features_;
}

std::vector<std::vector<double>> UniversalClassify1Plugin::getThresholds() const {
    return thresholds_;
}

int UniversalClassify1Plugin::classifyByFeatures(const std::map<std::string, double>& features) {
    // 简化的分类逻辑
    return 1;
}

bool UniversalClassify1Plugin::handleTransition(int current_status, int previous_status) {
    // 简化的过渡处理
    return false;
}

bool UniversalClassify1Plugin::handleTimeSeriesTransition(int current_status, int previous_status) {
    // 简化的时序过渡处理
    return false;
}

std::vector<double> UniversalClassify1Plugin::extractStatistic(const std::map<std::string, double>& features) {
    std::vector<double> stats;
    // 简化的统计量提取
    return stats;
}

int UniversalClassify1Plugin::calculateFeatureStatus(double feature_value, const std::vector<double>& threshold) {
    for (size_t i = 0; i < threshold.size(); ++i) {
        if (feature_value <= threshold[i]) {
            return static_cast<int>(i);
        }
    }
    return static_cast<int>(threshold.size());
}

int UniversalClassify1Plugin::calculateOverallStatus(const std::vector<int>& feature_statuses) {
    if (feature_statuses.empty()) return 0;
    
    int run_count = 0;
    for (int status : feature_statuses) {
        if (status > 0) run_count++;
    }
    
    return (run_count >= run_feature_num_) ? 1 : 0;
}

void UniversalClassify1Plugin::updateSlidingWindows(const std::vector<double>& stat_features) {
    // 更新滑动窗口逻辑
}

void UniversalClassify1Plugin::clearSlidingWindows() {
    for (size_t i = 0; i < sliding_windows_.size(); ++i) {
        for (size_t j = 0; j < sliding_windows_[i].size(); ++j) {
            sliding_windows_[i][j].clear();
        }
    }
}

} // namespace AlgorithmPlugins
