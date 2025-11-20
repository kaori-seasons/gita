#include "json_parser.h"
#include <sstream>
#include <cctype>
#include <algorithm>

std::map<std::string, std::string> SimpleJsonParser::parse(const std::string& json_str) {
    std::map<std::string, std::string> result;
    
    size_t pos = 0;
    pos = skip_whitespace(json_str, pos);
    
    // 跳过开头的 '{'
    if (pos < json_str.length() && json_str[pos] == '{') {
        pos++;
    }
    
    while (pos < json_str.length()) {
        pos = skip_whitespace(json_str, pos);
        
        // 检查是否到达结尾
        if (pos >= json_str.length() || json_str[pos] == '}') {
            break;
        }
        
        // 解析键
        if (json_str[pos] == '"') {
            std::string key = parse_string_value(json_str, pos);
            
            // 跳过 ':'
            pos = skip_whitespace(json_str, pos);
            if (pos < json_str.length() && json_str[pos] == ':') {
                pos++;
            }
            
            pos = skip_whitespace(json_str, pos);
            
            // 解析值
            std::string value;
            if (pos < json_str.length()) {
                if (json_str[pos] == '"') {
                    // 字符串值
                    value = parse_string_value(json_str, pos);
                } else if (std::isdigit(json_str[pos]) || json_str[pos] == '-' || json_str[pos] == '.') {
                    // 数字值
                    size_t start = pos;
                    while (pos < json_str.length() && 
                           (std::isdigit(json_str[pos]) || json_str[pos] == '.' || 
                            json_str[pos] == '-' || json_str[pos] == '+' || 
                            json_str[pos] == 'e' || json_str[pos] == 'E')) {
                        pos++;
                    }
                    value = json_str.substr(start, pos - start);
                } else if (json_str.substr(pos, 4) == "true") {
                    value = "true";
                    pos += 4;
                } else if (json_str.substr(pos, 5) == "false") {
                    value = "false";
                    pos += 5;
                } else if (json_str.substr(pos, 4) == "null") {
                    value = "null";
                    pos += 4;
                }
            }
            
            result[key] = value;
            
            // 跳过 ','
            pos = skip_whitespace(json_str, pos);
            if (pos < json_str.length() && json_str[pos] == ',') {
                pos++;
            }
        } else {
            pos++;
        }
    }
    
    return result;
}

std::string SimpleJsonParser::get_string(const std::string& json_str, const std::string& key) {
    auto data = parse(json_str);
    auto it = data.find(key);
    if (it != data.end()) {
        return it->second;
    }
    return "";
}

double SimpleJsonParser::get_double(const std::string& json_str, const std::string& key, double default_value) {
    std::string value_str = get_string(json_str, key);
    if (value_str.empty()) {
        return default_value;
    }
    
    try {
        return std::stod(value_str);
    } catch (...) {
        return default_value;
    }
}

int SimpleJsonParser::get_int(const std::string& json_str, const std::string& key, int default_value) {
    std::string value_str = get_string(json_str, key);
    if (value_str.empty()) {
        return default_value;
    }
    
    try {
        return std::stoi(value_str);
    } catch (...) {
        return default_value;
    }
}

std::string SimpleJsonParser::to_json(const std::map<std::string, std::string>& data) {
    std::ostringstream oss;
    oss << "{";
    
    bool first = true;
    for (const auto& pair : data) {
        if (!first) {
            oss << ",";
        }
        oss << "\"" << pair.first << "\":\"" << pair.second << "\"";
        first = false;
    }
    
    oss << "}";
    return oss.str();
}

std::string SimpleJsonParser::to_json_object(const std::map<std::string, double>& data) {
    std::ostringstream oss;
    oss << "{";
    
    bool first = true;
    for (const auto& pair : data) {
        if (!first) {
            oss << ",";
        }
        oss << "\"" << pair.first << "\":" << pair.second;
        first = false;
    }
    
    oss << "}";
    return oss.str();
}

size_t SimpleJsonParser::skip_whitespace(const std::string& str, size_t pos) {
    while (pos < str.length() && 
           (str[pos] == ' ' || str[pos] == '\t' || str[pos] == '\n' || str[pos] == '\r')) {
        pos++;
    }
    return pos;
}

std::string SimpleJsonParser::parse_string_value(const std::string& json_str, size_t& pos) {
    if (pos >= json_str.length() || json_str[pos] != '"') {
        return "";
    }
    
    pos++; // 跳过开头的 '"'
    size_t start = pos;
    
    // 查找结尾的 '"'（考虑转义）
    while (pos < json_str.length()) {
        if (json_str[pos] == '"' && (pos == start || json_str[pos - 1] != '\\')) {
            break;
        }
        pos++;
    }
    
    std::string result = json_str.substr(start, pos - start);
    pos++; // 跳过结尾的 '"'
    
    return result;
}

double SimpleJsonParser::parse_number_value(const std::string& json_str, size_t& pos) {
    size_t start = pos;
    while (pos < json_str.length() && 
           (std::isdigit(json_str[pos]) || json_str[pos] == '.' || 
            json_str[pos] == '-' || json_str[pos] == '+' || 
            json_str[pos] == 'e' || json_str[pos] == 'E')) {
        pos++;
    }
    
    try {
        return std::stod(json_str.substr(start, pos - start));
    } catch (...) {
        return 0.0;
    }
}


