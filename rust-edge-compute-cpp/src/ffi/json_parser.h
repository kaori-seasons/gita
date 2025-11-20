#pragma once

#include <string>
#include <map>
#include <vector>

// 简单的JSON解析器（不依赖外部库）
class SimpleJsonParser {
public:
    // 解析JSON字符串，提取键值对
    static std::map<std::string, std::string> parse(const std::string& json_str);
    
    // 提取特定键的值（字符串）
    static std::string get_string(const std::string& json_str, const std::string& key);
    
    // 提取特定键的值（数字）
    static double get_double(const std::string& json_str, const std::string& key, double default_value = 0.0);
    
    // 提取特定键的值（整数）
    static int get_int(const std::string& json_str, const std::string& key, int default_value = 0);
    
    // 生成JSON字符串
    static std::string to_json(const std::map<std::string, std::string>& data);
    
    // 生成简单的JSON对象
    static std::string to_json_object(const std::map<std::string, double>& data);
    
private:
    // 辅助函数：跳过空白字符
    static size_t skip_whitespace(const std::string& str, size_t pos);
    
    // 辅助函数：解析字符串值
    static std::string parse_string_value(const std::string& json_str, size_t& pos);
    
    // 辅助函数：解析数字值
    static double parse_number_value(const std::string& json_str, size_t& pos);
};

