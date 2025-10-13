// JSON处理器实现文件
// 处理输入输出JSON文件的读写和验证

#include "json_handler.hpp"
#include <fstream>
#include <iostream>
#include <limits>
#include <algorithm>

JsonHandler::JsonHandler() : reader_(), writer_(), fast_writer_() {}

bool JsonHandler::readJsonFile(const std::string& file_path, Json::Value& root) {
    try {
        std::ifstream file(file_path);
        if (!file.is_open()) {
            std::cerr << "无法打开文件: " << file_path << std::endl;
            return false;
        }

        std::string content((std::istreambuf_iterator<char>(file)),
                           std::istreambuf_iterator<char>());

        if (content.empty()) {
            std::cerr << "文件为空: " << file_path << std::endl;
            return false;
        }

        bool parsing_successful = reader_.parse(content, root);
        if (!parsing_successful) {
            std::cerr << "JSON解析失败: " << reader_.getFormattedErrorMessages() << std::endl;
            return false;
        }

        return true;
    } catch (const std::exception& e) {
        std::cerr << "读取JSON文件时发生异常: " << e.what() << std::endl;
        return false;
    }
}

bool JsonHandler::writeJsonFile(const std::string& file_path, const Json::Value& root) {
    try {
        std::ofstream file(file_path);
        if (!file.is_open()) {
            std::cerr << "无法创建文件: " << file_path << std::endl;
            return false;
        }

        // 使用StyledWriter格式化输出，便于阅读
        std::string output = writer_.write(root);
        file << output;

        if (!file.good()) {
            std::cerr << "写入文件失败: " << file_path << std::endl;
            return false;
        }

        file.close();
        return true;
    } catch (const std::exception& e) {
        std::cerr << "写入JSON文件时发生异常: " << e.what() << std::endl;
        return false;
    }
}

bool JsonHandler::validateInput(const Json::Value& input) {
    try {
        // 检查必要的字段
        if (!input.isObject()) {
            std::cerr << "输入必须是JSON对象" << std::endl;
            return false;
        }

        // 检查操作类型
        if (!input.isMember("operation") || !input["operation"].isString()) {
            std::cerr << "缺少或无效的操作类型字段" << std::endl;
            return false;
        }

        std::string operation = input["operation"].asString();
        if (operation != "matrix_multiplication") {
            std::cerr << "不支持的操作类型: " << operation << std::endl;
            return false;
        }

        // 检查矩阵参数
        if (!validateMatrix(input["matrix_a"], "matrix_a")) {
            return false;
        }

        if (!validateMatrix(input["matrix_b"], "matrix_b")) {
            return false;
        }

        // 检查可选参数
        if (input.isMember("precision")) {
            std::string precision = input["precision"].asString();
            if (precision != "float" && precision != "double") {
                std::cerr << "不支持的精度类型: " << precision << std::endl;
                return false;
            }
        }

        if (input.isMember("optimization")) {
            std::string optimization = input["optimization"].asString();
            std::vector<std::string> valid_opts = {"none", "basic", "avx", "avx2", "avx512"};
            if (std::find(valid_opts.begin(), valid_opts.end(), optimization) == valid_opts.end()) {
                std::cerr << "不支持的优化选项: " << optimization << std::endl;
                return false;
            }
        }

        return true;
    } catch (const std::exception& e) {
        std::cerr << "验证输入参数时发生异常: " << e.what() << std::endl;
        return false;
    }
}

bool JsonHandler::validateMatrix(const Json::Value& matrix_json, const std::string& matrix_name) {
    if (!matrix_json.isArray()) {
        std::cerr << matrix_name << " 必须是数组" << std::endl;
        return false;
    }

    if (matrix_json.size() == 0) {
        std::cerr << matrix_name << " 不能为空" << std::endl;
        return false;
    }

    size_t row_length = 0;
    bool first_row = true;

    for (Json::ArrayIndex i = 0; i < matrix_json.size(); ++i) {
        const Json::Value& row = matrix_json[i];

        if (!row.isArray()) {
            std::cerr << matrix_name << " 的第 " << i << " 行必须是数组" << std::endl;
            return false;
        }

        if (first_row) {
            row_length = row.size();
            first_row = false;
            if (row_length == 0) {
                std::cerr << matrix_name << " 的行不能为空" << std::endl;
                return false;
            }
        } else if (row.size() != row_length) {
            std::cerr << matrix_name << " 的行长度不一致" << std::endl;
            return false;
        }

        // 验证每一行的元素
        for (Json::ArrayIndex j = 0; j < row.size(); ++j) {
            const Json::Value& element = row[j];
            if (!element.isNumeric()) {
                std::cerr << matrix_name << " 的元素 [" << i << "][" << j << "] 必须是数字" << std::endl;
                return false;
            }

            // 检查数值范围
            double value = element.asDouble();
            if (!validateNumericValue(value, matrix_name + "[" + std::to_string(i) + "][" + std::to_string(j) + "]")) {
                return false;
            }
        }
    }

    // 检查矩阵大小限制
    size_t max_size = 10000; // 最大10000x10000的矩阵
    if (matrix_json.size() > max_size || row_length > max_size) {
        std::cerr << matrix_name << " 矩阵太大 (最大 " << max_size << "x" << max_size << ")" << std::endl;
        return false;
    }

    return true;
}

