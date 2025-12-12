#ifndef ALGORITHM_FFI_H
#define ALGORITHM_FFI_H

#ifdef __cplusplus
extern "C" {
#endif

#include <stdint.h>
#include <stdbool.h>

/// Algorithm execution input structure
typedef struct {
    const char* algorithm_name;
    const char* parameters_json;
    const char* device_id;
} AlgorithmInput;

/// Algorithm execution output structure
typedef struct {
    bool success;
    char* result_json;
    char* error_message;
    uint64_t execution_time_ms;
    uint64_t memory_used_bytes;
} AlgorithmOutput;

/// Initialize the algorithm executor
/// Returns: 0 on success, non-zero on failure
int algorithm_executor_init(void);

/// Execute an algorithm
/// Returns: dynamically allocated AlgorithmOutput
AlgorithmOutput* algorithm_executor_execute(const AlgorithmInput* input);

/// Free the output structure
void algorithm_output_free(AlgorithmOutput* output);

/// Get list of available plugins
/// Returns: comma-separated plugin names (must be freed by caller)
char* algorithm_get_available_plugins(void);

/// Free allocated string
void algorithm_free_string(char* ptr);

#ifdef __cplusplus
}
#endif

#endif // ALGORITHM_FFI_H
