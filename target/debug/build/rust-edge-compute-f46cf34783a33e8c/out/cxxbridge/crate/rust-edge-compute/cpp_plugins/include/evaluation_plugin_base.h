#pragma once

#include "plugin_base.h"
#include "data_types.h"
#include <memory>
#include <vector>
#include <map>

namespace AlgorithmPlugins {

/**
 * @brief 健康评估插件基类
 */
class EvaluationPluginBase : public IPlugin {
public:
    EvaluationPluginBase();
    virtual ~EvaluationPluginBase() = default;
    
    // IPlugin接口实现
    PluginType getType() const override { return PluginType::EVALUATION; }
    bool initialize(std::shared_ptr<PluginParameter> params) override;
    void cleanup() override;
    bool isInitialized() const override { return initialized_; }
    std::string getLastError() const override { return last_error_; }
    
    // 健康评估核心接口
    virtual bool evaluateHealth(std::shared_ptr<PluginData> input, 
                               std::shared_ptr<PluginResult> output) = 0;
    
    // 获取支持的数据类型
    virtual std::vector<DataType> getSupportedInputTypes() const = 0;
    virtual DataType getOutputType() const = 0;
    
    // 获取健康度定义
    virtual std::vector<std::string> getHealthDefinitions() const = 0;
    virtual std::vector<int> getDefaultScores() const = 0;

protected:
    bool initialized_ = false;
    std::string last_error_;
    std::shared_ptr<PluginParameter> parameters_;
    
    // 设置错误信息
    void setError(const std::string& error) { last_error_ = error; }
    
    // 参数验证
    virtual bool validateParameters() = 0;
    
    // 健康度计算辅助方法
    double calculateScore(double value, double threshold_low, double threshold_high);
    double calculateScore(double value, const std::vector<double>& thresholds);
    std::map<std::string, double> mergeHealthScores(const std::vector<std::map<std::string, double>>& scores);
};

/**
 * @brief 实时健康度评估插件基类
 */
class RealtimeHealthPluginBase : public EvaluationPluginBase {
public:
    RealtimeHealthPluginBase();
    virtual ~RealtimeHealthPluginBase() = default;
    
    // 获取支持的数据类型
    std::vector<DataType> getSupportedInputTypes() const override {
        return {DataType::FEATURE_DATA, DataType::REAL_TIME};
    }
    DataType getOutputType() const override {
        return DataType::FEATURE_DATA; // 输出健康度特征
    }
    
    // 健康评估核心接口
    bool evaluateHealth(std::shared_ptr<PluginData> input, 
                       std::shared_ptr<PluginResult> output) override;
    
protected:
    // 特征统计分析
    struct FeatureStat {
        std::string analysis_features;    // 分析的特征名称
        std::string analysis_status;     // 分析的状态
        std::vector<std::string> statistic; // 统计方法
        std::string result_key;          // 结果键名
        std::vector<double> thresholds;  // 阈值
        double upper_limit;              // 上限
        std::map<std::string, std::string> clean_formula; // 清洗方法
        std::map<std::string, std::string> move_smooth_param; // 移动平滑参数
        std::map<std::string, std::string> long_smooth; // 长期平滑参数
    };
    
    // 健康度配置
    struct HealthConfig {
        std::string name;                // 健康度名称
        std::string formula;             // 计算公式
        std::vector<double> weights;     // 权重
        std::vector<std::string> dependencies; // 依赖的特征
    };
    
    // 特征统计分析
    virtual std::vector<FeatureStat> getFeatureStats() const = 0;
    
    // 健康度配置
    virtual std::vector<HealthConfig> getHealthConfigs() const = 0;
    
    // 核心计算方法
    virtual std::map<std::string, double> calculateFeatureHealth(const FeatureStat& stat,
                                                                 const std::vector<double>& data) = 0;
    
    virtual std::map<std::string, double> calculateOverallHealth(const std::vector<HealthConfig>& configs,
                                                                const std::map<std::string, double>& feature_scores) = 0;
    
    // 数据缓存管理
    std::map<std::string, std::vector<double>> feature_cache_;
    std::map<std::string, std::vector<std::chrono::system_clock::time_point>> time_cache_;
    std::map<std::string, double> last_health_scores_;
    
    // 参数
    int offline_length_ = 86400 * 15;  // 离线重置时长（15天）
    int minimum_quantity_ = 30;         // 最小数据量
    int close_width_ = 1;               // 非关注状态持续时长
    
    // 状态管理
    int current_status_ = -1;
    int close_count_ = 0;
    int run_count_ = 0;
    std::chrono::system_clock::time_point prev_time_;
    
    // 离线检测
    void offlineCheck(std::chrono::system_clock::time_point current_time);
    
    // 状态检查和数据缓存
    bool statusCheckAndCacheData(std::shared_ptr<PluginData> input_data,
                                std::chrono::system_clock::time_point current_time);
    