bool JsonHandler::validateNumericValue(double value, const std::string& field_name) {
    // 检查是否为有限数值
    if (!std::isfinite(value)) {
        std::cerr << field_name << " 包含非有限数值" << std::endl;
        return false;
    }

    // 检查数值范围（避免溢出）
    double abs_value = std::abs(value);
    double max_value = 1e308; // 接近double的最大值

    if (abs_value > max_value) {
        std::cerr << field_name << " 数值过大: " << value << std::endl;
        return false;
    }

    return true;
}

bool JsonHandler::parseMatrices(const Json::Value& input,
                               MatrixMultiplication::Matrix& A,
                               MatrixMultiplication::Matrix& B) {
    try {
        if (!parseMatrix(input["matrix_a"], A)) {
            std::cerr << "解析矩阵A失败" << std::endl;
            return false;
        }

        if (!parseMatrix(input["matrix_b"], B)) {
            std::cerr << "解析矩阵B失败" << std::endl;
            return false;
        }

        // 验证矩阵乘法维度
        if (A[0].size() != B.size()) {
            std::cerr << "矩阵维度不匹配: A的列数(" << A[0].size()
                      << ") != B的行数(" << B.size() << ")" << std::endl;
            return false;
        }

        return true;
    } catch (const std::exception& e) {
        std::cerr << "解析矩阵参数时发生异常: " << e.what() << std::endl;
        return false;
    }
}

bool JsonHandler::parseMatrix(const Json::Value& matrix_json,
                             MatrixMultiplication::Matrix& matrix) {
    try {
        size_t rows = matrix_json.size();
        if (rows == 0) return false;

        size_t cols = matrix_json[0].size();
        matrix.resize(rows, MatrixMultiplication::MatrixRow(cols));

        for (Json::ArrayIndex i = 0; i < matrix_json.size(); ++i) {
            const Json::Value& row = matrix_json[i];
            for (Json::ArrayIndex j = 0; j < row.size(); ++j) {
                matrix[i][j] = row[j].asDouble();
            }
        }

        return true;
    } catch (const std::exception& e) {
        std::cerr << "解析矩阵时发生异常: " << e.what() << std::endl;
        return false;
    }
}

Json::Value JsonHandler::createErrorResponse(const std::string& error_message,
                                           const std::string& error_code) {
    Json::Value response;
    response["status"] = "error";
    response["error"] = error_message;
    response["error_code"] = error_code;
    response["timestamp"] = static_cast<Json::UInt64>(
        std::chrono::duration_cast<std::chrono::seconds>(
            std::chrono::system_clock::now().time_since_epoch()).count());
    return response;
}

Json::Value JsonHandler::createSuccessResponse(const MatrixMultiplication::Matrix& result,
                                             const std::string& algorithm_name,
                                             long long computation_time_ms) {
    Json::Value response;
    response["status"] = "success";
    response["algorithm"] = algorithm_name;
    response["computation_time_ms"] = static_cast<Json::UInt64>(computation_time_ms);

    // 添加结果矩阵
    Json::Value result_matrix(Json::arrayValue);
    for (const auto& row : result) {
        Json::Value json_row(Json::arrayValue);
        for (const auto& val : row) {
            json_row.append(val);
        }
        result_matrix.append(json_row);
    }
    response["result"] = result_matrix;

    // 添加元数据
    auto dims = getMatrixDimensions(result);
    response["metadata"]["result_rows"] = static_cast<Json::UInt64>(dims.first);
    response["metadata"]["result_cols"] = static_cast<Json::UInt64>(dims.second);
    response["metadata"]["timestamp"] = static_cast<Json::UInt64>(
        std::chrono::duration_cast<std::chrono::seconds>(
            std::chrono::system_clock::now().time_since_epoch()).count());

    return response;
}

std::pair<size_t, size_t> JsonHandler::getMatrixDimensions(const MatrixMultiplication::Matrix& matrix) {
    if (matrix.empty()) {
        return {0, 0};
    }
    return {matrix.size(), matrix[0].size()};
}
