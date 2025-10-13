#include "event_plugin_base.h"
#include <algorithm>
#include <chrono>

namespace AlgorithmPlugins {

// EventPluginBase实现
EventPluginBase::EventPluginBase() = default;

bool EventPluginBase::initialize(std::shared_ptr<PluginParameter> params) {
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

void EventPluginBase::cleanup() {
    initialized_ = false;
    parameters_.reset();
}

bool EventPluginBase::processEvent(std::shared_ptr<PluginData> input, 
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
        // 这里应该调用具体的事件处理实现
        // 由于这是基类，子类需要重写processEvent方法
        setError("事件处理方法未实现");
        return false;
    } catch (const std::exception& e) {
        setError("事件处理异常: " + std::string(e.what()));
        return false;
    }
}

void EventPluginBase::generateEvent(std::shared_ptr<PluginResult> output,
                                    EventType event_type,
                                    const std::string& event_name,
                                    const std::string& description,
                                    int severity_level) {
    output->setData("event_type", static_cast<int>(event_type));
    output->setData("event_name", event_name);
    output->setData("event_description", description);
    output->setData("severity_level", severity_level);
    output->setData("timestamp", std::chrono::duration_cast<std::chrono::milliseconds>(
        std::chrono::system_clock::now().time_since_epoch()).count());
}

int EventPluginBase::calculateAlarmLevel(double score, const std::vector<double>& alarm_lines) {
    if (alarm_lines.empty()) return 0;
    
    for (size_t i = 0; i < alarm_lines.size(); ++i) {
        if (score <= alarm_lines[i]) {
            return static_cast<int>(i + 1);
        }
    }
    
    return static_cast<int>(alarm_lines.size() + 1);
}

// ScoreAlarmPluginBase实现
ScoreAlarmPluginBase::ScoreAlarmPluginBase() = default;

bool ScoreAlarmPluginBase::processEvent(std::shared_ptr<PluginData> input, 
                                       std::shared_ptr<PluginResult> output) {
    try {
        // 获取健康度数据
        std::map<std::string, double> health_scores;
        
        if (auto feature_data = std::dynamic_pointer_cast<FeatureData>(input)) {
            for (const auto& health_name : getHealthDefinitions()) {
                double score = feature_data->getFeature(health_name);
                health_scores[health_name] = score;
            }
        } else if (auto realtime_data = std::dynamic_pointer_cast<RealTimeData>(input)) {
            for (const auto& health_name : getHealthDefinitions()) {
                double score = realtime_data->getCustomFeature(health_name);
                health_scores[health_name] = score;
            }
        }
        
        // 处理报警
        bool alarm_triggered = processAlarm(health_scores);
        
        if (alarm_triggered) {
            // 生成报警事件
            for (const auto& [health_name, score] : health_scores) {
                if (shouldTriggerAlarm(health_name, score)) {
                    int alarm_level = calculateAlarmLevel(score, getAlarmLines());
                    std::string event_name = "健康度报警_" + health_name;
                    std::string description = "设备健康度下降: " + std::to_string(score);
                    
                    generateEvent(output, EventType::SCORE_ALARM, event_name, description, alarm_level);
                    
                    // 更新报警状态
                    updateAlarmState(health_name, score);
                }
            }
        }
        
        return true;
        
    } catch (const std::exception& e) {
        setError("分数报警处理异常: " + std::string(e.what()));
        return false;
    }
}

bool ScoreAlarmPluginBase::shouldTriggerAlarm(const std::string& health_name, double score) {
    auto last_time_it = last_alarm_time_.find(health_name);
    auto count_it = alarm_count_.find(health_name);
    auto tolerable_it = tolerable_count_.find(health_name);
    
    // 检查报警间隔
    if (last_time_it != last_alarm_time_.end()) {
        auto now = std::chrono::system_clock::now();
        auto duration = std::chrono::duration_cast<std::chrono::seconds>(now - last_time_it->second);
        if (duration.count() < alarm_interval_) {
            return false;
        }
    }
    
    // 检查可容忍次数
    int current_count = (count_it != alarm_count_.end()) ? count_it->second : 0;
    int tolerable_count = (tolerable_it != tolerable_count_.end()) ? tolerable_it->second : 0;
    
    if (current_count < tolerable_count) {
        return false;
    }
    
    return true;
}

void ScoreAlarmPluginBase::updateAlarmState(const std::string& health_name, double score) {
    last_alarm_time_[health_name] = std::chrono::system_clock::now();
    
    auto count_it = alarm_count_.find(health_name);
    if (count_it != alarm_count_.end()) {
        count_it->second++;
    } else {
        alarm_count_[health_name] = 1;
    }
    
    // 重置可容忍计数
    tolerable_count_[health_name] = 0;
}

// ScoreAlarm5Plugin实现
ScoreAlarm5Plugin::ScoreAlarm5Plugin() = default;

bool ScoreAlarm5Plugin::validateParameters() {
    // 获取必需参数
    auto health_def_array = parameters_->getStringArray("health_define");
    auto alarm_line_array = parameters_->getDoubleArray("alarm_line");
    
    if (health_def_array.empty()) {
        setError("health_define参数不能为空");
        return false;
    }
    
    if (alarm_line_array.empty()) {
        setError("alarm_line参数不能为空");
        return false;
    }
    
    // 转换参数
    health_definitions_ = health_def_array;
    alarm_lines_ = alarm_line_array;
    
    // 获取可选参数
    tolerable_length_ = parameters_->getInt("tolerable_length", 5);
    alarm_interval_ = parameters_->getInt("alarm_interval", 180);
    
    return true;
}

std::vector<std::string> ScoreAlarm5Plugin::getHealthDefinitions() const {
    return health_definitions_;
}

std::vector<double> ScoreAlarm5Plugin::getAlarmLines() const {
    return alarm_lines_;
}

bool ScoreAlarm5Plugin::processAlarm(const std::map<std::string, double>& health_scores) {
    bool alarm_triggered = false;
    
    for (const auto& [health_name, score] : health_scores) {
        if (shouldTriggerAlarm(health_name, score)) {
            alarm_triggered = true;
            break;
        }
    }
    
    return alarm_triggered;
}

// StatusAlarmPluginBase实现
StatusAlarmPluginBase::StatusAlarmPluginBase() = default;

bool StatusAlarmPluginBase::processEvent(std::shared_ptr<PluginData> input, 
                                        std::shared_ptr<PluginResult> output) {
    try {
        // 获取状态数据
        int status = 0;
        std::string status_name = "未知";
        
        if (auto status_data = std::dynamic_pointer_cast<StatusData>(input)) {
            status = status_data->getStatus();
            status_name = status_data->getStatusName(status);
        } else if (auto feature_data = std::dynamic_pointer_cast<FeatureData>(input)) {
            status = static_cast<int>(feature_data->getFeature("status"));
            auto status_mapping = getStatusMapping();
            auto it = status_mapping.find(status);
            if (it != status_mapping.end()) {
                status_name = it->second;
            }
        }
        
        // 处理状态报警
        if (shouldTriggerStatusAlarm(status)) {
            bool alarm_triggered = processStatusAlarm(status, status_name);
            
            if (alarm_triggered) {
                // 生成状态报警事件
                std::string event_name = "状态报警_" + status_name;
                std::string description = "设备状态异常: " + status_name;
                
                generateEvent(output, EventType::STATUS_ALARM, event_name, description, 1);
                
                // 更新状态报警状态
                updateStatusAlarmState(status);
            }
        }
        
        return true;
        
    } catch (const std::exception& e) {
        setError("状态报警处理异常: " + std::string(e.what()));
        return false;
    }
}

bool StatusAlarmPluginBase::shouldTriggerStatusAlarm(int status) {
    if (!alarm_enabled_) return false;
    
    auto last_time_it = last_alarm_time_.find(status);
    auto count_it = alarm_count_.find(status);
    auto max_num_it = max_alarm_num_.find(status);
    
    // 检查最大报警次数
    int current_count = (count_it != alarm_count_.end()) ? count_it->second : 0;
    int max_count = (max_num_it != max_alarm_num_.end()) ? max_num_it->second : 10;
    
    if (current_count >= max_count) {
        return false;
    }
    
    // 检查报警间隔
    if (last_time_it != last_alarm_time_.end()) {
        auto now = std::chrono::system_clock::now();
        auto duration = std::chrono::duration_cast<std::chrono::seconds>(now - last_time_it->second);
        
        auto recovery_it = recovery_reset_time_.find(status);
        int recovery_time = (recovery_it != recovery_reset_time_.end()) ? recovery_it->second : 3600;
        
        if (duration.count() < recovery_time) {
            return false;
        }
    }
    
    return true;
}

void StatusAlarmPluginBase::updateStatusAlarmState(int status) {
    last_alarm_time_[status] = std::chrono::system_clock::now();
    
    auto count_it = alarm_count_.find(status);
    if (count_it != alarm_count_.end()) {
        count_it->second++;
    } else {
        alarm_count_[status] = 1;
    }
}

// StatusAlarm4Plugin实现
StatusAlarm4Plugin::StatusAlarm4Plugin() = default;

bool StatusAlarm4Plugin::validateParameters() {
    // 获取必需参数
    auto status_mapping_str = parameters_->getString("status_mapping", "");
    auto alarm_rule_str = parameters_->getString("alarm_rule", "");
    
    if (status_mapping_str.empty()) {
        setError("status_mapping参数不能为空");
        return false;
    }
    
    if (alarm_rule_str.empty()) {
        setError("alarm_rule参数不能为空");
        return false;
    }
    
    // 解析参数
    parseStatusMapping(status_mapping_str);
    parseAlarmRules(alarm_rule_str);
    
    // 获取可选参数
    alarm_enabled_ = parameters_->getBool("alarm", true);
    
    return true;
}

std::map<int, std::string> StatusAlarm4Plugin::getStatusMapping() const {
    return status_mapping_;
}

std::map<int, std::map<std::string, std::string>> StatusAlarm4Plugin::getAlarmRules() const {
    return alarm_rules_;
}

bool StatusAlarm4Plugin::processStatusAlarm(int status, const std::string& status_name) {
    auto it = alarm_rules_.find(std::to_string(status));
    if (it != alarm_rules_.end()) {
        // 处理报警逻辑
        return true;
    }
    
    return false;
}

void StatusAlarm4Plugin::parseStatusMapping(const std::string& mapping_str) {
    // 简化的状态映射解析
    // 生产环境应使用JSON解析库
    status_mapping_[0] = "停机";
    status_mapping_[1] = "运行";
    status_mapping_[2] = "过渡";
    status_mapping_[3] = "异常";
}

void StatusAlarm4Plugin::parseAlarmRules(const std::string& rules_str) {
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

} // namespace AlgorithmPlugins
