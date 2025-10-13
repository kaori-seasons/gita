#include "feature_plugin_base.h"
#include <algorithm>
#include <numeric>
#include <cmath>

namespace AlgorithmPlugins {

// FeaturePluginBase实现
FeaturePluginBase::FeaturePluginBase() = default;

bool FeaturePluginBase::initialize(std::shared_ptr<PluginParameter> params) {
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

void FeaturePluginBase::cleanup() {
    initialized_ = false;
    parameters_.reset();
}

bool FeaturePluginBase::extractFeatures(std::shared_ptr<PluginData> input, 
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
        // 这里应该调用具体的特征提取实现
        // 由于这是基类，子类需要重写extractFeatures方法
        setError("特征提取方法未实现");
        return false;
    } catch (const std::exception& e) {
        setError("特征提取异常: " + std::string(e.what()));
        return false;
    }
}

// VibrationFeaturePluginBase实现
VibrationFeaturePluginBase::VibrationFeaturePluginBase() = default;

bool VibrationFeaturePluginBase::extractFeatures(std::shared_ptr<PluginData> input, 
                                                 std::shared_ptr<PluginResult> output) {
    auto batch_data = std::dynamic_pointer_cast<BatchData>(input);
    if (!batch_data) {
        setError("输入数据类型错误，期望BatchData");
        return false;
    }
    
    std::map<std::string, double> features;
    bool success = computeVibrationFeatures(
        batch_data->getWaveData(),
        batch_data->getSpeedData(),
        batch_data->getSamplingRate(),
        features
    );
    
    if (success) {
        for (const auto& [key, value] : features) {
            output->setData(key, value);
        }
    }
    
    return success;
}

bool VibrationFeaturePluginBase::preprocessWave(const std::vector<double>& input_wave,
                                                std::vector<double>& output_wave) {
    try {
        output_wave = input_wave;
        
        // 去除直流分量
        if (!input_wave.empty()) {
            double mean = std::accumulate(input_wave.begin(), input_wave.end(), 0.0) / input_wave.size();
            for (double& value : output_wave) {
                value -= mean;
            }
        }
        
        return true;
    } catch (const std::exception& e) {
        setError("波形预处理异常: " + std::string(e.what()));
        return false;
    }
}

bool VibrationFeaturePluginBase::computeSpectrum(const std::vector<double>& wave_data,
                                                 int sampling_rate,
                                                 std::vector<double>& frequencies,
                                                 std::vector<double>& amplitudes) {
    try {
        int n = wave_data.size();
        if (n < 2) {
            setError("波形数据长度不足");
            return false;
        }
        
        frequencies.clear();
        amplitudes.clear();
        
        // 计算频率分辨率
        double freq_resolution = static_cast<double>(sampling_rate) / n;
        
        // 简化的FFT实现（生产环境应使用FFTW）
        for (int i = 0; i < n / 2; ++i) {
            frequencies.push_back(i * freq_resolution);
            
            // 计算幅度
            double real = 0.0, imag = 0.0;
            for (int j = 0; j < n; ++j) {
                double angle = -2.0 * M_PI * i * j / n;
                real += wave_data[j] * std::cos(angle);
                imag += wave_data[j] * std::sin(angle);
            }
            
            double amplitude = std::sqrt(real * real + imag * imag) / n;
            amplitudes.push_back(amplitude);
        }
        
        return true;
    } catch (const std::exception& e) {
        setError("频谱计算异常: " + std::string(e.what()));
        return false;
    }
}

bool VibrationFeaturePluginBase::segmentByStatus(const std::vector<double>& wave_data,
                                                 const std::vector<double>& speed_data,
                                                 std::vector<std::vector<double>>& segments,
                                                 std::vector<int>& statuses) {
    try {
        segments.clear();
        statuses.clear();
        
        // 简化的工况分割实现
        if (wave_data.size() < sampling_rate_ * duration_limit_) {
            segments.push_back(wave_data);
            statuses.push_back(1); // 默认运行状态
            return true;
        }
        
        // 基于转速变化进行分割
        int segment_size = sampling_rate_ * 30; // 30秒一段
        for (size_t i = 0; i < wave_data.size(); i += segment_size) {
            size_t end = std::min(i + segment_size, wave_data.size());
            
            std::vector<double> segment(wave_data.begin() + i, wave_data.begin() + end);
            segments.push_back(segment);
            
            // 根据转速判断工况
            int status = determineStatus(speed_data, i, end);
            statuses.push_back(status);
        }
        
        return true;
    } catch (const std::exception& e) {
        setError("工况分割异常: " + std::string(e.what()));
        return false;
    }
}

int VibrationFeaturePluginBase::determineStatus(const std::vector<double>& speed_data,
                                               size_t start, size_t end) {
    if (speed_data.empty()) return 1;
    
    // 计算该段的平均转速
    double avg_speed = 0.0;
    int count = 0;
    
    for (size_t i = start; i < end && i < speed_data.size(); ++i) {
        avg_speed += speed_data[i];
        count++;
    }
    
    if (count == 0) return 1;
    avg_speed /= count;
    
    // 简化的状态判断逻辑
    if (avg_speed < 10.0) return 0;      // 停机
    else if (avg_speed < 50.0) return 2; // 过渡
    else return 1;                        // 运行
}

// RealTimeFeaturePluginBase实现
RealTimeFeaturePluginBase::RealTimeFeaturePluginBase() = default;

bool RealTimeFeaturePluginBase::extractFeatures(std::shared_ptr<PluginData> input, 
                                               std::shared_ptr<PluginResult> output) {
    auto realtime_data = std::dynamic_pointer_cast<RealTimeData>(input);
    if (!realtime_data) {
        setError("输入数据类型错误，期望RealTimeData");
        return false;
    }
    
    std::map<std::string, double> features;
    bool success = computeRealTimeFeatures(realtime_data, features);
    
    if (success) {
        for (const auto& [key, value] : features) {
            output->setData(key, value);
        }
    }
    
    return success;
}

// CurrentFeaturePlugin实现
CurrentFeaturePlugin::CurrentFeaturePlugin() = default;

bool CurrentFeaturePlugin::validateParameters() {
    current_data_key_ = parameters_->getString("current_data_key", "current");
    window_size_ = parameters_->getInt("window_size", 10);
    smoothing_factor_ = parameters_->getDouble("smoothing_factor", 0.1);
    
    if (window_size_ <= 0) {
        setError("窗口大小必须大于0");
        return false;
    }
    
    if (smoothing_factor_ < 0.0 || smoothing_factor_ > 1.0) {
        setError("平滑因子必须在0-1之间");
        return false;
    }
    
    return true;
}

bool CurrentFeaturePlugin::computeRealTimeFeatures(std::shared_ptr<RealTimeData> input_data,
                                                  std::map<std::string, double>& features) {
    try {
        // 获取电流数据
        double current_value = input_data->getCustomFeature(current_data_key_);
        
        // 计算基础特征
        features["current_rms"] = std::abs(current_value);
        features["current_peak"] = std::abs(current_value);
        features["current_mean"] = current_value;
        features["current_std"] = 0.0; // 单点数据无法计算标准差
        
        // 计算波峰因子
        if (features["current_rms"] > 0) {
            features["current_crest"] = features["current_peak"] / features["current_rms"];
        } else {
            features["current_crest"] = 0.0;
        }
        
        return true;
    } catch (const std::exception& e) {
        setError("电流特征计算异常: " + std::string(e.what()));
        return false;
    }
}

// TemperatureFeaturePlugin实现
TemperatureFeaturePlugin::TemperatureFeaturePlugin() = default;

bool TemperatureFeaturePlugin::validateParameters() {
    temperature_data_key_ = parameters_->getString("temperature_data_key", "temperature");
    window_size_ = parameters_->getInt("window_size", 10);
    trend_window_ = parameters_->getInt("trend_window", 5);
    
    if (window_size_ <= 0) {
        setError("窗口大小必须大于0");
        return false;
    }
    
    if (trend_window_ <= 0) {
        setError("趋势窗口大小必须大于0");
        return false;
    }
    
    return true;
}

bool TemperatureFeaturePlugin::computeRealTimeFeatures(std::shared_ptr<RealTimeData> input_data,
                                                      std::map<std::string, double>& features) {
    try {
        // 获取温度数据
        double temperature_value = input_data->getTemperature();
        
        // 计算基础特征
        features["temp_avg"] = temperature_value;
        features["temp_max"] = temperature_value;
        features["temp_min"] = temperature_value;
        features["temp_std"] = 0.0; // 单点数据无法计算标准差
        
        // 计算趋势（简化实现）
        features["temp_trend"] = 0.0; // 需要历史数据才能计算趋势
        
        return true;
    } catch (const std::exception& e) {
        setError("温度特征计算异常: " + std::string(e.what()));
        return false;
    }
}

// AudioFeaturePlugin实现
AudioFeaturePlugin::AudioFeaturePlugin() = default;

bool AudioFeaturePlugin::validateParameters() {
    audio_data_key_ = parameters_->getString("audio_data_key", "audio");
    sampling_rate_ = parameters_->getInt("sampling_rate", 44100);
    window_size_ = parameters_->getInt("window_size", 1024);
    fft_size_ = parameters_->getInt("fft_size", 2048);
    
    if (sampling_rate_ <= 0) {
        setError("采样率必须大于0");
        return false;
    }
    
    if (window_size_ <= 0) {
        setError("窗口大小必须大于0");
        return false;
    }
    
    if (fft_size_ <= 0) {
        setError("FFT大小必须大于0");
        return false;
    }
    
    return true;
}

bool AudioFeaturePlugin::computeRealTimeFeatures(std::shared_ptr<RealTimeData> input_data,
                                                 std::map<std::string, double>& features) {
    try {
        // 获取音频数据
        double audio_value = input_data->getCustomFeature(audio_data_key_);
        
        // 计算基础特征
        features["audio_rms"] = std::abs(audio_value);
        features["audio_spectrum_energy"] = audio_value * audio_value;
        features["audio_peak_freq"] = 0.0; // 需要频谱分析
        features["audio_mean_freq"] = 0.0; // 需要频谱分析
        features["audio_spectral_centroid"] = 0.0; // 需要频谱分析
        
        return true;
    } catch (const std::exception& e) {
        setError("音频特征计算异常: " + std::string(e.what()));
        return false;
    }
}

} // namespace AlgorithmPlugins
