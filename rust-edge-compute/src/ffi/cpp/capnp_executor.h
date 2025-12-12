// C++ Example: Using Cap'n Proto for Zero-Copy Algorithm Execution
//
// This example demonstrates how to:
// 1. Receive Cap'n Proto binary messages from Rust
// 2. Access data in-place without deserialization
// 3. Process data efficiently
// 4. Return results in Cap'n Proto format

#pragma once

#include "algorithm.capnp.h"
#include <vector>
#include <cmath>
#include <memory>
#include <unordered_map>
#include <string>
#include <cstdint>
#include <stdexcept>
#include <cstdio>
#include <ctime>

namespace algorithm {
namespace cpp {

/// Zero-copy vibration data processor using Cap'n Proto
class CapnProtoAlgorithmExecutor {
public:
    /// Constructor - initializes plugin registry
    CapnProtoAlgorithmExecutor() {
        registerBuiltInPlugins();
    }

    /// Destructor
    ~CapnProtoAlgorithmExecutor() = default;
    /// Execute algorithm on Cap'n Proto binary message
    /// @param input_bytes: Binary Cap'n Proto message containing AlgorithmInput
    /// @return Binary Cap'n Proto message containing AlgorithmOutput
    std::vector<uint8_t> executeAlgorithmCapnProto(
        const uint8_t* input_bytes,
        size_t input_size);

    /// Process vibration data with zero-copy access
    /// @param vib_bytes: Binary Cap'n Proto message containing VibrationData
    /// @return Binary Cap'n Proto message containing VibrationFeatures
    std::vector<uint8_t> processVibrationDataCapnProto(
        const uint8_t* vib_bytes,
        size_t vib_size);

    // ========================================================================
    // Plugin Algorithm Functions
    // ========================================================================

    /// Execute vibrate31 plugin - Vibration analysis algorithm
    /// @param vib_data_bytes: VibrationData message
    /// @param params_json: Algorithm parameters in JSON format
    /// @return VibrationAnalysisResponse message
    std::vector<uint8_t> executeVibrate31Plugin(
        const uint8_t* vib_data_bytes,
        size_t vib_data_size,
        const char* params_json);

    /// Execute generic algorithm plugin with dynamic parameters
    /// @param algorithm_name: Name of the algorithm to execute
    /// @param input_bytes: Serialized AlgorithmInput message
    /// @return Serialized AlgorithmOutput message
    std::vector<uint8_t> executePlugin(
        const char* algorithm_name,
        const uint8_t* input_bytes,
        size_t input_size);

    /// Load and initialize a plugin
    /// @param plugin_name: Name of plugin to load
    /// @return true if successful, false otherwise
    bool loadPlugin(const char* plugin_name);

    /// Unload a plugin and free resources
    /// @param plugin_name: Name of plugin to unload
    /// @return true if successful, false otherwise
    bool unloadPlugin(const char* plugin_name);

    /// Get list of available plugins
    /// @return Vector of plugin names
    std::vector<std::string> getAvailablePlugins() const;

    /// Get information about a specific plugin
    /// @param plugin_name: Name of the plugin
    /// @return Plugin information as JSON string
    std::string getPluginInfo(const char* plugin_name) const;

    /// Get system and plugin status
    /// @return Binary Cap'n Proto message containing SystemStatus
    std::vector<uint8_t> getSystemStatus() const;

    /// Perform health check on all loaded plugins
    /// @return true if all plugins are healthy, false otherwise
    bool healthCheck() const;

private:
    /// Extract and analyze vibration features (zero-copy)
    /// @param reader: Cap'n Proto reader for VibrationData
    /// @return Computed features
    VibrationFeatures analyzeVibration(
        const VibrationData::Reader& reader);

    /// Plugin registry for tracking loaded plugins
    struct PluginEntry {
        std::string name;
        std::string version;
        bool initialized;
        uint64_t execution_count;
        uint64_t failure_count;
    };

    std::unordered_map<std::string, PluginEntry> loaded_plugins_;

    /// Helper function to dispatch algorithm execution based on name
    std::vector<uint8_t> dispatchAlgorithmExecution(
        const std::string& algorithm_name,
        const AlgorithmInput::Reader& input_reader);

