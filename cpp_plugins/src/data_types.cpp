#include "data_types.h"
#include <sstream>
#include <iomanip>
#include <cmath>

namespace AlgorithmPlugins {

// RealTimeData实现
RealTimeData::RealTimeData(const std::string& deviceId, 
                           std::chrono::system_clock::time_point timestamp)
    : device_id_(deviceId), timestamp_(timestamp) {
}

std::string RealTimeData::serialize() const {
    std::ostringstream oss;
    oss << std::fixed << std::setprecision(6);
    
    oss << "{"
        << "\"device_id\":\"" << device_id_ << "\","
        << "\"timestamp\":" << std::chrono::duration_cast<std::chrono::milliseconds>(
            timestamp_.time_since_epoch()).count() << ","
        << "\"type\":\"real_time\","
        << "\"mean_hf\":" << mean_hf_ << ","
        << "\"mean_lf\":" << mean_lf_ << ","
        << "\"mean\":" << mean_ << ","
        << "\"std\":" << std_ << ","
        << "\"feature1\":" << feature1_ << ","
        << "\"feature2\":" << feature2_ << ","
        << "\"feature3\":" << feature3_ << ","
        << "\"feature4\":" << feature4_ << ","
        << "\"temperature\":" << temperature_ << ","
        << "\"speed\":" << speed_ << ","
        << "\"peak_freq\":" << peak_freq_ << ","
        << "\"peak_powers\":" << peak_powers_;
    
    // 添加自定义特征
    if (!custom_features_.empty()) {
        oss << ",\"custom_features\":{";
        bool first = true;
        for (const auto& [key, value] : custom_features_) {
            if (!first) oss << ",";
            oss << "\"" << key << "\":" << value;
            first = false;
        }
        oss << "}";
    }
    
    // 添加扩展数据
    if (!extend_data_.empty()) {
        oss << ",\"extend_data\":{";
        bool first = true;
        for (const auto& [key, value] : extend_data_) {
            if (!first) oss << ",";
            oss << "\"" << key << "\":\"" << value << "\"";
            first = false;
        }
        oss << "}";
    }
    
    oss << "}";
    return oss.str();
}

bool RealTimeData::deserialize(const std::string& data) {
    // 简化的JSON解析实现
    // 生产环境中应使用专业的JSON库
    try {
        // 这里应该实现完整的JSON解析
        // 为了简化，这里只做基本验证
        if (data.find("\"type\":\"real_time\"") == std::string::npos) {
            return false;
        }
        
        // 提取device_id
        size_t start = data.find("\"device_id\":\"") + 13;
        size_t end = data.find("\"", start);
        if (start != std::string::npos && end != std::string::npos) {
            device_id_ = data.substr(start, end - start);
        }
        
        // 提取数值字段
        auto extractDouble = [&](const std::string& key) -> double {
            std::string search = "\"" + key + "\":";
            size_t pos = data.find(search);
            if (pos != std::string::npos) {
                pos += search.length();
                size_t end = data.find_first_of(",}", pos);
                if (end != std::string::npos) {
                    return std::stod(data.substr(pos, end - pos));
                }
            }
            return 0.0;
        };
        
        mean_hf_ = extractDouble("mean_hf");
        mean_lf_ = extractDouble("mean_lf");
        mean_ = extractDouble("mean");
        std_ = extractDouble("std");
        feature1_ = extractDouble("feature1");
        feature2_ = extractDouble("feature2");
        feature3_ = extractDouble("feature3");
        feature4_ = extractDouble("feature4");
        temperature_ = extractDouble("temperature");
        speed_ = extractDouble("speed");
        peak_freq_ = extractDouble("peak_freq");
        peak_powers_ = extractDouble("peak_powers");
        
        return true;
    } catch (const std::exception& e) {
        return false;
    }
}

// BatchData实现
BatchData::BatchData(const std::string& deviceId, 
                     std::chrono::system_clock::time_point timestamp)
    : device_id_(deviceId), timestamp_(timestamp) {
}

std::string BatchData::serialize() const {
    std::ostringstream oss;
    oss << std::fixed << std::setprecision(6);
    
    oss << "{"
        << "\"device_id\":\"" << device_id_ << "\","
        << "\"timestamp\":" << std::chrono::duration_cast<std::chrono::milliseconds>(
            timestamp_.time_since_epoch()).count() << ","
        << "\"type\":\"batch_data\","
        << "\"sampling_rate\":" << sampling_rate_ << ","
        << "\"status\":" << status_ << ","
        << "\"start_index\":" << start_index_ << ","
        << "\"stop_index\":" << stop_index_ << ","
        << "\"wave_data\":[";
    
    // 序列化波形数据
    for (size_t i = 0; i < wave_data_.size(); ++i) {
        if (i > 0) oss << ",";
        oss << wave_data_[i];
    }
    
    oss << "],\"speed_data\":[";
    
    // 序列化转速数据
    for (size_t i = 0; i < speed_data_.size(); ++i) {
        if (i > 0) oss << ",";
        oss << speed_data_[i];
    }
    
    oss << "]}";
    return oss.str();
}

bool BatchData::deserialize(const std::string& data) {
    try {
        if (data.find("\"type\":\"batch_data\"") == std::string::npos) {
            return false;
        }
        
        // 提取device_id
        size_t start = data.find("\"device_id\":\"") + 13;
        size_t end = data.find("\"", start);
        if (start != std::string::npos && end != std::string::npos) {
            device_id_ = data.substr(start, end - start);
        }
        
        // 提取数值字段
        auto extractInt = [&](const std::string& key) -> int {
            std::string search = "\"" + key + "\":";
            size_t pos = data.find(search);
            if (pos != std::string::npos) {
                pos += search.length();
                size_t end = data.find_first_of(",}", pos);
                if (end != std::string::npos) {
                    return std::stoi(data.substr(pos, end - pos));
                }
            }
            return 0;
        };
        
        sampling_rate_ = extractInt("sampling_rate");
        status_ = extractInt("status");
        start_index_ = extractInt("start_index");
        stop_index_ = extractInt("stop_index");
        
        // 提取数组数据
        auto extractArray = [&](const std::string& key) -> std::vector<double> {
            std::string search = "\"" + key + "\":[";
            size_t start = data.find(search);
            if (start == std::string::npos) return {};
            
            start += search.length();
            size_t end = data.find("]", start);
            if (end == std::string::npos) return {};
            
            std::string arrayStr = data.substr(start, end - start);
            std::vector<double> result;
            
            std::istringstream iss(arrayStr);
            std::string token;
            while (std::getline(iss, token, ',')) {
                if (!token.empty()) {
                    result.push_back(std::stod(token));
                }
            }
            
            return result;
        };
        
        wave_data_ = extractArray("wave_data");
        speed_data_ = extractArray("speed_data");
        
        return true;
    } catch (const std::exception& e) {
        return false;
    }
}

// FeatureData实现
FeatureData::FeatureData(const std::string& deviceId, 
                         std::chrono::system_clock::time_point timestamp)
    : device_id_(deviceId), timestamp_(timestamp) {
}

std::string FeatureData::serialize() const {
    std::ostringstream oss;
    oss << std::fixed << std::setprecision(6);
    
    oss << "{"
        << "\"device_id\":\"" << device_id_ << "\","
        << "\"timestamp\":" << std::chrono::duration_cast<std::chrono::milliseconds>(
            timestamp_.time_since_epoch()).count() << ","
        << "\"type\":\"feature_data\","
        << "\"features\":{";
    
    bool first = true;
    for (const auto& [key, value] : features_) {
        if (!first) oss << ",";
        oss << "\"" << key << "\":" << value;
        first = false;
    }
    
    oss << "}}";
    return oss.str();
}

bool FeatureData::deserialize(const std::string& data) {
    try {
        if (data.find("\"type\":\"feature_data\"") == std::string::npos) {
            return false;
        }
        
        // 提取device_id
        size_t start = data.find("\"device_id\":\"") + 13;
        size_t end = data.find("\"", start);
        if (start != std::string::npos && end != std::string::npos) {
            device_id_ = data.substr(start, end - start);
        }
        
        // 提取特征数据
        size_t featuresStart = data.find("\"features\":{") + 11;
        size_t featuresEnd = data.find("}", featuresStart);
        if (featuresStart != std::string::npos && featuresEnd != std::string::npos) {
            std::string featuresStr = data.substr(featuresStart, featuresEnd - featuresStart);
            
            std::istringstream iss(featuresStr);
            std::string token;
            while (std::getline(iss, token, ',')) {
                size_t colonPos = token.find(':');
                if (colonPos != std::string::npos) {
                    std::string key = token.substr(0, colonPos);
                    std::string value = token.substr(colonPos + 1);
                    
                    // 清理引号
                    key.erase(std::remove(key.begin(), key.end(), '"'), key.end());
                    key.erase(std::remove(key.begin(), key.end(), ' '), key.end());
                    
                    if (!key.empty() && !value.empty()) {
                        features_[key] = std::stod(value);
                    }
                }
            }
        }
        
        return true;
    } catch (const std::exception& e) {
        return false;
    }
}

// StatusData实现
StatusData::StatusData(const std::string& deviceId, 
                       std::chrono::system_clock::time_point timestamp)
    : device_id_(deviceId), timestamp_(timestamp) {
}

std::string StatusData::serialize() const {
    std::ostringstream oss;
    
    oss << "{"
        << "\"device_id\":\"" << device_id_ << "\","
        << "\"timestamp\":" << std::chrono::duration_cast<std::chrono::milliseconds>(
            timestamp_.time_since_epoch()).count() << ","
        << "\"type\":\"status_data\","
        << "\"status\":" << status_ << ","
        << "\"status_desc\":\"" << status_desc_ << "\","
        << "\"status_mapping\":{";
    
    bool first = true;
    for (const auto& [key, value] : status_mapping_) {
        if (!first) oss << ",";
        oss << "\"" << key << "\":\"" << value << "\"";
        first = false;
    }
    
    oss << "}}";
    return oss.str();
}

bool StatusData::deserialize(const std::string& data) {
    try {
        if (data.find("\"type\":\"status_data\"") == std::string::npos) {
            return false;
        }
        
        // 提取device_id
        size_t start = data.find("\"device_id\":\"") + 13;
        size_t end = data.find("\"", start);
        if (start != std::string::npos && end != std::string::npos) {
            device_id_ = data.substr(start, end - start);
        }
        
        // 提取status
        size_t statusStart = data.find("\"status\":") + 9;
        size_t statusEnd = data.find(",", statusStart);
        if (statusStart != std::string::npos && statusEnd != std::string::npos) {
            status_ = std::stoi(data.substr(statusStart, statusEnd - statusStart));
        }
        
        // 提取status_desc
        size_t descStart = data.find("\"status_desc\":\"") + 15;
        size_t descEnd = data.find("\"", descStart);
        if (descStart != std::string::npos && descEnd != std::string::npos) {
            status_desc_ = data.substr(descStart, descEnd - descStart);
        }
        
        return true;
    } catch (const std::exception& e) {
        return false;
    }
}

// PluginResultImpl实现
PluginResultImpl::PluginResultImpl() = default;

void PluginResultImpl::setData(const std::string& key, const std::string& value) {
    string_data_[key] = value;
}

void PluginResultImpl::setData(const std::string& key, double value) {
    double_data_[key] = value;
}

void PluginResultImpl::setData(const std::string& key, int value) {
    int_data_[key] = value;
}

std::string PluginResultImpl::getStringData(const std::string& key) const {
    auto it = string_data_.find(key);
    return (it != string_data_.end()) ? it->second : "";
}

double PluginResultImpl::getDoubleData(const std::string& key) const {
    auto it = double_data_.find(key);
    return (it != double_data_.end()) ? it->second : 0.0;
}

int PluginResultImpl::getIntData(const std::string& key) const {
    auto it = int_data_.find(key);
    return (it != int_data_.end()) ? it->second : 0;
}

bool PluginResultImpl::hasData(const std::string& key) const {
    return string_data_.find(key) != string_data_.end() ||
           double_data_.find(key) != double_data_.end() ||
           int_data_.find(key) != int_data_.end();
}

std::string PluginResultImpl::serialize() const {
    std::ostringstream oss;
    oss << std::fixed << std::setprecision(6);
    
    oss << "{";
    bool first = true;
    
    // 序列化字符串数据
    for (const auto& [key, value] : string_data_) {
        if (!first) oss << ",";
        oss << "\"" << key << "\":\"" << value << "\"";
        first = false;
    }
    
    // 序列化双精度数据
    for (const auto& [key, value] : double_data_) {
        if (!first) oss << ",";
        oss << "\"" << key << "\":" << value;
        first = false;
    }
    
    // 序列化整数数据
    for (const auto& [key, value] : int_data_) {
        if (!first) oss << ",";
        oss << "\"" << key << "\":" << value;
        first = false;
    }
    
    oss << "}";
    return oss.str();
}

bool PluginResultImpl::deserialize(const std::string& data) {
    try {
        // 简化的JSON解析
        std::istringstream iss(data);
        std::string token;
        
        while (std::getline(iss, token, ',')) {
            size_t colonPos = token.find(':');
            if (colonPos != std::string::npos) {
                std::string key = token.substr(0, colonPos);
                std::string value = token.substr(colonPos + 1);
                
                // 清理引号
                key.erase(std::remove(key.begin(), key.end(), '"'), key.end());
                key.erase(std::remove(key.begin(), key.end(), ' '), key.end());
                
                if (!key.empty() && !value.empty()) {
                    // 判断数据类型
                    if (value.front() == '"' && value.back() == '"') {
                        // 字符串类型
                        value = value.substr(1, value.length() - 2);
                        string_data_[key] = value;
                    } else if (value.find('.') != std::string::npos) {
                        // 双精度类型
                        double_data_[key] = std::stod(value);
                    } else {
                        // 整数类型
                        int_data_[key] = std::stoi(value);
                    }
                }
            }
        }
        
        return true;
    } catch (const std::exception& e) {
        return false;
    }
}

// PluginParameterImpl实现
PluginParameterImpl::PluginParameterImpl() = default;

std::string PluginParameterImpl::getString(const std::string& key, const std::string& defaultValue) const {
    auto it = string_params_.find(key);
    return (it != string_params_.end()) ? it->second : defaultValue;
}

double PluginParameterImpl::getDouble(const std::string& key, double defaultValue) const {
    auto it = double_params_.find(key);
    return (it != double_params_.end()) ? it->second : defaultValue;
}

int PluginParameterImpl::getInt(const std::string& key, int defaultValue) const {
    auto it = int_params_.find(key);
    return (it != int_params_.end()) ? it->second : defaultValue;
}

bool PluginParameterImpl::getBool(const std::string& key, bool defaultValue) const {
    auto it = bool_params_.find(key);
    return (it != bool_params_.end()) ? it->second : defaultValue;
}

std::vector<double> PluginParameterImpl::getDoubleArray(const std::string& key) const {
    auto it = double_array_params_.find(key);
    return (it != double_array_params_.end()) ? it->second : std::vector<double>();
}

std::vector<int> PluginParameterImpl::getIntArray(const std::string& key) const {
    auto it = int_array_params_.find(key);
    return (it != int_array_params_.end()) ? it->second : std::vector<int>();
}

void PluginParameterImpl::setString(const std::string& key, const std::string& value) {
    string_params_[key] = value;
}

void PluginParameterImpl::setDouble(const std::string& key, double value) {
    double_params_[key] = value;
}

void PluginParameterImpl::setInt(const std::string& key, int value) {
    int_params_[key] = value;
}

void PluginParameterImpl::setBool(const std::string& key, bool value) {
    bool_params_[key] = value;
}

void PluginParameterImpl::setDoubleArray(const std::string& key, const std::vector<double>& value) {
    double_array_params_[key] = value;
}

void PluginParameterImpl::setIntArray(const std::string& key, const std::vector<int>& value) {
    int_array_params_[key] = value;
}

std::string PluginParameterImpl::serialize() const {
    std::ostringstream oss;
    oss << std::fixed << std::setprecision(6);
    
    oss << "{";
    bool first = true;
    
    // 序列化字符串参数
    for (const auto& [key, value] : string_params_) {
        if (!first) oss << ",";
        oss << "\"" << key << "\":\"" << value << "\"";
        first = false;
    }
    
    // 序列化双精度参数
    for (const auto& [key, value] : double_params_) {
        if (!first) oss << ",";
        oss << "\"" << key << "\":" << value;
        first = false;
    }
    
    // 序列化整数参数
    for (const auto& [key, value] : int_params_) {
        if (!first) oss << ",";
        oss << "\"" << key << "\":" << value;
        first = false;
    }
    
    // 序列化布尔参数
    for (const auto& [key, value] : bool_params_) {
        if (!first) oss << ",";
        oss << "\"" << key << "\":" << (value ? "true" : "false");
        first = false;
    }
    
    // 序列化数组参数
    for (const auto& [key, value] : double_array_params_) {
        if (!first) oss << ",";
        oss << "\"" << key << "\":[";
        for (size_t i = 0; i < value.size(); ++i) {
            if (i > 0) oss << ",";
            oss << value[i];
        }
        oss << "]";
        first = false;
    }
    
    for (const auto& [key, value] : int_array_params_) {
        if (!first) oss << ",";
        oss << "\"" << key << "\":[";
        for (size_t i = 0; i < value.size(); ++i) {
            if (i > 0) oss << ",";
            oss << value[i];
        }
        oss << "]";
        first = false;
    }
    
    oss << "}";
    return oss.str();
}

bool PluginParameterImpl::deserialize(const std::string& data) {
    try {
        // 简化的JSON解析实现
        // 生产环境中应使用专业的JSON库
        std::istringstream iss(data);
        std::string token;
        
        while (std::getline(iss, token, ',')) {
            size_t colonPos = token.find(':');
            if (colonPos != std::string::npos) {
                std::string key = token.substr(0, colonPos);
                std::string value = token.substr(colonPos + 1);
                
                // 清理引号
                key.erase(std::remove(key.begin(), key.end(), '"'), key.end());
                key.erase(std::remove(key.begin(), key.end(), ' '), key.end());
                
                if (!key.empty() && !value.empty()) {
                    // 判断数据类型并解析
                    if (value.front() == '"' && value.back() == '"') {
                        // 字符串类型
                        value = value.substr(1, value.length() - 2);
                        string_params_[key] = value;
                    } else if (value == "true" || value == "false") {
                        // 布尔类型
                        bool_params_[key] = (value == "true");
                    } else if (value.front() == '[' && value.back() == ']') {
                        // 数组类型
                        std::string arrayStr = value.substr(1, value.length() - 2);
                        std::istringstream arrayIss(arrayStr);
                        std::string arrayToken;
                        std::vector<double> doubleArray;
                        std::vector<int> intArray;
                        
                        while (std::getline(arrayIss, arrayToken, ',')) {
                            if (!arrayToken.empty()) {
                                if (arrayToken.find('.') != std::string::npos) {
                                    doubleArray.push_back(std::stod(arrayToken));
                                } else {
                                    intArray.push_back(std::stoi(arrayToken));
                                }
                            }
                        }
                        
                        if (!doubleArray.empty()) {
                            double_array_params_[key] = doubleArray;
                        } else if (!intArray.empty()) {
                            int_array_params_[key] = intArray;
                        }
                    } else if (value.find('.') != std::string::npos) {
                        // 双精度类型
                        double_params_[key] = std::stod(value);
                    } else {
                        // 整数类型
                        int_params_[key] = std::stoi(value);
                    }
                }
            }
        }
        
        return true;
    } catch (const std::exception& e) {
        return false;
    }
}

// PluginResultImpl数组方法实现
std::vector<std::string> PluginResultImpl::getStringArray(const std::string& key) const {
    // 简化实现，返回单个字符串作为数组
    std::string value = getStringData(key);
    return value.empty() ? std::vector<std::string>() : std::vector<std::string>{value};
}

std::vector<double> PluginResultImpl::getDoubleArray(const std::string& key) const {
    // 简化实现，返回单个值作为数组
    double value = getDoubleData(key);
    return (value == 0.0) ? std::vector<double>() : std::vector<double>{value};
}

std::vector<int> PluginResultImpl::getIntArray(const std::string& key) const {
    // 简化实现，返回单个值作为数组
    int value = getIntData(key);
    return (value == 0) ? std::vector<int>() : std::vector<int>{value};
}

} // namespace AlgorithmPlugins
