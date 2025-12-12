#include "json_parser.h"
#include <sstream>
#include <iostream>

// 简单的 JSON 字段提取
std::string parse_json_field(const std::string& json, const std::string& field) {
    // 这是一个简化版本，只处理简单的 JSON
    std::string search_key = "\"" + field + "\":";
    size_t pos = json.find(search_key);
    if (pos == std::string::npos) {
        return "";
    }
    
    pos += search_key.length();
    
    // 跳过空格
    while (pos < json.length() && (json[pos] == ' ' || json[pos] == '\t')) {
        pos++;
    }
    
    // 提取值（假设是字符串或数字）
    if (json[pos] == '"') {
        // 字符串值
        pos++;
        std::string value;
        while (pos < json.length() && json[pos] != '"') {
            value += json[pos];
            pos++;
        }
        return value;
    } else {
        // 数字或其他值
        std::string value;
        while (pos < json.length() && json[pos] != ',' && json[pos] != '}') {
            value += json[pos];
            pos++;
        }
        return value;
    }
}
