#pragma once

#include "feature_plugin_base.h"
#include <vector>
#include <map>

namespace AlgorithmPlugins {

/**
 * @brief 振动特征提取插件V31
 * 
 * 基于频谱分析提取振动特征，支持工况分割和特征计算
 */
class Vibrate31Plugin : public VibrationFeaturePluginBase {
public:
    Vibrate31Plugin();
    virtual ~Vibrate31Plugin();
    
    // IPlugin接口实现
    std::string getName() const override;
    std::string getVersion() const override;
    std::string getDescription() const override;
    
    std::vector<std::string> getRequiredParameters() const override;
    std::vector<std::string> getOptionalParameters() const override;
    std::vector<std::string> getFeatureNames() const override;

protected:
    // FeaturePluginBase接口实现
    bool validateParameters() override;
    
    // VibrationFeaturePluginBase接口实现
    bool computeVibrationFeatures(const std::vector<double>& wave_data,
                                 const std::vector<double>& speed_data,
                                 int sampling_rate,
                                 std::map<std::string, double>& features) override;

private:
    // 核心计算方法
    bool computeSegmentFeatures(const std::vector<double>& segment_wave,
                               const std::vector<double>& speed_data,
                               int status,
                               std::map<std::string, double>& features);
    
    bool mergeSegmentFeatures(const std::vector<std::map<std::string, double>>& segment_features,
                             std::map<std::string, double>& merged_features);
    
    // 特征计算辅助方法
    double computeDCValue(const std::vector<double>& wave_data, int sampling_rate);
    double computeMean(const std::vector<double>& data);
    double computeStd(const std::vector<double>& data);
    double computeMeanHF(const std::vector<double>& data);
    double computeMeanLF(const std::vector<double>& data);
    
    // 频谱分析方法
    bool computeSpectrum(const std::vector<double>& wave_data,
                        int sampling_rate,
                        std::vector<double>& frequencies,
                        std::vector<double>& amplitudes);
    
    double findPeakFrequency(const std::vector<double>& frequencies,
                           const std::vector<double>& amplitudes);
    double findPeakPower(const std::vector<double>& amplitudes);
    double computeSpectrumEnergy(const std::vector<double>& amplitudes);
    
    // 工况分割方法
    bool segmentByStatus(const std::vector<double>& wave_data,
                        const std::vector<double>& speed_data,
                        std::vector<std::vector<double>>& segments,
                        std::vector<int>& statuses);
    
    int determineStatus(const std::vector<double>& speed_data,
                       size_t start, size_t end);
};

} // namespace AlgorithmPlugins
