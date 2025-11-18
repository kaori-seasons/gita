#pragma once

#include "plugin_base.h"
#include "data_types.h"
#include <memory>
#include <vector>
#include <map>

namespace AlgorithmPlugins {

/**
 * @brief 事件类型枚举
 */
enum class EventType {
    SCORE_ALARM = 0,        // 分数异常报警
    PERIOD = 1,             // 周期事件
    PART = 2,               // 零件加工
    QUALITY_INSPECTION = 3, // 质检
    OPERATION_START = 4,    // 操作开始
    OPERATION_STOP = 5,     // 操作结束
    EXTEND_EVENT = 6,       // 扩展事件
    STATUS_ALARM = 7,       // 工况状态异常报警
    INTEGRATE_ALARM = 8,    // 整机报警
    FEATURES_ALARM = 9      // 特征报警
};

/**
 * @brief 事件处理插件基类
 */
class EventPluginBase : public IPlugin {
public:
    EventPluginBase();
    virtual ~EventPluginBase() = default;
    
    // IPlugin接口实现
    PluginType getType() const override { return PluginType::EVENT; }
    bool initialize(std::shared_ptr<PluginParameter> params) override;
    void cleanup() override;
    bool isInitialized() const override { return initialized_; }
    std::string getLastError() const override { return last_error_; }
    
    // 事件处理核心接口
    virtual bool processEvent(std::shared_ptr<PluginData> input, 
                             std::shared_ptr<PluginResult> output) = 0;
    
    // 获取支持的数据类型
    virtual std::vector<DataType> getSupportedInputTypes() const = 0;
    virtual DataType getOutputType() const = 0;
    
    // 获取事件类型
    virtual EventType getEventType() const = 0;

protected:
    bool initialized_ = false;
    std::string last_error_;
    std::shared_ptr<PluginParameter> parameters_;
    
    // 设置错误信息
    void setError(const std::string& error) { last_error_ = error; }
    
    // 参数验证
    virtual bool validateParameters() = 0;
    
    // 事件生成辅助方法
    void generateEvent(std::shared_ptr<PluginResult> output,
                      EventType event_type,
                      const std::string& event_name,
                      const std::string& description,
                      int severity_level = 0);
    
    // 报警级别计算
    int calculateAlarmLevel(double score, const std::vector<double>& alarm_lines);
};

/**
 * @brief 分数报警插件基类
 */
class ScoreAlarmPluginBase : public EventPluginBase {
public:
    ScoreAlarmPluginBase();
    virtual ~ScoreAlarmPluginBase() = default;
    
    // 获取支持的数据类型
    std::vector<DataType> getSupportedInputTypes() const override {
        return {DataType::FEATURE_DATA, DataType::REAL_TIME};
    }
    DataType getOutputType() const override {
        return DataType::FEATURE_DATA; // 输出事件数据
    }
    
    // 事件处理核心接口
    bool processEvent(std::shared_ptr<PluginData> input, 
                     std::shared_ptr<PluginResult> output) override;
    
    EventType getEventType() const override { return EventType::SCORE_ALARM; }

protected:
    // 健康度定义
    virtual std::vector<std::string> getHealthDefinitions() const = 0;
    
    // 报警线配置
    virtual std::vector<double> getAlarmLines() const = 0;
    
    // 报警处理
    virtual bool processAlarm(const std::map<std::string, double>& health_scores) = 0;
    
    // 参数
    int tolerable_length_ = 5;      // 可容忍时长
    int alarm_interval_ = 180;       // 报警间隔（秒）
    
    // 报警状态管理
    std::map<std::string, std::chrono::system_clock::time_point> last_alarm_time_;
    std::map<std::string, int> alarm_count_;
    std::map<std::string, int> tolerable_count_;
    
    // 检查是否应该触发报警
    bool shouldTriggerAlarm(const std::string& health_name, double score);
    
