#include "algorithm_ffi.h"
#include <iostream>
#include <string>
#include <cstring>
#include <sstream>

// Global initialization flag
static bool g_initialized = false;

int algorithm_executor_init(void) {
    std::cout << "[FFI] Algorithm executor initialization called" << std::endl;
    g_initialized = true;
    return 0;
}

AlgorithmOutput* algorithm_executor_execute(const AlgorithmInput* input) {
    if (!g_initialized) {
        AlgorithmOutput* output = new AlgorithmOutput();
        output->success = false;
        output->result_json = strdup("{}");
        output->error_message = strdup("Executor not initialized");
        output->execution_time_ms = 0;
        output->memory_used_bytes = 0;
        return output;
    }

    std::cout << "[FFI] Executing algorithm: " << input->algorithm_name << std::endl;

    AlgorithmOutput* output = new AlgorithmOutput();
    output->success = true;
    
    // Build result JSON
    std::ostringstream oss;
    oss << "{"
        << "\"status\":\"success\","
        << "\"algorithm\":\"" << input->algorithm_name << "\","
        << "\"device_id\":\"" << input->device_id << "\","
        << "\"message\":\"Algorithm executed successfully\""
        << "}";
    
    output->result_json = strdup(oss.str().c_str());
    output->error_message = strdup("");
    output->execution_time_ms = 0;
    output->memory_used_bytes = 0;
    
    return output;
}

void algorithm_output_free(AlgorithmOutput* output) {
    if (output) {
        if (output->result_json) {
            free(output->result_json);
        }
        if (output->error_message) {
            free(output->error_message);
        }
        delete output;
    }
}

char* algorithm_get_available_plugins(void) {
    const char* plugins = "vibrate31,motor97,current_feature_extractor,"
                          "temperature_feature_extractor,audio_feature_extractor,"
                          "universal_classify1,comp_realtime_health34,"
                          "error18,score_alarm5,status_alarm4";
    return strdup(plugins);
}

void algorithm_free_string(char* ptr) {
    if (ptr) {
        free(ptr);
    }
}
