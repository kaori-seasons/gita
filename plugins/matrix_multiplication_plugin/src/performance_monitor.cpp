// 性能监控器实现文件
// 监控算法执行的性能指标和资源使用情况

#include "performance_monitor.hpp"
#include <iostream>
#include <iomanip>
#include <algorithm>
#include <numeric>

PerformanceMonitor::PerformanceMonitor() : profiling_data_() {}

void PerformanceMonitor::startProfiling(const std::string& name) {
    auto it = profiling_data_.find(name);
    if (it == profiling_data_.end()) {
        profiling_data_[name] = std::make_unique<ProfilingData>(name);
        it = profiling_data_.find(name);
    }

    if (it->second->is_running) {
        std::cerr << "警告: 性能分析 '" << name << "' 已经在运行" << std::endl;
        return;
    }

    it->second->start_time = std::chrono::high_resolution_clock::now();
    it->second->is_running = true;
}

void PerformanceMonitor::endProfiling(const std::string& name) {
    auto it = profiling_data_.find(name);
    if (it == profiling_data_.end()) {
        std::cerr << "错误: 性能分析 '" << name << "' 不存在" << std::endl;
        return;
    }

    if (!it->second->is_running) {
        std::cerr << "警告: 性能分析 '" << name << "' 未在运行" << std::endl;
        return;
    }

    it->second->end_time = std::chrono::high_resolution_clock::now();
    auto duration = std::chrono::duration_cast<std::chrono::nanoseconds>(
        it->second->end_time - it->second->start_time);
    it->second->duration_ns = duration.count();
    it->second->is_running = false;
}

long long PerformanceMonitor::getDurationNs(const std::string& name) const {
    auto it = profiling_data_.find(name);
    if (it == profiling_data_.end()) {
        return 0;
    }
    return it->second->duration_ns;
}

double PerformanceMonitor::getDurationMs(const std::string& name) const {
    return static_cast<double>(getDurationNs(name)) / 1e6;
}

bool PerformanceMonitor::isProfiling(const std::string& name) const {
    auto it = profiling_data_.find(name);
    if (it == profiling_data_.end()) {
        return false;
    }
    return it->second->is_running;
}

std::vector<PerformanceMonitor::ProfilingData> PerformanceMonitor::getAllProfilingData() const {
    std::vector<ProfilingData> result;
    result.reserve(profiling_data_.size());

    for (const auto& pair : profiling_data_) {
        result.push_back(*pair.second);
    }

    return result;
}

