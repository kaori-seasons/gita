#pragma once

#include "plugin_base.h"
#include <vector>
#include <map>
#include <memory>

namespace AlgorithmPlugins {

/**
 * @brief 实时数据类
 */
class RealTimeData : public PluginData {
public:
    RealTimeData(const std::string& deviceId, 
                 std::chrono::system_clock::time_point timestamp);
    
    // 基础特征数据
    void setMeanHF(double value) { mean_hf_ = value; }
    void setMeanLF(double value) { mean_lf_ = value; }
    void setMean(double value) { mean_ = value; }
    void setStd(double value) { std_ = value; }
    
    double getMeanHF() const { return mean_hf_; }
    double getMeanLF() const { return mean_lf_; }
    double getMean() const { return mean_; }
    double getStd() const { return std_; }
    
    // 高分辨率特征数据
    void setFeature1(double value) { feature1_ = value; }
    void setFeature2(double value) { feature2_ = value; }
    void setFeature3(double value) { feature3_ = value; }
    void setFeature4(double value) { feature4_ = value; }
    
    double getFeature1() const { return feature1_; }
    double getFeature2() const { return feature2_; }
    double getFeature3() const { return feature3_; }
    double getFeature4() const { return feature4_; }
    
    // 其他数据
    void setTemperature(double value) { temperature_ = value; }
    void setSpeed(double value) { speed_ = value; }
    void setPeakFreq(double value) { peak_freq_ = value; }
    void setPeakPowers(double value) { peak_powers_ = value; }
    
    double getTemperature() const { return temperature_; }
    double getSpeed() const { return speed_; }
    double getPeakFreq() const { return peak_freq_; }
    double getPeakPowers() const { return peak_powers_; }
    
    // 自定义特征
    void setCustomFeature(const std::string& key, double value) {
        custom_features_[key] = value;
    }
    double getCustomFeature(const std::string& key) const {
        auto it = custom_features_.find(key);
        return (it != custom_features_.end()) ? it->second : 0.0;
    }
    
    // 扩展数据
    void setExtendData(const std::string& key, const std::string& value) {
        extend_data_[key] = value;
    }
    std::string getExtendData(const std::string& key) const {
        auto it = extend_data_.find(key);
        return (it != extend_data_.end()) ? it->second : "";
    }
    
    // PluginData接口实现
    DataType getType() const override { return DataType::REAL_TIME; }
    std::chrono::system_clock::time_point getTimestamp() const override { return timestamp_; }
    std::string getDeviceId() const override { return device_id_; }
    std::string serialize() const override;
    bool deserialize(const std::string& data) override;

private:
    std::string device_id_;
    std::chrono::system_clock::time_point timestamp_;
    
    // 基础特征
    double mean_hf_ = 0.0;
    double mean_lf_ = 0.0;
    double mean_ = 0.0;
    double std_ = 0.0;
    
    // 高分辨率特征
    double feature1_ = 0.0;
    double feature2_ = 0.0;
    double feature3_ = 0.0;
    double feature4_ = 0.0;
    
    // 其他数据
    double temperature_ = 0.0;
    double speed_ = 0.0;
    double peak_freq_ = 0.0;
    double peak_powers_ = 0.0;
    
    // 自定义特征和扩展数据
    std::map<std::string, double> custom_features_;
    std::map<std::string, std::string> extend_data_;
};

/**
 * @brief 批次数据类（如振动数据）
 */
class BatchData : public PluginData {
public:
    BatchData(const std::string& deviceId, 
              std::chrono::system_clock::time_point timestamp);
    
    // 波形数据
    void setWaveData(const std::vector<double>& wave) { wave_data_ = wave; }
    const std::vector<double>& getWaveData() const { return wave_data_; }
    
    // 转速数据
    void setSpeedData(const std::vector<double>& speed) { speed_data_ = speed; }
    const std::vector<double>& getSpeedData() const { return speed_data_; }
    
    // 采样率
    void setSamplingRate(int rate) { sampling_rate_ = rate; }
    int getSamplingRate() const { return sampling_rate_; }
    
    // 工况状态
    void setStatus(int status) { status_ = status; }
    int getStatus() const { return status_; }
    
    // 波形分割信息
    void setStartIndex(int start) { start_index_ = start; }
    void setStopIndex(int stop) { stop_index_ = stop; }
    int getStartIndex() const { return start_index_; }
    int getStopIndex() const { return stop_index_; }
    
    // PluginData接口实现
    DataType getType() const override { return DataType::BATCH_DATA; }
    std::chrono::system_clock::time_point getTimestamp() const override { return timestamp_; }
    std::string getDeviceId() const override { return device_id_; }
    std::string serialize() const override;
    bool deserialize(const std::string& data) override;

private:
    std::string device_id_;
    std::chrono::system_clock::time_point timestamp_;
    std::vector<double> wave_data_;
    std::vector<double> speed_data_;
    int sampling_rate_ = 1000;
    int status_ = 0;
    int start_index_ = 0;
    int stop_index_ = 0;
};

/**
 * @brief 特征数据类
 */
class FeatureData : public PluginData {
public:
    FeatureData(const std::string& deviceId, 
                std::chrono::system_clock::time_point timestamp);
    