    // 更新报警状态
    void updateAlarmState(const std::string& health_name, double score);
};

/**
 * @brief 分数报警插件V5
 */
class ScoreAlarm5Plugin : public ScoreAlarmPluginBase {
public:
    ScoreAlarm5Plugin();
    virtual ~ScoreAlarm5Plugin() = default;
    
    std::string getName() const override { return "score_alarm5"; }
    std::string getVersion() const override { return "1.0.0"; }
    std::string getDescription() const override { 
        return "分数报警插件V5，基于健康度分数触发报警事件"; 
    }
    
    std::vector<std::string> getRequiredParameters() const override {
        return {"health_define", "alarm_line"};
    }
    std::vector<std::string> getOptionalParameters() const override {
        return {"tolerable_length", "alarm_interval"};
    }

protected:
    bool validateParameters() override;
    std::vector<std::string> getHealthDefinitions() const override;
    std::vector<double> getAlarmLines() const override;
    bool processAlarm(const std::map<std::string, double>& health_scores) override;
    
private:
    std::vector<std::string> health_definitions_;
    std::vector<double> alarm_lines_;
    
    // 报警级别名称
    std::vector<std::string> alarm_level_names_ = {
        "正常", "轻微", "一般", "严重", "危险", "紧急"
    };
};

/**
 * @brief 状态报警插件基类
 */
class StatusAlarmPluginBase : public EventPluginBase {
public:
    StatusAlarmPluginBase();
    virtual ~StatusAlarmPluginBase() = default;
    
    // 获取支持的数据类型
    std::vector<DataType> getSupportedInputTypes() const override {
        return {DataType::STATUS_DATA, DataType::FEATURE_DATA};
    }
    DataType getOutputType() const override {
        return DataType::FEATURE_DATA;
    }
    
    // 事件处理核心接口
    bool processEvent(std::shared_ptr<PluginData> input, 
                     std::shared_ptr<PluginResult> output) override;
    
    EventType getEventType() const override { return EventType::STATUS_ALARM; }

protected:
    // 状态报警配置
    virtual std::map<int, std::string> getStatusMapping() const = 0;
    virtual std::map<int, std::map<std::string, std::string>> getAlarmRules() const = 0;
    
    // 状态报警处理
    virtual bool processStatusAlarm(int status, const std::string& status_name) = 0;
    
    // 参数
    bool alarm_enabled_ = true;
    
    // 报警状态管理
    std::map<int, std::chrono::system_clock::time_point> last_alarm_time_;
    std::map<int, int> alarm_count_;
    std::map<int, int> max_alarm_num_;
    std::map<int, int> recovery_reset_time_;
    std::map<int, int> force_reset_time_;
    
    // 检查是否应该触发状态报警
    bool shouldTriggerStatusAlarm(int status);
    
    // 更新状态报警状态
    void updateStatusAlarmState(int status);
};

/**
 * @brief 状态报警插件V4
 */
class StatusAlarm4Plugin : public StatusAlarmPluginBase {
public:
    StatusAlarm4Plugin();
    virtual ~StatusAlarm4Plugin() = default;
    
    std::string getName() const override { return "status_alarm4"; }
    std::string getVersion() const override { return "1.0.0"; }
    std::string getDescription() const override { 
        return "状态报警插件V4，基于设备状态触发报警事件"; 
    }
    
    std::vector<std::string> getRequiredParameters() const override {
        return {"status_mapping", "alarm_rule"};
    }
    std::vector<std::string> getOptionalParameters() const override {
        return {"alarm"};
    }

protected:
    bool validateParameters() override;
    std::map<int, std::string> getStatusMapping() const override;
    std::map<int, std::map<std::string, std::string>> getAlarmRules() const override;
    bool processStatusAlarm(int status, const std::string& status_name) override;
    
private:
    std::map<int, std::string> status_mapping_;
    std::map<int, std::map<std::string, std::string>> alarm_rules_;
    
    // 解析报警规则
    void parseAlarmRules();
};

} // namespace AlgorithmPlugins