void PerformanceMonitor::printReport() const {
    std::cout << "\n=== 性能分析报告 ===" << std::endl;

    if (profiling_data_.empty()) {
        std::cout << "没有性能分析数据" << std::endl;
        return;
    }

    // 收集所有完成的数据
    std::vector<const ProfilingData*> completed_data;
    for (const auto& pair : profiling_data_) {
        if (!pair.second->is_running && pair.second->duration_ns > 0) {
            completed_data.push_back(pair.second.get());
        }
    }

    if (completed_data.empty()) {
        std::cout << "没有完成的性能分析数据" << std::endl;
        return;
    }

    // 按执行时间排序
    std::sort(completed_data.begin(), completed_data.end(),
              [](const ProfilingData* a, const ProfilingData* b) {
                  return a->duration_ns > b->duration_ns; // 降序
              });

    // 计算统计信息
    std::vector<long long> durations;
    for (const auto* data : completed_data) {
        durations.push_back(data->duration_ns);
    }

    auto total_duration = std::accumulate(durations.begin(), durations.end(), 0LL);
    auto avg_duration = total_duration / durations.size();
    auto min_duration = *std::min_element(durations.begin(), durations.end());
    auto max_duration = *std::max_element(durations.begin(), durations.end());

    // 打印汇总信息
    std::cout << "总执行时间: " << formatDuration(total_duration) << std::endl;
    std::cout << "平均执行时间: " << formatDuration(avg_duration) << std::endl;
    std::cout << "最短执行时间: " << formatDuration(min_duration) << std::endl;
    std::cout << "最长执行时间: " << formatDuration(max_duration) << std::endl;
    std::cout << "分析项数量: " << completed_data.size() << std::endl;

    // 打印详细信息
    std::cout << "\n详细分析:" << std::endl;
    std::cout << std::left << std::setw(30) << "分析项"
              << std::right << std::setw(15) << "执行时间"
              << std::right << std::setw(12) << "百分比" << std::endl;
    std::cout << std::string(57, '-') << std::endl;

    for (const auto* data : completed_data) {
        double percentage = static_cast<double>(data->duration_ns) / total_duration * 100.0;
        std::cout << std::left << std::setw(30) << data->name
                  << std::right << std::setw(15) << formatDuration(data->duration_ns)
                  << std::right << std::setw(11) << std::fixed << std::setprecision(2) << percentage << "%"
                  << std::endl;
    }

    // 显示仍在运行的项目
    bool has_running = false;
    for (const auto& pair : profiling_data_) {
        if (pair.second->is_running) {
            if (!has_running) {
                std::cout << "\n仍在运行的项目:" << std::endl;
                has_running = true;
            }
            auto current_duration = std::chrono::duration_cast<std::chrono::nanoseconds>(
                std::chrono::high_resolution_clock::now() - pair.second->start_time);
            std::cout << "  " << pair.first << ": " << formatDuration(current_duration.count()) << " (运行中)" << std::endl;
        }
    }

    std::cout << "=== 性能分析报告结束 ===" << std::endl;
}

void PerformanceMonitor::reset() {
    profiling_data_.clear();
}

std::string PerformanceMonitor::formatDuration(long long ns) const {
    if (ns < 1000) {
        return std::to_string(ns) + " ns";
    } else if (ns < 1000000) {
        return std::to_string(ns / 1000) + " μs";
    } else if (ns < 1000000000) {
        return std::to_string(ns / 1000000) + " ms";
    } else {
        double seconds = static_cast<double>(ns) / 1e9;
        std::stringstream ss;
        ss << std::fixed << std::setprecision(3) << seconds << " s";
        return ss.str();
    }
}

long long PerformanceMonitor::getCurrentTimeNs() {
    return std::chrono::duration_cast<std::chrono::nanoseconds>(
        std::chrono::high_resolution_clock::now().time_since_epoch()).count();
}

// 注意：以下函数在不同平台上的实现可能不同
// 这里提供基本的实现，实际使用时可能需要根据平台调整

size_t PerformanceMonitor::getCurrentMemoryUsage() {
    // 在Linux系统上，可以通过读取/proc/self/statm获取内存信息
    // 这里提供一个简化的实现
    try {
        std::ifstream statm("/proc/self/statm");
        if (statm.is_open()) {
            size_t size, resident, share, text, lib, data, dt;
            statm >> size >> resident >> share >> text >> lib >> data >> dt;
            statm.close();

            // 返回常驻内存大小（KB）
            return resident * 4; // 4KB per page
        }
    } catch (...) {
        // 如果无法获取，返回0
    }

    return 0;
}

double PerformanceMonitor::getCurrentCpuUsage() {
    // 获取CPU使用率的实现比较复杂
    // 这里返回一个占位符值
    // 实际实现需要读取/proc/stat等文件

    static auto last_time = std::chrono::steady_clock::now();
    static double last_usage = 0.0;

    auto current_time = std::chrono::steady_clock::now();
    auto elapsed = std::chrono::duration_cast<std::chrono::milliseconds>(
        current_time - last_time).count();

    // 简化的CPU使用率计算
    if (elapsed > 1000) { // 每秒更新一次
        // 这里应该读取实际的CPU统计信息
        // 暂时返回一个随机值用于演示
        last_usage = (rand() % 100) / 100.0;
        last_time = current_time;
    }

    return last_usage;
}