    // 特征数据管理
    void setFeature(const std::string& name, double value) {
        features_[name] = value;
    }
    double getFeature(const std::string& name) const {
        auto it = features_.find(name);
        return (it != features_.end()) ? it->second : 0.0;
    }
    bool hasFeature(const std::string& name) const {
        return features_.find(name) != features_.end();
    }
    
    // 批量设置特征
    void setFeatures(const std::map<std::string, double>& features) {
        features_ = features;
    }
    const std::map<std::string, double>& getFeatures() const {
        return features_;
    }
    
    // PluginData接口实现
    DataType getType() const override { return DataType::FEATURE_DATA; }
    std::chrono::system_clock::time_point getTimestamp() const override { return timestamp_; }
    std::string getDeviceId() const override { return device_id_; }
    std::string serialize() const override;
    bool deserialize(const std::string& data) override;

private:
    std::string device_id_;
    std::chrono::system_clock::time_point timestamp_;
    std::map<std::string, double> features_;
};

/**
 * @brief 状态数据类
 */
class StatusData : public PluginData {
public:
    StatusData(const std::string& deviceId, 
               std::chrono::system_clock::time_point timestamp);
    
    // 状态值
    void setStatus(int status) { status_ = status; }
    int getStatus() const { return status_; }
    
    // 状态描述
    void setStatusDescription(const std::string& desc) { status_desc_ = desc; }
    std::string getStatusDescription() const { return status_desc_; }
    
    // 状态映射
    void setStatusMapping(const std::map<int, std::string>& mapping) {
        status_mapping_ = mapping;
    }
    std::string getStatusName(int status) const {
        auto it = status_mapping_.find(status);
        return (it != status_mapping_.end()) ? it->second : "Unknown";
    }
    
    // PluginData接口实现
    DataType getType() const override { return DataType::STATUS_DATA; }
    std::chrono::system_clock::time_point getTimestamp() const override { return timestamp_; }
    std::string getDeviceId() const override { return device_id_; }
    std::string serialize() const override;
    bool deserialize(const std::string& data) override;

private:
    std::string device_id_;
    std::chrono::system_clock::time_point timestamp_;
    int status_ = 0;
    std::string status_desc_;
    std::map<int, std::string> status_mapping_;
};

/**
 * @brief 插件结果实现类
 */
class PluginResultImpl : public PluginResult {
public:
    PluginResultImpl();
    
    // PluginResult接口实现
    void setData(const std::string& key, const std::string& value) override;
    void setData(const std::string& key, double value) override;
    void setData(const std::string& key, int value) override;
    
    std::string getStringData(const std::string& key) const override;
    double getDoubleData(const std::string& key) const override;
    int getIntData(const std::string& key) const override;
    
    bool hasData(const std::string& key) const override;
    std::string serialize() const override;
    bool deserialize(const std::string& data) override;
    
    // 数组参数访问方法
    std::vector<std::string> getStringArray(const std::string& key) const;
    std::vector<double> getDoubleArray(const std::string& key) const;
    std::vector<int> getIntArray(const std::string& key) const;

private:
    std::map<std::string, std::string> string_data_;
    std::map<std::string, double> double_data_;
    std::map<std::string, int> int_data_;
};

/**
 * @brief 插件参数实现类
 */
class PluginParameterImpl : public PluginParameter {
public:
    PluginParameterImpl();
    
    // PluginParameter接口实现
    std::string getString(const std::string& key, const std::string& defaultValue = "") const override;
    double getDouble(const std::string& key, double defaultValue = 0.0) const override;
    int getInt(const std::string& key, int defaultValue = 0) const override;
    bool getBool(const std::string& key, bool defaultValue = false) const override;
    std::vector<double> getDoubleArray(const std::string& key) const override;
    std::vector<int> getIntArray(const std::string& key) const override;
    
    void setString(const std::string& key, const std::string& value) override;
    void setDouble(const std::string& key, double value) override;
    void setInt(const std::string& key, int value) override;
    void setBool(const std::string& key, bool value) override;
    void setDoubleArray(const std::string& key, const std::vector<double>& value) override;
    void setIntArray(const std::string& key, const std::vector<int>& value) override;
    
    std::string serialize() const override;
    bool deserialize(const std::string& data) override;

private:
    std::map<std::string, std::string> string_params_;
    std::map<std::string, double> double_params_;
    std::map<std::string, int> int_params_;
    std::map<std::string, bool> bool_params_;
    std::map<std::string, std::vector<double>> double_array_params_;
    std::map<std::string, std::vector<int>> int_array_params_;
};

} // namespace AlgorithmPlugins
