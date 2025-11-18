#pragma once

#include "plugin_base.h"
#include "data_types.h"
#include <memory>
#include <deque>

namespace AlgorithmPlugins {

/**
 * @brief 状态识别插件基类
 */
class DecisionPluginBase : public IPlugin {
public:
    DecisionPluginBase();
    virtual ~DecisionPluginBase() = default;
    
    // IPlugin接口实现
    PluginType getType() const override { return PluginType::DECISION; }
    bool initialize(std::shared_ptr<PluginParameter> params) override;
    void cleanup() override;
    bool isInitialized() const override { return initialized_; }
    std::string getLastError() const override { return last_error_; }
    
    // 状态识别核心接口
    virtual bool classifyStatus(std::shared_ptr<PluginData> input, 
                               std::shared_ptr<PluginResult> output) = 0;
    
    // 获取支持的数据类型
    virtual std::vector<DataType> getSupportedInputTypes() const = 0;
    virtual DataType getOutputType() const = 0;
    
    // 获取状态映射
    virtual std::map<int, std::string> getStatusMapping() const = 0;

protected:
    bool initialized_ = false;
    std::string last_error_;
    std::shared_ptr<PluginParameter> parameters_;
    
    // 设置错误信息
    void setError(const std::string& error) { last_error_ = error; }
    
    // 参数验证
    virtual bool validateParameters() = 0;
    
    // 状态历史管理
    std::deque<int> status_history_;
    size_t max_history_size_ = 10;
    
    void addStatusToHistory(int status);
    int getMostFrequentStatus() const;
    bool isStatusTransition(int current_status, int previous_status) const;
};

/**
 * @brief 通用状态识别插件基类
 */
class UniversalClassifyPluginBase : public DecisionPluginBase {
public:
    UniversalClassifyPluginBase();
    virtual ~UniversalClassifyPluginBase() = default;
    
    // 获取支持的数据类型
    std::vector<DataType> getSupportedInputTypes() const override {
        return {DataType::FEATURE_DATA, DataType::REAL_TIME};
    }
    DataType getOutputType() const override {
        return DataType::STATUS_DATA;
    }
    
    // 状态识别核心接口
    bool classifyStatus(std::shared_ptr<PluginData> input, 
                       std::shared_ptr<PluginResult> output) override;
    
protected:
    // 特征选择
    virtual std::vector<std::string> getSelectFeatures() const = 0;
    
    // 阈值配置
    virtual std::vector<std::vector<double>> getThresholds() const = 0;
    
    // 状态映射
    virtual std::map<int, std::string> getStatusMapping() const override = 0;
    
    // 核心分类逻辑
    virtual int classifyByFeatures(const std::map<std::string, double>& features) = 0;
    
    // 过渡状态处理
    virtual bool handleTransition(int current_status, int previous_status) = 0;
    
    // 时序过渡处理
    virtual bool handleTimeSeriesTransition(int current_status, int previous_status) = 0;
    
    // 参数
    int offline_length_ = 3600;  // 离线重置时长（秒）
    int transition_status_ = 2;   // 过渡状态值
    int time_series_status_ = 5;   // 时序过渡状态值
    std::vector<int> transition_width_ = {60, 10};  // 过渡时长配置
    std::vector<int> time_series_width_ = {0, 0};   // 时序过渡时长配置
    int run_feature_num_ = 1;     // 运转特征数量
    int veto_index_ = -1;          // 一票否决权索引
    
    // 状态历史
    std::chrono::system_clock::time_point prev_time_;
    std::chrono::system_clock::time_point time_point_[2]; // 关机、开机时间点
    int transition_counter_ = 0;
    int close_counter_ = 0;
    int time_series_counter_ = 0;
    int prev_status_ = -1;
    
    // 离线检测
    void offlineCheck(std::chrono::system_clock::time_point current_time);
    
    // 重置状态
    void resetState();
};

/**
 * @brief 电机状态识别插件
 */
class Motor97Plugin : public UniversalClassifyPluginBase {
public:
    Motor97Plugin();
    virtual ~Motor97Plugin() = default;
    
    std::string getName() const override { return "motor97"; }
    std::string getVersion() const override { return "1.0.0"; }
    std::string getDescription() const override { 
        return "电机状态识别插件M97，基于多特征阈值进行状态分类"; 
    }
    
    std::vector<std::string> getRequiredParameters() const override {
        return {"select_features", "threshold"};
    }
    std::vector<std::string> getOptionalParameters() const override {
        return {"transition_status", "alarm", "alarm_rule", "status_mapping"};
    }
    
    std::map<int, std::string> getStatusMapping() const override {
        return status_mapping_;
    }

protected:
    bool validateParameters() override;
    std::vector<std::string> getSelectFeatures() const override;
    std::vector<std::vector<double>> getThresholds() const override;
    int classifyByFeatures(const std::map<std::string, double>& features) override;
    bool handleTransition(int current_status, int previous_status) override;
    bool handleTimeSeriesTransition(int current_status, int previous_status) override;
    
private:
    std::vector<std::string> select_features_;
    std::vector<std::vector<double>> thresholds_;
    std::map<int, std::string> status_mapping_;
    bool alarm_enabled_ = false;
    std::map<std::string, std::map<std::string, std::string>> alarm_rules_;
    
    // 特征状态计算
    int calculateFeatureStatus(double feature_value, const std::vector<double>& threshold);
    
    // 综合状态计算
    int calculateOverallStatus(const std::vector<int>& feature_statuses);
    
    // 报警处理
    bool processAlarm(int status);
};

/**
 * @brief 通用分类器插件
 */
class UniversalClassify1Plugin : public UniversalClassifyPluginBase {
public:
    UniversalClassify1Plugin();
    virtual ~UniversalClassify1Plugin() = default;
    
    std::string getName() const override { return "universal_classify1"; }
    std::string getVersion() const override { return "1.0.0"; }
    std::string getDescription() const override { 
        return "通用状态分类器插件UC1，支持多特征、多阈值、统计量分析"; 
    }
    
    std::vector<std::string> getRequiredParameters() const override {
        return {"select_features", "threshold"};
    }
    std::vector<std::string> getOptionalParameters() const override {
        return {"statistic", "window_width", "veto_index", "run_feature_num",
                "transition_status", "transition_width", "time_series_status", 
                "time_series_width", "offline_length"};
    }
    
    std::map<int, std::string> getStatusMapping() const override {
        return status_mapping_;
    }

protected:
    bool validateParameters() override;
    std::vector<std::string> getSelectFeatures() const override;
    std::vector<std::vector<double>> getThresholds() const override;
    int classifyByFeatures(const std::map<std::string, double>& features) override;
    bool handleTransition(int current_status, int previous_status) override;
    bool handleTimeSeriesTransition(int current_status, int previous_status) override;
    
private:
    std::vector<std::string> select_features_;
    std::vector<std::vector<double>> thresholds_;
    std::vector<std::string> statistics_;
    std::vector<std::vector<int>> window_widths_;
    std::map<int, std::string> status_mapping_;
    
    // 滑动窗口数据
    std::vector<std::vector<std::deque<double>>> sliding_windows_;
    
    // 统计量计算
    std::vector<double> extractStatistic(const std::map<std::string, double>& features);
    
    // 特征状态计算
    int calculateFeatureStatus(double feature_value, const std::vector<double>& threshold);
    
    // 综合状态计算
    int calculateOverallStatus(const std::vector<int>& feature_statuses);
    
    // 滑动窗口管理
    void updateSlidingWindows(const std::vector<double>& stat_features);
    void clearSlidingWindows();
};

} // namespace AlgorithmPlugins
