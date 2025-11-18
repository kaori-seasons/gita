#pragma once

#include "plugin_base.h"
#include "data_types.h"
#include <memory>

namespace AlgorithmPlugins {

/**
 * @brief 特征提取插件基类
 */
class FeaturePluginBase : public IPlugin {
public:
    FeaturePluginBase();
    virtual ~FeaturePluginBase() = default;
    
    // IPlugin接口实现
    PluginType getType() const override { return PluginType::FEATURE; }
    bool initialize(std::shared_ptr<PluginParameter> params) override;
    void cleanup() override;
    bool isInitialized() const override { return initialized_; }
    std::string getLastError() const override { return last_error_; }
    
    // 特征提取核心接口
    virtual bool extractFeatures(std::shared_ptr<PluginData> input, 
                                 std::shared_ptr<PluginResult> output) = 0;
    
    // 获取支持的数据类型
    virtual std::vector<DataType> getSupportedInputTypes() const = 0;
    virtual DataType getOutputType() const = 0;
    
    // 获取特征名称列表
    virtual std::vector<std::string> getFeatureNames() const = 0;

protected:
    bool initialized_ = false;
    std::string last_error_;
    std::shared_ptr<PluginParameter> parameters_;
    
    // 设置错误信息
    void setError(const std::string& error) { last_error_ = error; }
    
    // 参数验证
    virtual bool validateParameters() = 0;
};

/**
 * @brief 振动特征提取插件基类
 */
class VibrationFeaturePluginBase : public FeaturePluginBase {
public:
    VibrationFeaturePluginBase();
    virtual ~VibrationFeaturePluginBase() = default;
    
    // 获取支持的数据类型
    std::vector<DataType> getSupportedInputTypes() const override {
        return {DataType::BATCH_DATA};
    }
    DataType getOutputType() const override {
        return DataType::FEATURE_DATA;
    }
    
    // 特征提取核心接口
    bool extractFeatures(std::shared_ptr<PluginData> input, 
                        std::shared_ptr<PluginResult> output) override;
    
protected:
    // 振动特征计算接口
    virtual bool computeVibrationFeatures(const std::vector<double>& wave_data,
                                          const std::vector<double>& speed_data,
                                          int sampling_rate,
                                          std::map<std::string, double>& features) = 0;
    
    // 波形预处理
    virtual bool preprocessWave(const std::vector<double>& input_wave,
                               std::vector<double>& output_wave);
    
    // 频谱分析
    virtual bool computeSpectrum(const std::vector<double>& wave_data,
                                int sampling_rate,
                                std::vector<double>& frequencies,
                                std::vector<double>& amplitudes);
    
    // 工况分割
    virtual bool segmentByStatus(const std::vector<double>& wave_data,
                                const std::vector<double>& speed_data,
                                std::vector<std::vector<double>>& segments,
                                std::vector<int>& statuses);
    
    // 参数
    int sampling_rate_ = 1000;
    int duration_limit_ = 10;
    double dc_threshold_ = 500.0;
};

/**
 * @brief 实时特征提取插件基类
 */
class RealTimeFeaturePluginBase : public FeaturePluginBase {
public:
    RealTimeFeaturePluginBase();
    virtual ~RealTimeFeaturePluginBase() = default;
    
    // 获取支持的数据类型
    std::vector<DataType> getSupportedInputTypes() const override {
        return {DataType::REAL_TIME};
    }
    DataType getOutputType() const override {
        return DataType::FEATURE_DATA;
    }
    
    // 特征提取核心接口
    bool extractFeatures(std::shared_ptr<PluginData> input, 
                        std::shared_ptr<PluginResult> output) override;
    
protected:
    // 实时特征计算接口
    virtual bool computeRealTimeFeatures(std::shared_ptr<RealTimeData> input_data,
                                        std::map<std::string, double>& features) = 0;
};

/**
 * @brief 电流特征提取插件
 */
class CurrentFeaturePlugin : public RealTimeFeaturePluginBase {
public:
    CurrentFeaturePlugin();
    virtual ~CurrentFeaturePlugin() = default;
    
    std::string getName() const override { return "current_feature_extractor"; }
    std::string getVersion() const override { return "1.0.0"; }
    std::string getDescription() const override { 
        return "电流特征提取插件，计算电流RMS、峰值、波峰因子等特征"; 
    }
    
    std::vector<std::string> getRequiredParameters() const override {
        return {"current_data_key"};
    }
    std::vector<std::string> getOptionalParameters() const override {
        return {"window_size", "smoothing_factor"};
    }
    
    std::vector<std::string> getFeatureNames() const override {
        return {"current_rms", "current_peak", "current_crest", 
                "current_mean", "current_std"};
    }

protected:
    bool validateParameters() override;
    bool computeRealTimeFeatures(std::shared_ptr<RealTimeData> input_data,
                                std::map<std::string, double>& features) override;
    
private:
    std::string current_data_key_ = "current";
    int window_size_ = 10;
    double smoothing_factor_ = 0.1;
};

/**
 * @brief 温度特征提取插件
 */
class TemperatureFeaturePlugin : public RealTimeFeaturePluginBase {
public:
    TemperatureFeaturePlugin();
    virtual ~TemperatureFeaturePlugin() = default;
    
    std::string getName() const override { return "temperature_feature_extractor"; }
    std::string getVersion() const override { return "1.0.0"; }
    std::string getDescription() const override { 
        return "温度特征提取插件，计算温度平均值、最大值、最小值、标准差等特征"; 
    }
    
    std::vector<std::string> getRequiredParameters() const override {
        return {"temperature_data_key"};
    }
    std::vector<std::string> getOptionalParameters() const override {
        return {"window_size", "trend_window"};
    }
    
    std::vector<std::string> getFeatureNames() const override {
        return {"temp_avg", "temp_max", "temp_min", "temp_std", "temp_trend"};
    }

protected:
    bool validateParameters() override;
    bool computeRealTimeFeatures(std::shared_ptr<RealTimeData> input_data,
                                std::map<std::string, double>& features) override;
    
private:
    std::string temperature_data_key_ = "temperature";
    int window_size_ = 10;
    int trend_window_ = 5;
};

/**
 * @brief 声音特征提取插件
 */
class AudioFeaturePlugin : public RealTimeFeaturePluginBase {
public:
    AudioFeaturePlugin();
    virtual ~AudioFeaturePlugin() = default;
    
    std::string getName() const override { return "audio_feature_extractor"; }
    std::string getVersion() const override { return "1.0.0"; }
    std::string getDescription() const override { 
        return "声音特征提取插件，计算音频RMS、频谱特征、峰值频率等特征"; 
    }
    
    std::vector<std::string> getRequiredParameters() const override {
        return {"audio_data_key", "sampling_rate"};
    }
    std::vector<std::string> getOptionalParameters() const override {
        return {"window_size", "fft_size"};
    }
    
    std::vector<std::string> getFeatureNames() const override {
        return {"audio_rms", "audio_spectrum_energy", "audio_peak_freq", 
                "audio_mean_freq", "audio_spectral_centroid"};
    }

protected:
    bool validateParameters() override;
    bool computeRealTimeFeatures(std::shared_ptr<RealTimeData> input_data,
                                std::map<std::string, double>& features) override;
    
private:
    std::string audio_data_key_ = "audio";
    int sampling_rate_ = 44100;
    int window_size_ = 1024;
    int fft_size_ = 2048;
};

} // namespace AlgorithmPlugins