    // 重置缓存
    void resetCache(bool all_cache = true);
};

/**
 * @brief 实时健康度评估插件V34
 */
class CompRealtimeHealth34Plugin : public RealtimeHealthPluginBase {
public:
    CompRealtimeHealth34Plugin();
    virtual ~CompRealtimeHealth34Plugin() = default;
    
    std::string getName() const override { return "comp_realtime_health34"; }
    std::string getVersion() const override { return "1.0.0"; }
    std::string getDescription() const override { 
        return "实时健康度评估插件V34，基于多特征统计分析计算健康度"; 
    }
    
    std::vector<std::string> getRequiredParameters() const override {
        return {"feature_stats", "healths"};
    }
    std::vector<std::string> getOptionalParameters() const override {
        return {"offline_length", "minimum_quantity", "close_width"};
    }
    
    std::vector<std::string> getHealthDefinitions() const override;
    std::vector<int> getDefaultScores() const override;

protected:
    bool validateParameters() override;
    std::vector<FeatureStat> getFeatureStats() const override;
    std::vector<HealthConfig> getHealthConfigs() const override;
    std::map<std::string, double> calculateFeatureHealth(const FeatureStat& stat,
                                                        const std::vector<double>& data) override;
    std::map<std::string, double> calculateOverallHealth(const std::vector<HealthConfig>& configs,
                                                        const std::map<std::string, double>& feature_scores) override;
    
private:
    std::vector<FeatureStat> feature_stats_;
    std::vector<HealthConfig> health_configs_;
    std::vector<std::string> health_definitions_;
    std::vector<int> default_scores_;
    
    // 统计量计算
    double calculateStatistic(const std::vector<double>& data, const std::string& method);
    
    // 数据清洗
    std::vector<double> cleanData(const std::vector<double>& data, 
                                 const std::map<std::string, std::string>& clean_formula);
    
    // 平滑处理
    std::vector<double> smoothData(const std::vector<double>& data,
                                 const std::map<std::string, std::string>& smooth_param);
    
    // 分数转换
    double convertToScore(double value, const std::vector<double>& thresholds, double upper_limit);
};

/**
 * @brief 错误检测插件基类
 */
class ErrorDetectionPluginBase : public EvaluationPluginBase {
public:
    ErrorDetectionPluginBase();
    virtual ~ErrorDetectionPluginBase() = default;
    
    // 获取支持的数据类型
    std::vector<DataType> getSupportedInputTypes() const override {
        return {DataType::FEATURE_DATA, DataType::REAL_TIME};
    }
    DataType getOutputType() const override {
        return DataType::FEATURE_DATA;
    }
    
    // 健康评估核心接口
    bool evaluateHealth(std::shared_ptr<PluginData> input, 
                       std::shared_ptr<PluginResult> output) override;
    
protected:
    // 错误检测配置
    struct ErrorConfig {
        std::string feature_name;        // 特征名称
        std::vector<double> thresholds;  // 阈值
        double upper_limit;              // 上限
        std::map<std::string, std::string> smooth_param; // 平滑参数
        int error_width;                 // 错误宽度
    };
    
    // 错误检测配置
    virtual std::vector<ErrorConfig> getErrorConfigs() const = 0;
    
    // 核心计算方法
    virtual std::map<std::string, double> calculateErrorHealth(const ErrorConfig& config,
                                                             const std::vector<double>& data) = 0;
    
    // 参数
    bool auto_mode_ = false;
    int error_width_ = 30;
    
    // 数据缓存
    std::map<std::string, std::vector<double>> feature_cache_;
    std::map<std::string, double> last_scores_;
    
    // 平滑处理
    std::vector<double> smoothData(const std::vector<double>& data,
                                 const std::map<std::string, std::string>& smooth_param);
};

/**
 * @brief 错误检测插件V18
 */
class Error18Plugin : public ErrorDetectionPluginBase {
public:
    Error18Plugin();
    virtual ~Error18Plugin() = default;
    
    std::string getName() const override { return "error18"; }
    std::string getVersion() const override { return "1.0.0"; }
    std::string getDescription() const override { 
        return "错误检测插件V18，基于多特征阈值检测设备异常"; 
    }
    
    std::vector<std::string> getRequiredParameters() const override {
        return {"threshold", "upper_limit"};
    }
    std::vector<std::string> getOptionalParameters() const override {
        return {"auto", "error_width", "move_smooth_param", "long_smooth"};
    }
    
    std::vector<std::string> getHealthDefinitions() const override;
    std::vector<int> getDefaultScores() const override;

protected:
    bool validateParameters() override;
    std::vector<ErrorConfig> getErrorConfigs() const override;
    std::map<std::string, double> calculateErrorHealth(const ErrorConfig& config,
                                                      const std::vector<double>& data) override;
    
private:
    std::vector<std::vector<double>> thresholds_;
    std::vector<double> upper_limits_;
    std::map<std::string, std::string> move_smooth_param_;
    std::map<std::string, std::string> long_smooth_;
    
    std::vector<std::string> health_definitions_;
    std::vector<int> default_scores_;
    
    // 特征名称（从参数中获取）
    std::vector<std::string> feature_names_;
};

} // namespace AlgorithmPlugins
