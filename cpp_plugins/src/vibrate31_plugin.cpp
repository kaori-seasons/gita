#include "vibrate31_plugin.h"
#include <algorithm>
#include <cmath>
#include <numeric>
#include <iostream>

namespace AlgorithmPlugins {

Vibrate31Plugin::Vibrate31Plugin() : VibrationFeaturePluginBase() {
    // 初始化默认参数
    sampling_rate_ = 1000;
    duration_limit_ = 10;
    dc_threshold_ = 500.0;
}

Vibrate31Plugin::~Vibrate31Plugin() = default;

std::string Vibrate31Plugin::getName() const {
    return "vibrate31";
}

std::string Vibrate31Plugin::getVersion() const {
    return "1.0.0";
}

std::string Vibrate31Plugin::getDescription() const {
    return "振动特征提取插件V31，基于频谱分析提取振动特征";
}

std::vector<std::string> Vibrate31Plugin::getRequiredParameters() const {
    return {"sampling_rate"};
}

std::vector<std::string> Vibrate31Plugin::getOptionalParameters() const {
    return {"duration_limit", "dc_threshold", "select_features"};
}

std::vector<std::string> Vibrate31Plugin::getFeatureNames() const {
    return {
        "mean_hf", "mean_lf", "mean", "std",
        "peak_freq", "peak_power", "spectrum_energy",
        "load", "start", "stop"
    };
}

bool Vibrate31Plugin::validateParameters() {
    // 验证采样率
    if (sampling_rate_ <= 0) {
        setError("采样率必须大于0");
        return false;
    }
    
    // 验证时长限制
    if (duration_limit_ <= 0) {
        setError("时长限制必须大于0");
        return false;
    }
    
    return true;
}

bool Vibrate31Plugin::computeVibrationFeatures(const std::vector<double>& wave_data,
                                               const std::vector<double>& speed_data,
                                               int sampling_rate,
                                               std::map<std::string, double>& features) {
    try {
        // 1. 数据长度校验
        double duration = static_cast<double>(wave_data.size()) / sampling_rate;
        if (duration < duration_limit_) {
            setError("波形时长不足，不进行频谱分析特征计算");
            return false;
        }
        
        // 2. 直流量校验
        if (dc_threshold_ > 0) {
            double dc_value = computeDCValue(wave_data, sampling_rate);
            if (dc_value >= dc_threshold_) {
                setError("波形存在严重的直流干扰: " + std::to_string(dc_value));
                return false;
            }
        }
        
        // 3. 工况分割
        std::vector<std::vector<double>> segments;
        std::vector<int> statuses;
        std::vector<std::vector<double>> speed_segments;
        
        if (!segmentByStatus(wave_data, speed_data, segments, statuses)) {
            setError("工况分割失败");
            return false;
        }
        
        // 4. 计算特征
        if (segments.empty()) {
            setError("未识别到有效工况批次");
            return false;
        }
        
        // 计算每个工况段的特征
        std::vector<std::map<std::string, double>> segment_features;
        for (size_t i = 0; i < segments.size(); ++i) {
            if (segments[i].size() / sampling_rate < duration_limit_) {
                continue; // 跳过时长不足的段
            }
            
            std::map<std::string, double> seg_features;
            if (computeSegmentFeatures(segments[i], speed_data, statuses[i], seg_features)) {
                segment_features.push_back(seg_features);
            }
        }
        
        // 5. 合并特征
        if (!mergeSegmentFeatures(segment_features, features)) {
            setError("特征合并失败");
            return false;
        }
        
        return true;
        
    } catch (const std::exception& e) {
        setError("计算振动特征时发生异常: " + std::string(e.what()));
        return false;
    }
}

bool Vibrate31Plugin::computeSegmentFeatures(const std::vector<double>& segment_wave,
                                             const std::vector<double>& speed_data,
                                             int status,
                                             std::map<std::string, double>& features) {
    try {
        // 计算基础统计特征
        features["mean"] = computeMean(segment_wave);
        features["std"] = computeStd(segment_wave);
        features["mean_hf"] = computeMeanHF(segment_wave);
        features["mean_lf"] = computeMeanLF(segment_wave);
        
        // 计算频谱特征
        std::vector<double> frequencies, amplitudes;
        if (computeSpectrum(segment_wave, sampling_rate_, frequencies, amplitudes)) {
            features["peak_freq"] = findPeakFrequency(frequencies, amplitudes);
            features["peak_power"] = findPeakPower(amplitudes);
            features["spectrum_energy"] = computeSpectrumEnergy(amplitudes);
        }
        
        // 添加工况信息
        features["load"] = static_cast<double>(status);
        
        return true;
        
    } catch (const std::exception& e) {
        setError("计算段特征时发生异常: " + std::string(e.what()));
        return false;
    }
}

bool Vibrate31Plugin::mergeSegmentFeatures(const std::vector<std::map<std::string, double>>& segment_features,
                                          std::map<std::string, double>& merged_features) {
    if (segment_features.empty()) {
        return false;
    }
    
    try {
        // 获取所有特征名称
        std::vector<std::string> feature_names;
        for (const auto& name : getFeatureNames()) {
            if (segment_features[0].find(name) != segment_features[0].end()) {
                feature_names.push_back(name);
            }
        }
        
        // 合并特征（使用平均值）
        for (const auto& name : feature_names) {
            double sum = 0.0;
            int count = 0;
            
            for (const auto& seg_features : segment_features) {
                auto it = seg_features.find(name);
                if (it != seg_features.end()) {
                    sum += it->second;
                    count++;
                }
            }
            
            if (count > 0) {
                merged_features[name] = sum / count;
            }
        }
        
        return true;
        
    } catch (const std::exception& e) {
        setError("合并特征时发生异常: " + std::string(e.what()));
        return false;
    }
}

double Vibrate31Plugin::computeDCValue(const std::vector<double>& wave_data, int sampling_rate) {
    // 计算低频成分的能量（近似直流量）
    std::vector<double> frequencies, amplitudes;
    if (!computeSpectrum(wave_data, sampling_rate, frequencies, amplitudes)) {
        return 0.0;
    }
    
    double dc_value = 0.0;
    for (size_t i = 0; i < frequencies.size(); ++i) {
        if (frequencies[i] <= 0.1) { // 0.1Hz以下的频率成分
            dc_value += amplitudes[i];
        }
    }
    
    return dc_value;
}

double Vibrate31Plugin::computeMean(const std::vector<double>& data) {
    if (data.empty()) return 0.0;
    return std::accumulate(data.begin(), data.end(), 0.0) / data.size();
}

double Vibrate31Plugin::computeStd(const std::vector<double>& data) {
    if (data.size() < 2) return 0.0;
    
    double mean = computeMean(data);
    double sum_sq_diff = 0.0;
    
    for (double value : data) {
        double diff = value - mean;
        sum_sq_diff += diff * diff;
    }
    
    return std::sqrt(sum_sq_diff / (data.size() - 1));
}

double Vibrate31Plugin::computeMeanHF(const std::vector<double>& data) {
    // 高频成分的平均值（简化实现）
    return computeMean(data);
}

double Vibrate31Plugin::computeMeanLF(const std::vector<double>& data) {
    // 低频成分的平均值（简化实现）
    return computeMean(data);
}

bool Vibrate31Plugin::computeSpectrum(const std::vector<double>& wave_data,
                                      int sampling_rate,
                                      std::vector<double>& frequencies,
                                      std::vector<double>& amplitudes) {
    try {
        // 简化的FFT实现（实际项目中应使用FFTW或其他专业库）
        int n = wave_data.size();
        if (n < 2) return false;
        
        frequencies.clear();
        amplitudes.clear();
        
        // 计算频率分辨率
        double freq_resolution = static_cast<double>(sampling_rate) / n;
        
        // 计算幅度谱（简化实现）
        for (int i = 0; i < n / 2; ++i) {
            frequencies.push_back(i * freq_resolution);
            
            // 简化的幅度计算
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

double Vibrate31Plugin::findPeakFrequency(const std::vector<double>& frequencies,
                                          const std::vector<double>& amplitudes) {
    if (amplitudes.empty()) return 0.0;
    
    auto max_it = std::max_element(amplitudes.begin(), amplitudes.end());
    size_t max_index = std::distance(amplitudes.begin(), max_it);
    
    return frequencies[max_index];
}

double Vibrate31Plugin::findPeakPower(const std::vector<double>& amplitudes) {
    if (amplitudes.empty()) return 0.0;
    
    auto max_it = std::max_element(amplitudes.begin(), amplitudes.end());
    return *max_it;
}

double Vibrate31Plugin::computeSpectrumEnergy(const std::vector<double>& amplitudes) {
    if (amplitudes.empty()) return 0.0;
    
    double energy = 0.0;
    for (double amplitude : amplitudes) {
        energy += amplitude * amplitude;
    }
    
    return energy;
}

bool Vibrate31Plugin::segmentByStatus(const std::vector<double>& wave_data,
                                     const std::vector<double>& speed_data,
                                     std::vector<std::vector<double>>& segments,
                                     std::vector<int>& statuses) {
    try {
        // 简化的工况分割实现
        // 实际项目中应根据具体需求实现更复杂的工况识别算法
        
        segments.clear();
        statuses.clear();
        
        // 如果数据量较小，直接作为一个段处理
        if (wave_data.size() < sampling_rate_ * duration_limit_) {
            segments.push_back(wave_data);
            statuses.push_back(1); // 默认运行状态
            return true;
        }
        
        // 基于转速变化进行简单分割
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

int Vibrate31Plugin::determineStatus(const std::vector<double>& speed_data,
                                    size_t start, size_t end) {
    if (speed_data.empty()) return 1; // 默认运行状态
    
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

} // namespace AlgorithmPlugins