    /// Vibrate31-specific implementation with zero-copy processing
    VibrationFeatures executeVibrate31Internal(
        const VibrationData::Reader& vib_reader,
        const std::string& params_json);

    /// Register built-in plugins (called during construction)
    void registerBuiltInPlugins();
    /// @param wave_data: Waveform data (reader access)
    /// @param sampling_rate: Sampling frequency in Hz
    /// @return Frequency domain features (peak frequency, power, etc.)
    struct FreqFeatures {
        double peak_freq;
        double peak_power;
        double spectrum_energy;
    };

    FreqFeatures computeFrequencyFeatures(
        const capnp::List<double>::Reader& wave_data,
        int32_t sampling_rate);

    /// Compute time-domain statistical features
    /// @param wave_data: Waveform data
    /// @return Statistical properties
    struct TimeFeatures {
        double mean;
        double std_dev;
    };

    TimeFeatures computeTimeFeatures(
        const capnp::List<double>::Reader& wave_data);

    /// Separate high and low frequency components
    struct FreqBands {
        double mean_hf;  // High frequency (5-10kHz)
        double mean_lf;  // Low frequency (0-2kHz)
    };

    FreqBands separateFrequencyBands(
        const capnp::List<double>::Reader& wave_data,
        int32_t sampling_rate);
};

// ============================================================================
// Implementation Details
// ============================================================================

inline std::vector<uint8_t> CapnProtoAlgorithmExecutor::executeAlgorithmCapnProto(
    const uint8_t* input_bytes,
    size_t input_size) {
    
    try {
        // Reconstruct message from bytes (zero-copy read)
        auto words = capnp::ArrayPtr<const capnp::word>(
            reinterpret_cast<const capnp::word*>(input_bytes),
            input_size / sizeof(capnp::word));
        
        capnp::SegmentArrayMessageReader message_reader(
            capnp::ArrayPtr<const capnp::word*>(&words, 1));
        
        auto input_reader = message_reader.getRoot<AlgorithmInput>();

        // Access fields in-place (no copying)
        auto algorithm_name = input_reader.getAlgorithmName();
        auto device_id = input_reader.getDeviceId();
        auto timestamp_ms = input_reader.getTimestampMs();
        auto request_id = input_reader.getRequestId();

        // Build response message
        capnp::MallocMessageBuilder response_builder;
        auto output = response_builder.initRoot<AlgorithmOutput>();
        
        // Set success and populate fields
        output.setSuccess(true);
        output.setResultJson(R"({"status":"processed"})");
        output.setErrorMessage("");
        output.setExecutionTimeMs(1);  // Placeholder
        output.setMemoryUsedBytes(0);
        output.setRequestId(request_id);

        // Serialize response to bytes
        std::vector<uint8_t> response_bytes;
        auto response_words = capnp::messageToWordArray(response_builder);
        auto response_data = capnp::ArrayPtr<const uint8_t>(
            reinterpret_cast<const uint8_t*>(response_words.begin()),
            response_words.size() * sizeof(capnp::word));
        
        response_bytes.assign(response_data.begin(), response_data.end());
        return response_bytes;

    } catch (const std::exception& e) {
        // Handle error and return error response
        capnp::MallocMessageBuilder error_builder;
        auto error_output = error_builder.initRoot<AlgorithmOutput>();
        error_output.setSuccess(false);
        error_output.setErrorMessage(std::string("Error: ") + e.what());
        
        std::vector<uint8_t> error_bytes;
        auto error_words = capnp::messageToWordArray(error_builder);
        auto error_data = capnp::ArrayPtr<const uint8_t>(
            reinterpret_cast<const uint8_t*>(error_words.begin()),
            error_words.size() * sizeof(capnp::word));
        
        error_bytes.assign(error_data.begin(), error_data.end());
        return error_bytes;
    }
}

inline std::vector<uint8_t> CapnProtoAlgorithmExecutor::processVibrationDataCapnProto(
    const uint8_t* vib_bytes,
    size_t vib_size) {
    
    try {
        // Reconstruct message from bytes (zero-copy read)
        auto words = capnp::ArrayPtr<const capnp::word>(
            reinterpret_cast<const capnp::word*>(vib_bytes),
            vib_size / sizeof(capnp::word));
        
        capnp::SegmentArrayMessageReader message_reader(
            capnp::ArrayPtr<const capnp::word*>(&words, 1));
        
        auto vib_reader = message_reader.getRoot<VibrationData>();

        // Analyze vibration data (reads happen in-place, no copying)
        auto features = analyzeVibration(vib_reader);

        // Build response
        capnp::MallocMessageBuilder response_builder;
        auto response = response_builder.initRoot<VibrationAnalysisResponse>();
        
        // Copy computed features (this is the only copy)
        auto features_builder = response.initFeatures();
        features_builder.setMeanHf(features.mean_hf);
        features_builder.setMeanLf(features.mean_lf);
        features_builder.setMean(features.mean);
        features_builder.setStdDev(features.std_dev);
        features_builder.setPeakFreq(features.peak_freq);
        features_builder.setPeakPower(features.peak_power);
        features_builder.setSpectrumEnergy(features.spectrum_energy);
        features_builder.setStatus(features.status);
        features_builder.setLoad(features.load);
        features_builder.setTimestamp(features.timestamp);
        
        response.setSuccess(true);
        response.setExecutionTimeMs(0);  // Measure in practice

        // Serialize to bytes
        std::vector<uint8_t> response_bytes;
        auto response_words = capnp::messageToWordArray(response_builder);
        auto response_data = capnp::ArrayPtr<const uint8_t>(
            reinterpret_cast<const uint8_t*>(response_words.begin()),
            response_words.size() * sizeof(capnp::word));
        
        response_bytes.assign(response_data.begin(), response_data.end());
        return response_bytes;

    } catch (const std::exception& e) {
        // Error handling omitted for brevity
        std::vector<uint8_t> empty;
        return empty;
    }
}

inline CapnProtoAlgorithmExecutor::FreqFeatures 
CapnProtoAlgorithmExecutor::computeFrequencyFeatures(
    const capnp::List<double>::Reader& wave_data,
    int32_t sampling_rate) {
    
    FreqFeatures freq;
    
    // Simple peak detection (for demonstration)
    double max_val = 0.0;
    size_t max_idx = 0;
    
    for (uint32_t i = 0; i < wave_data.size(); ++i) {
        if (wave_data[i] > max_val) {
            max_val = wave_data[i];
            max_idx = i;
        }
    }
    
    // Compute approximate frequency
    freq.peak_freq = (static_cast<double>(max_idx) * sampling_rate) / wave_data.size();
    freq.peak_power = max_val;
    
    // Estimate spectrum energy (sum of squares)
    double energy = 0.0;
    for (uint32_t i = 0; i < wave_data.size(); ++i) {
        energy += wave_data[i] * wave_data[i];
    }
    freq.spectrum_energy = energy;
    
    return freq;
}

inline CapnProtoAlgorithmExecutor::TimeFeatures 
CapnProtoAlgorithmExecutor::computeTimeFeatures(
    const capnp::List<double>::Reader& wave_data) {
    
    TimeFeatures time;
    
    // Compute mean
    double sum = 0.0;
    for (uint32_t i = 0; i < wave_data.size(); ++i) {
        sum += wave_data[i];
    }
    time.mean = sum / wave_data.size();
    
    // Compute standard deviation
    double var = 0.0;
    for (uint32_t i = 0; i < wave_data.size(); ++i) {
        double diff = wave_data[i] - time.mean;
        var += diff * diff;
    }
    time.std_dev = std::sqrt(var / wave_data.size());
    
    return time;
}

inline VibrationFeatures CapnProtoAlgorithmExecutor::analyzeVibration(
    const VibrationData::Reader& reader) {
    
    VibrationFeatures features;
    
    // Access data in-place (no copying!)
    auto wave_data = reader.getWaveData();
    auto speed_data = reader.getSpeedData();
    int32_t sampling_rate = reader.getSamplingRate();
    
    // Compute frequency features
    auto freq_feat = computeFrequencyFeatures(wave_data, sampling_rate);
    features.peak_freq = freq_feat.peak_freq;
    features.peak_power = freq_feat.peak_power;
    features.spectrum_energy = freq_feat.spectrum_energy;
    
    // Compute time-domain features
    auto time_feat = computeTimeFeatures(wave_data);
    features.mean = time_feat.mean;
    features.std_dev = time_feat.std_dev;
    
    // Separate frequency bands
    auto freq_bands = separateFrequencyBands(wave_data, sampling_rate);
    features.mean_hf = freq_bands.mean_hf;
    features.mean_lf = freq_bands.mean_lf;
    
    // Set metadata
    features.status = 0;  // Healthy
    features.load = 50.0;  // 50% load
    features.timestamp = reader.getTimestamp();
    
    return features;
}

inline CapnProtoAlgorithmExecutor::FreqBands 
CapnProtoAlgorithmExecutor::separateFrequencyBands(
    const capnp::List<double>::Reader& wave_data,
    int32_t sampling_rate) {
    
    FreqBands bands;
    bands.mean_hf = 0.0;
    bands.mean_lf = 0.0;
    
    // Simplified separation (in production, use proper FFT)
    // HF: 5-10kHz, LF: 0-2kHz
    uint32_t hf_count = 0, lf_count = 0;
    
    for (uint32_t i = 0; i < wave_data.size() / 2; ++i) {
        double freq = (static_cast<double>(i) * sampling_rate) / wave_data.size();
        
        if (freq >= 5000 && freq <= 10000) {
            bands.mean_hf += std::abs(wave_data[i]);
            hf_count++;
        } else if (freq >= 0 && freq <= 2000) {
            bands.mean_lf += std::abs(wave_data[i]);
            lf_count++;
        }
    }
    
    if (hf_count > 0) bands.mean_hf /= hf_count;
    if (lf_count > 0) bands.mean_lf /= lf_count;
    
    return bands;
}

// ============================================================================
// Plugin Algorithm Implementations
// ============================================================================

inline bool CapnProtoAlgorithmExecutor::loadPlugin(const char* plugin_name) {
    if (!plugin_name) return false;
    
    std::string name(plugin_name);
    
    // Check if already loaded
    if (loaded_plugins_.find(name) != loaded_plugins_.end()) {
        return true;  // Already loaded
    }
    
    // Register new plugin
    PluginEntry entry;
    entry.name = name;
    entry.version = "1.0.0";  // Default version
    entry.initialized = true;
    entry.execution_count = 0;
    entry.failure_count = 0;
    
    loaded_plugins_[name] = entry;
    return true;
}

inline bool CapnProtoAlgorithmExecutor::unloadPlugin(const char* plugin_name) {
    if (!plugin_name) return false;
    
    std::string name(plugin_name);
    auto it = loaded_plugins_.find(name);
    
    if (it != loaded_plugins_.end()) {
        loaded_plugins_.erase(it);
        return true;
    }
    
    return false;  // Plugin not found
}

inline std::vector<std::string> CapnProtoAlgorithmExecutor::getAvailablePlugins() const {
    std::vector<std::string> plugins;
    
    for (const auto& pair : loaded_plugins_) {
        plugins.push_back(pair.first);
    }
    
    return plugins;
}

inline std::string CapnProtoAlgorithmExecutor::getPluginInfo(const char* plugin_name) const {
    if (!plugin_name) return "{}";
    
    std::string name(plugin_name);
    auto it = loaded_plugins_.find(name);
    
    if (it == loaded_plugins_.end()) {
        return "{\"error\":\"plugin_not_found\"}";
    }
    
    const auto& entry = it->second;
    
    // Return plugin info as JSON
    char buffer[512];
    snprintf(buffer, sizeof(buffer),
        "{\"name\":\"%s\",\"version\":\"%s\","
        "\"initialized\":%s,\"execution_count\":%lu,\"failure_count\":%lu}",
        entry.name.c_str(),
        entry.version.c_str(),
        entry.initialized ? "true" : "false",
        entry.execution_count,
        entry.failure_count);
    
    return std::string(buffer);
}

inline std::vector<uint8_t> CapnProtoAlgorithmExecutor::getSystemStatus() const {
    capnp::MallocMessageBuilder builder;
    auto status = builder.initRoot<SystemStatus>();
    
    // Set memory stats (placeholder)
    status.setTotalMemoryBytes(1024 * 1024 * 1024);  // 1GB
    status.setUsedMemoryBytes(512 * 1024 * 1024);    // 512MB
    
    // Set plugin stats
    status.setActivePlugins(loaded_plugins_.size());
    status.setTotalPlugins(loaded_plugins_.size());
    
    // Set system health
    status.setSystemHealth("healthy");
    
    // Set task stats
    status.setActiveTaskCount(0);
    status.setCompletedTaskCount(0);
    status.setFailedTaskCount(0);
    
    status.setAverageExecutionTimeMs(0.0);
    status.setCpuUsagePercent(0.0);
    status.setTimestamp(std::time(nullptr) * 1000);
    
    // Serialize
    std::vector<uint8_t> response_bytes;
    auto response_words = capnp::messageToWordArray(builder);
    auto response_data = capnp::ArrayPtr<const uint8_t>(
        reinterpret_cast<const uint8_t*>(response_words.begin()),
        response_words.size() * sizeof(capnp::word));
    
    response_bytes.assign(response_data.begin(), response_data.end());
    return response_bytes;
}

inline bool CapnProtoAlgorithmExecutor::healthCheck() const {
    // Check if all loaded plugins are healthy
    for (const auto& pair : loaded_plugins_) {
        if (!pair.second.initialized) {
            return false;
        }
    }
    return true;
}

inline std::vector<uint8_t> CapnProtoAlgorithmExecutor::executeVibrate31Plugin(
    const uint8_t* vib_data_bytes,
    size_t vib_data_size,
    const char* params_json) {
    
    try {
        // Reconstruct vibration data message (zero-copy read)
        auto words = capnp::ArrayPtr<const capnp::word>(
            reinterpret_cast<const capnp::word*>(vib_data_bytes),
            vib_data_size / sizeof(capnp::word));
        
        capnp::SegmentArrayMessageReader message_reader(
            capnp::ArrayPtr<const capnp::word*>(&words, 1));
        
        auto vib_reader = message_reader.getRoot<VibrationData>();
        
        // Execute vibrate31 analysis (zero-copy)
        std::string params(params_json ? params_json : "{}");
        auto features = executeVibrate31Internal(vib_reader, params);
        
        // Build response
        capnp::MallocMessageBuilder response_builder;
        auto response = response_builder.initRoot<VibrationAnalysisResponse>();
        
        // Populate features
        auto features_builder = response.initFeatures();
        features_builder.setMeanHf(features.mean_hf);
        features_builder.setMeanLf(features.mean_lf);
        features_builder.setMean(features.mean);
        features_builder.setStdDev(features.std_dev);
        features_builder.setPeakFreq(features.peak_freq);
        features_builder.setPeakPower(features.peak_power);
        features_builder.setSpectrumEnergy(features.spectrum_energy);
        features_builder.setStatus(features.status);
        features_builder.setLoad(features.load);
        features_builder.setTimestamp(features.timestamp);
        
        response.setSuccess(true);
        response.setExecutionTimeMs(0);  // Measure in production
        
        // Serialize
        std::vector<uint8_t> response_bytes;
        auto response_words = capnp::messageToWordArray(response_builder);
        auto response_data = capnp::ArrayPtr<const uint8_t>(
            reinterpret_cast<const uint8_t*>(response_words.begin()),
            response_words.size() * sizeof(capnp::word));
        
        response_bytes.assign(response_data.begin(), response_data.end());
        
        // Update plugin stats
        if (loaded_plugins_.find("vibrate31") != loaded_plugins_.end()) {
            loaded_plugins_["vibrate31"].execution_count++;
        }
        
        return response_bytes;
        
    } catch (const std::exception& e) {
        // Error response
        if (loaded_plugins_.find("vibrate31") != loaded_plugins_.end()) {
            loaded_plugins_["vibrate31"].failure_count++;
        }
        
        capnp::MallocMessageBuilder error_builder;
        auto error_response = error_builder.initRoot<VibrationAnalysisResponse>();
        error_response.setSuccess(false);
        error_response.setErrorMessage(std::string("Error: ") + e.what());
        
        std::vector<uint8_t> error_bytes;
        auto error_words = capnp::messageToWordArray(error_builder);
        auto error_data = capnp::ArrayPtr<const uint8_t>(
            reinterpret_cast<const uint8_t*>(error_words.begin()),
            error_words.size() * sizeof(capnp::word));
        
        error_bytes.assign(error_data.begin(), error_data.end());
        return error_bytes;
    }
}

inline std::vector<uint8_t> CapnProtoAlgorithmExecutor::executePlugin(
    const char* algorithm_name,
    const uint8_t* input_bytes,
    size_t input_size) {
    
    try {
        if (!algorithm_name) {
            throw std::invalid_argument("Algorithm name is null");
        }
        
        // Reconstruct input message
        auto words = capnp::ArrayPtr<const capnp::word>(
            reinterpret_cast<const capnp::word*>(input_bytes),
            input_size / sizeof(capnp::word));
        
        capnp::SegmentArrayMessageReader message_reader(
            capnp::ArrayPtr<const capnp::word*>(&words, 1));
        
        auto input_reader = message_reader.getRoot<AlgorithmInput>();
        
        // Dispatch to appropriate algorithm
        return dispatchAlgorithmExecution(std::string(algorithm_name), input_reader);
        
    } catch (const std::exception& e) {
        // Error response
        capnp::MallocMessageBuilder error_builder;
        auto output = error_builder.initRoot<AlgorithmOutput>();
        output.setSuccess(false);
        output.setErrorMessage(std::string("Error: ") + e.what());
        
        std::vector<uint8_t> error_bytes;
        auto error_words = capnp::messageToWordArray(error_builder);
        auto error_data = capnp::ArrayPtr<const uint8_t>(
            reinterpret_cast<const uint8_t*>(error_words.begin()),
            error_words.size() * sizeof(capnp::word));
        
        error_bytes.assign(error_data.begin(), error_data.end());
        return error_bytes;
    }
}

inline std::vector<uint8_t> CapnProtoAlgorithmExecutor::dispatchAlgorithmExecution(
    const std::string& algorithm_name,
    const AlgorithmInput::Reader& input_reader) {
    
    // Route to specific algorithm implementation
    if (algorithm_name == "vibrate31") {
        // Extract vibration data from input if available
        // For now, return a generic success response
        auto params = input_reader.getParametersJson();
        
        capnp::MallocMessageBuilder response_builder;
        auto output = response_builder.initRoot<AlgorithmOutput>();
        output.setSuccess(true);
        output.setResultJson(R"({"algorithm":"vibrate31","status":"processed"})");
        output.setExecutionTimeMs(0);
        output.setMemoryUsedBytes(0);
        output.setRequestId(input_reader.getRequestId());
        
        std::vector<uint8_t> response_bytes;
        auto response_words = capnp::messageToWordArray(response_builder);
        auto response_data = capnp::ArrayPtr<const uint8_t>(
            reinterpret_cast<const uint8_t*>(response_words.begin()),
            response_words.size() * sizeof(capnp::word));
        
        response_bytes.assign(response_data.begin(), response_data.end());
        return response_bytes;
    }
    
    // Unknown algorithm
    capnp::MallocMessageBuilder error_builder;
    auto output = error_builder.initRoot<AlgorithmOutput>();
    output.setSuccess(false);
    output.setErrorMessage(std::string("Unknown algorithm: ") + algorithm_name);
    output.setRequestId(input_reader.getRequestId());
    
    std::vector<uint8_t> error_bytes;
    auto error_words = capnp::messageToWordArray(error_builder);
    auto error_data = capnp::ArrayPtr<const uint8_t>(
        reinterpret_cast<const uint8_t*>(error_words.begin()),
        error_words.size() * sizeof(capnp::word));
    
    error_bytes.assign(error_data.begin(), error_data.end());
    return error_bytes;
}

inline VibrationFeatures CapnProtoAlgorithmExecutor::executeVibrate31Internal(
    const VibrationData::Reader& vib_reader,
    const std::string& params_json) {
    
    // Use existing vibration analysis (zero-copy)
    return analyzeVibration(vib_reader);
}

inline void CapnProtoAlgorithmExecutor::registerBuiltInPlugins() {
    // Register built-in plugins
    loadPlugin("vibrate31");    // Vibration analysis
    // Add more built-in plugins as needed
}

}  // namespace cpp
}  // namespace algorithm
