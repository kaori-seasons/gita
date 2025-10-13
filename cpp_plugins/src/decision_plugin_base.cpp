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

bool DecisionPluginBase::classifyStatus(std::shared_ptr<PluginData> input, 
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
        // 这里应该调用具体的状态分类实现
        // 由于这是基类，子类需要重写classifyStatus方法
        setError("状态分类方法未实现");
        return false;
    } catch (const std::exception& e) {
        setError("状态分类异常: " + std::string(e.what()));
        return false;
    }
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
        // 离线检测
        offlineCheck(std::chrono::system_clock::now());
        
        // 获取特征值
        std::map<std::string, double> features = feature_data->getFeatures();
        
        // 计算各特征状态
        std::vector<int> feature_statuses;
        for (const auto& feature_name : getSelectFeatures()) {
            auto it = features.find(feature_name);
            if (it != features.end()) {
                int status = calculateFeatureStatus(it->second, getThresholds()[feature_statuses.size()]);
                feature_statuses.push_back(status);
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
    
    // 计算运转特征数量
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
    
    // 简化的置信度计算
    int consistent_count = 0;
    int most_frequent = getMostFrequentStatus();
    
    for (int status : feature_statuses) {
        if (status == most_frequent) {
            consistent_count++;
        }
    }
    
    return static_cast<double>(consistent_count) / feature_statuses.size();
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
    thresholds_ = threshold_array;
    
    // 获取可选参数
    transition_status_ = parameters_->getInt("transition_status", 2);
    alarm_enabled_ = parameters_->getBool("alarm", true);
    
    // 解析状态映射
    auto status_mapping_str = parameters_->getString("status_mapping", "");
    if (!status_mapping_str.empty()) {
        parseStatusMapping(status_mapping_str);
    }
    
    // 解析报警规则
    auto alarm_rule_str = parameters_->getString("alarm_rule", "");
    if (!alarm_rule_str.empty()) {
        parseAlarmRules(alarm_rule_str);
    }
    
    return true;
}

std::vector<std::string> Motor97Plugin::getSelectFeatures() const {
    return select_features_;
}

std::vector<std::vector<double>> Motor97Plugin::getThresholds() const {
    return thresholds_;
}

std::map<int, std::string> Motor97Plugin::getStatusMapping() const {
    return status_mapping_;
}

int Motor97Plugin::classifyByFeatures(const std::map<std::string, double>& features) {
    std::vector<int> feature_statuses;
    
    for (size_t i = 0; i < select_features_.size(); ++i) {
        auto it = features.find(select_features_[i]);
        if (it != features.end()) {
            int status = calculateFeatureStatus(it->second, thresholds_[i]);
            feature_statuses.push_back(status);
        } else {
            return 0; // 缺少特征时返回停机状态
        }
    }
    
    return calculateOverallStatus(feature_statuses);
}

bool Motor97Plugin::handleTransition(int current_status, int previous_status) {
    if (current_status == 1 && previous_status == 0) {
        // 开机过渡
        transition_counter_++;
        if (transition_counter_ >= transition_width_[0]) {
            transition_counter_ = 0;
            return true;
        }
    } else if (current_status == 0 && previous_status == 1) {
        // 关机过渡
        close_counter_++;
        if (close_counter_ >= transition_width_[1]) {
            close_counter_ = 0;
            transition_counter_ = 0;
        }
    }
    
    return false;
}

bool Motor97Plugin::handleTimeSeriesTransition(int current_status, int previous_status) {
    if (time_series_width_[0] > 0) {
        if (current_status != previous_status) {
            time_series_counter_++;
            if (time_series_counter_ >= time_series_width_[0]) {
                time_series_counter_ = 0;
                return true;
            }
        } else {
            time_series_counter_ = 0;
        }
    }
    
    return false;
}

int Motor97Plugin::calculateFeatureStatus(double feature_value, const std::vector<double>& threshold) {
    for (size_t i = 0; i < threshold.size(); ++i) {
        if (feature_value <= threshold[i]) {
            return static_cast<int>(i);
        }
    }
    return static_cast<int>(threshold.size());
}

int Motor97Plugin::calculateOverallStatus(const std::vector<int>& feature_statuses) {
    if (feature_statuses.empty()) return 0;
    
    // 简化的综合状态计算
    int run_count = 0;
    for (int status : feature_statuses) {
        if (status > 0) run_count++;
    }
    
    if (run_count >= run_feature_num_) {
        return 1; // 运行状态
    } else {
        return 0; // 停机状态
    }
}

bool Motor97Plugin::processAlarm(int status) {
    if (!alarm_enabled_) return false;
    
    auto it = alarm_rules_.find(std::to_string(status));
    if (it != alarm_rules_.end()) {
        // 处理报警逻辑
        return true;
    }
    
    return false;
}

void Motor97Plugin::parseStatusMapping(const std::string& mapping_str) {
    // 简化的状态映射解析
    // 生产环境应使用JSON解析库
    status_mapping_[0] = "停机";
    status_mapping_[1] = "运行";
    status_mapping_[2] = "过渡";
    status_mapping_[3] = "异常";
}

void Motor97Plugin::parseAlarmRules(const std::string& rules_str) {
    // 简化的报警规则解析
    // 生产环境应使用JSON解析库
    std::map<std::string, std::string> rule;
    rule["push_way"] = "HMI";
    rule["trigger_time"] = "259200";
    rule["max_alarm_num"] = "1";
    rule["recovery_reset_time"] = "3600";
    rule["force_reset_time"] = "604800";
    rule["name"] = "停机";
    
    alarm_rules_["0"] = rule;
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
    thresholds_ = threshold_array;
    
    // 获取可选参数
    auto statistic_array = parameters_->getStringArray("statistic");
    auto window_width_array = parameters_->getIntArray("window_width");
    
    statistics_ = statistic_array;
    window_widths_ = window_width_array;
    
    // 初始化滑动窗口
    initializeSlidingWindows();
    
    // 获取其他参数
    veto_index_ = parameters_->getInt("veto_index", -1);
    run_feature_num_ = parameters_->getInt("run_feature_num", 1);
    transition_status_ = parameters_->getInt("transition_status", 2);
    time_series_status_ = parameters_->getInt("time_series_status", 5);
    
    auto transition_width_array = parameters_->getIntArray("transition_width");
    auto time_series_width_array = parameters_->getIntArray("time_series_width");
    
    if (!transition_width_array.empty()) {
        transition_width_ = {transition_width_array[0], transition_width_array[1]};
    }
    
    if (!time_series_width_array.empty()) {
        time_series_width_ = {time_series_width_array[0], time_series_width_array[1]};
    }
    
    offline_length_ = parameters_->getInt("offline_length", 3600);
    
    return true;
}

std::vector<std::string> UniversalClassify1Plugin::getSelectFeatures() const {
    return select_features_;
}

std::vector<std::vector<double>> UniversalClassify1Plugin::getThresholds() const {
    return thresholds_;
}

std::map<int, std::string> UniversalClassify1Plugin::getStatusMapping() const {
    return status_mapping_;
}

int UniversalClassify1Plugin::classifyByFeatures(const std::map<std::string, double>& features) {
    // 提取统计量
    auto stat_features = extractStatistic(features);
    
    // 更新滑动窗口
    updateSlidingWindows(stat_features);
    
    // 计算各特征状态
    std::vector<int> feature_statuses;
    for (size_t i = 0; i < stat_features.size(); ++i) {
        int status = calculateFeatureStatus(stat_features[i], thresholds_[i]);
        feature_statuses.push_back(status);
    }
    
    return calculateOverallStatus(feature_statuses);
}

bool UniversalClassify1Plugin::handleTransition(int current_status, int previous_status) {
    // 实现过渡状态处理逻辑
    return UniversalClassifyPluginBase::handleTransition(current_status, previous_status);
}

bool UniversalClassify1Plugin::handleTimeSeriesTransition(int current_status, int previous_status) {
    // 实现时序过渡处理逻辑
    return UniversalClassifyPluginBase::handleTimeSeriesTransition(current_status, previous_status);
}

std::vector<double> UniversalClassify1Plugin::extractStatistic(const std::map<std::string, double>& features) {
    std::vector<double> stat_features;
    
    for (size_t i = 0; i < select_features_.size(); ++i) {
        auto it = features.find(select_features_[i]);
        if (it != features.end()) {
            stat_features.push_back(it->second);
        } else {
            stat_features.push_back(0.0);
        }
    }
    
    return stat_features;
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
    
    // 检查一票否决权
    if (veto_index_ >= 0 && veto_index_ < static_cast<int>(feature_statuses.size())) {
        if (feature_statuses[veto_index_] == 0) {
            return 0; // 停机状态
        }
    }
    
    // 计算运转特征数量
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

void UniversalClassify1Plugin::initializeSlidingWindows() {
    sliding_windows_.clear();
    
    for (size_t i = 0; i < select_features_.size(); ++i) {
        if (i < statistics_.size() && !statistics_[i].empty()) {
            std::vector<std::deque<double>> windows;
            for (int width : window_widths_[i]) {
                windows.emplace_back(width);
            }
            sliding_windows_.push_back(windows);
        } else {
            sliding_windows_.push_back({});
        }
    }
}

void UniversalClassify1Plugin::updateSlidingWindows(const std::vector<double>& stat_features) {
    for (size_t i = 0; i < stat_features.size() && i < sliding_windows_.size(); ++i) {
        for (auto& window : sliding_windows_[i]) {
            window.push_back(stat_features[i]);
        }
    }
}

void UniversalClassify1Plugin::clearSlidingWindows() {
    for (auto& feature_windows : sliding_windows_) {
        for (auto& window : feature_windows) {
            window.clear();
        }
    }
}

} // namespace AlgorithmPlugins
