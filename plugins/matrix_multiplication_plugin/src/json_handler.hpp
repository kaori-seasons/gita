// JSON处理器头文件
// 处理输入输出JSON文件的读写和验证

#ifndef JSON_HANDLER_HPP
#define JSON_HANDLER_HPP

#include <string>
#include <vector>
#include <json/json.h>
#include "matrix_multiplication.hpp"

class JsonHandler {
public:
    JsonHandler();
    ~JsonHandler() = default;

    // 读取JSON文件
    bool readJsonFile(const std::string& file_path, Json::Value& root);

    // 写入JSON文件
    bool writeJsonFile(const std::string& file_path, const Json::Value& root);

    // 验证输入参数
    bool validateInput(const Json::Value& input);

    // 解析矩阵参数
    bool parseMatrices(const Json::Value& input,
                      MatrixMultiplication::Matrix& A,
                      MatrixMultiplication::Matrix& B);

    // 创建错误响应
    Json::Value createErrorResponse(const std::string& error_message,
                                   const std::string& error_code = "UNKNOWN_ERROR");

    // 创建成功响应
    Json::Value createSuccessResponse(const MatrixMultiplication::Matrix& result,
                                     const std::string& algorithm_name,
                                     long long computation_time_ms);

private:
    // 验证矩阵格式
    bool validateMatrix(const Json::Value& matrix_json, const std::string& matrix_name);

    // 解析单个矩阵
    bool parseMatrix(const Json::Value& matrix_json, MatrixMultiplication::Matrix& matrix);

    // 验证数值范围
    bool validateNumericValue(double value, const std::string& field_name);

    // 获取矩阵维度信息
    std::pair<size_t, size_t> getMatrixDimensions(const MatrixMultiplication::Matrix& matrix);

    // JSON读取器和写入器
    Json::Reader reader_;
    Json::StyledWriter writer_;
    Json::FastWriter fast_writer_;
};

#endif // JSON_HANDLER_HPP
