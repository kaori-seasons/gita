// 性能监控器头文件
// 监控算法执行的性能指标和资源使用情况

#ifndef PERFORMANCE_MONITOR_HPP
#define PERFORMANCE_MONITOR_HPP

#include <string>
#include <chrono>
#include <unordered_map>
#include <vector>
#include <memory>

class PerformanceMonitor {
public:
    struct ProfilingData {
        std::string name;
        std::chrono::high_resolution_clock::time_point start_time;
        std::chrono::high_resolution_clock::time_point end_time;
        long long duration_ns;
        bool is_running;

        ProfilingData(const std::string& n)
            : name(n), duration_ns(0), is_running(false) {}
    };

    PerformanceMonitor();
    ~PerformanceMonitor() = default;

    // 开始性能分析
    void startProfiling(const std::string& name);

    // 结束性能分析
    void endProfiling(const std::string& name);

    // 获取执行时间（纳秒）
    long long getDurationNs(const std::string& name) const;

    // 获取执行时间（毫秒）
    double getDurationMs(const std::string& name) const;

    // 检查分析是否正在运行
    bool isProfiling(const std::string& name) const;

    // 获取所有分析数据
    std::vector<ProfilingData> getAllProfilingData() const;

    // 打印性能报告
    void printReport() const;

    // 重置所有分析数据
    void reset();

    // 获取内存使用情况（如果可用）
    static size_t getCurrentMemoryUsage();

    // 获取CPU使用率（如果可用）
    static double getCurrentCpuUsage();

private:
    std::unordered_map<std::string, std::unique_ptr<ProfilingData>> profiling_data_;

    // 格式化时间输出
    std::string formatDuration(long long ns) const;

    // 获取系统时间
    static long long getCurrentTimeNs();
};

#endif // PERFORMANCE_MONITOR_HPP
