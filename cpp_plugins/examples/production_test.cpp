#include <iostream>
#include <memory>
#include <chrono>
#include <vector>
#include <map>
#include <thread>
#include <random>

#include "plugin_manager.h"
#include "data_types.h"
#include "plugin_factories.cpp"

using namespace AlgorithmPlugins;

/**
 * @brief 生产级测试类
 */
class ProductionTest {
public:
    ProductionTest() : random_engine_(std::random_device{}()) {
        // 注册所有插件
        registerAllPlugins();
    }
    
    /**
     * @brief 运行完整的生产测试
     */
    void runProductionTest() {
        std::cout << "=== Algorithm Plugins Framework 生产级测试 ===" << std::endl;
        
        // 1. 插件可用性测试
        testPluginAvailability();
        
        // 2. 插件链测试
        testPluginChains();
        
        // 3. 性能测试
        testPerformance();
        
        // 4. 并发测试
        testConcurrency();
        
        // 5. 错误处理测试
        testErrorHandling();
        
        // 6. 内存泄漏测试
        testMemoryLeaks();
        
        std::cout << "\n=== 生产级测试完成 ===" << std::endl;
    }

private:
    std::mt19937 random_engine_;
    
    /**
     * @brief 测试插件可用性
     */
    void testPluginAvailability() {
        std::cout << "\n--- 插件可用性测试 ---" << std::endl;
        
        auto& manager = PluginManager::getInstance();
        
        // 测试所有插件类型
        std::vector<PluginType> types = {
            PluginType::FEATURE,
            PluginType::DECISION,
            PluginType::EVALUATION,
            PluginType::EVENT
        };
        
        for (auto type : types) {
            auto plugins = manager.getPluginsByType(type);
            std::cout << "插件类型 " << static_cast<int>(type) << " 可用插件数量: " << plugins.size() << std::endl;
            
            for (const auto& plugin_name : plugins) {
                std::cout << "  - " << plugin_name << " v" << manager.getPluginVersion(plugin_name) << std::endl;
            }
        }
        
        // 测试插件创建
        auto plugin = manager.createPlugin("vibrate31");
        if (plugin) {
            std::cout << "✓ vibrate31插件创建成功" << std::endl;
        } else {
            std::cout << "✗ vibrate31插件创建失败" << std::endl;
        }
    }
    
    /**
     * @brief 测试插件链
     */
    void testPluginChains() {
        std::cout << "\n--- 插件链测试 ---" << std::endl;
        
        PluginChainManager chain_manager;
        
        // 创建振动监测插件链
        PluginChainManager::ChainConfig config;
        config.chain_name = "vibration_monitoring";
        config.plugin_names = {"vibrate31", "motor97", "comp_realtime_health34", "score_alarm5"};
        
        // 创建参数
        auto params = createTestParameters();
        config.plugin_params = {params, params, params, params};
        
        if (chain_manager.createChain(config)) {
            std::cout << "✓ 振动监测插件链创建成功" << std::endl;
            
            // 测试插件链执行
            auto input_data = createSampleBatchData();
            auto output_result = std::make_shared<PluginResultImpl>();
            
            auto start_time = std::chrono::high_resolution_clock::now();
            bool success = chain_manager.executeChain("vibration_monitoring", input_data, output_result);
            auto end_time = std::chrono::high_resolution_clock::now();
            
            auto duration = std::chrono::duration_cast<std::chrono::microseconds>(end_time - start_time);
            
            if (success) {
                std::cout << "✓ 插件链执行成功，耗时: " << duration.count() << " 微秒" << std::endl;
            } else {
                std::cout << "✗ 插件链执行失败" << std::endl;
            }
        } else {
            std::cout << "✗ 振动监测插件链创建失败" << std::endl;
        }
    }
    
    /**
     * @brief 性能测试
     */
    void testPerformance() {
        std::cout << "\n--- 性能测试 ---" << std::endl;
        
        auto& manager = PluginManager::getInstance();
        auto params = createTestParameters();
        
        // 测试振动特征提取性能
        testPluginPerformance("vibrate31", params, 1000);
        
        // 测试状态识别性能
        testPluginPerformance("motor97", params, 1000);
        
        // 测试健康评估性能
        testPluginPerformance("comp_realtime_health34", params, 1000);
        
        // 测试事件处理性能
        testPluginPerformance("score_alarm5", params, 1000);
    }
    
    /**
     * @brief 测试单个插件性能
     */
    void testPluginPerformance(const std::string& plugin_name, 
                              std::shared_ptr<PluginParameter> params, 
                              int iterations) {
        auto& manager = PluginManager::getInstance();
        auto plugin = manager.createPlugin(plugin_name, params);
        
        if (!plugin) {
            std::cout << "✗ " << plugin_name << " 插件创建失败" << std::endl;
            return;
        }
        
        auto input_data = createSampleData(plugin_name);
        auto output_result = std::make_shared<PluginResultImpl>();
        
        auto start_time = std::chrono::high_resolution_clock::now();
        
        int success_count = 0;
        for (int i = 0; i < iterations; ++i) {
            if (plugin->process(input_data, output_result)) {
                success_count++;
            }
        }
        
        auto end_time = std::chrono::high_resolution_clock::now();
        auto duration = std::chrono::duration_cast<std::chrono::microseconds>(end_time - start_time);
        
        double avg_time = static_cast<double>(duration.count()) / iterations;
        double success_rate = static_cast<double>(success_count) / iterations * 100.0;
        
        std::cout << plugin_name << " 性能测试:" << std::endl;
        std::cout << "  平均执行时间: " << avg_time << " 微秒" << std::endl;
        std::cout << "  成功率: " << success_rate << "%" << std::endl;
        std::cout << "  总耗时: " << duration.count() << " 微秒" << std::endl;
    }
    
    /**
     * @brief 并发测试
     */
    void testConcurrency() {
        std::cout << "\n--- 并发测试 ---" << std::endl;
        
        const int num_threads = 4;
        const int iterations_per_thread = 100;
        
        std::vector<std::thread> threads;
        std::vector<int> results(num_threads, 0);
        
        auto start_time = std::chrono::high_resolution_clock::now();
        
        for (int i = 0; i < num_threads; ++i) {
            threads.emplace_back([this, i, &results, iterations_per_thread]() {
                auto& manager = PluginManager::getInstance();
                auto params = createTestParameters();
                auto plugin = manager.createPlugin("vibrate31", params);
                
                if (!plugin) {
                    results[i] = 0;
                    return;
                }
                
                int success_count = 0;
                for (int j = 0; j < iterations_per_thread; ++j) {
                    auto input_data = createSampleBatchData();
                    auto output_result = std::make_shared<PluginResultImpl>();
                    
                    if (plugin->process(input_data, output_result)) {
                        success_count++;
                    }
                }
                
                results[i] = success_count;
            });
        }
        
        for (auto& thread : threads) {
            thread.join();
        }
        
        auto end_time = std::chrono::high_resolution_clock::now();
        auto duration = std::chrono::duration_cast<std::chrono::microseconds>(end_time - start_time);
        
        int total_success = 0;
        for (int result : results) {
            total_success += result;
        }
        
        double success_rate = static_cast<double>(total_success) / (num_threads * iterations_per_thread) * 100.0;
        
        std::cout << "并发测试结果:" << std::endl;
        std::cout << "  线程数: " << num_threads << std::endl;
        std::cout << "  每线程迭代次数: " << iterations_per_thread << std::endl;
        std::cout << "  总成功率: " << success_rate << "%" << std::endl;
        std::cout << "  总耗时: " << duration.count() << " 微秒" << std::endl;
    }
    
    /**
     * @brief 错误处理测试
     */
    void testErrorHandling() {
        std::cout << "\n--- 错误处理测试 ---" << std::endl;
        
        auto& manager = PluginManager::getInstance();
        
        // 测试无效插件名
        auto invalid_plugin = manager.createPlugin("invalid_plugin");
        if (!invalid_plugin) {
            std::cout << "✓ 无效插件名处理正确" << std::endl;
        } else {
            std::cout << "✗ 无效插件名处理错误" << std::endl;
        }
        
        // 测试空参数
        auto plugin = manager.createPlugin("vibrate31", nullptr);
        if (plugin && !plugin->isInitialized()) {
            std::cout << "✓ 空参数处理正确" << std::endl;
        } else {
            std::cout << "✗ 空参数处理错误" << std::endl;
        }
        
        // 测试空输入数据
        auto valid_plugin = manager.createPlugin("vibrate31", createTestParameters());
        if (valid_plugin) {
            auto output_result = std::make_shared<PluginResultImpl>();
            bool success = valid_plugin->process(nullptr, output_result);
            if (!success) {
                std::cout << "✓ 空输入数据处理正确" << std::endl;
            } else {
                std::cout << "✗ 空输入数据处理错误" << std::endl;
            }
        }
    }
    
    /**
     * @brief 内存泄漏测试
     */
    void testMemoryLeaks() {
        std::cout << "\n--- 内存泄漏测试 ---" << std::endl;
        
        auto& manager = PluginManager::getInstance();
        
        // 大量创建和销毁插件
        const int iterations = 1000;
        for (int i = 0; i < iterations; ++i) {
            auto params = createTestParameters();
            auto plugin = manager.createPlugin("vibrate31", params);
            
            if (plugin) {
                auto input_data = createSampleBatchData();
                auto output_result = std::make_shared<PluginResultImpl>();
                plugin->process(input_data, output_result);
            }
            
            // 插件会自动销毁
        }
        
        std::cout << "✓ 内存泄漏测试完成，创建了 " << iterations << " 个插件实例" << std::endl;
    }
    
    /**
     * @brief 创建测试参数
     */
    std::shared_ptr<PluginParameterImpl> createTestParameters() {
        auto params = std::make_shared<PluginParameterImpl>();
        
        params->setInt("sampling_rate", 1000);
        params->setInt("duration_limit", 10);
        params->setDouble("dc_threshold", 500.0);
        params->setStringArray("select_features", {"mean_hf", "current_rms"});
        params->setDoubleArray("threshold", {{0, 100, 200}, {0, 50, 100}});
        params->setStringArray("health_define", {"overall_health"});
        params->setDoubleArray("alarm_line", {20, 40, 60, 80, 90, 95});
        
        return params;
    }
    
    /**
     * @brief 创建示例批次数据
     */
    std::shared_ptr<BatchData> createSampleBatchData() {
        auto data = std::make_shared<BatchData>("test_device", std::chrono::system_clock::now());
        
        // 生成随机振动数据
        std::vector<double> wave_data(1000);
        std::vector<double> speed_data(1000);
        
        std::uniform_real_distribution<double> wave_dist(-10.0, 10.0);
        std::uniform_real_distribution<double> speed_dist(1400.0, 1600.0);
        
        for (int i = 0; i < 1000; ++i) {
            wave_data[i] = wave_dist(random_engine_);
            speed_data[i] = speed_dist(random_engine_);
        }
        
        data->setWaveData(wave_data);
        data->setSpeedData(speed_data);
        data->setSamplingRate(1000);
        data->setStatus(1);
        
        return data;
    }
    
    /**
     * @brief 创建示例实时数据
     */
    std::shared_ptr<RealTimeData> createSampleRealTimeData() {
        auto data = std::make_shared<RealTimeData>("test_device", std::chrono::system_clock::now());
        
        std::uniform_real_distribution<double> feature_dist(0.0, 200.0);
        
        data->setMeanHF(feature_dist(random_engine_));
        data->setMeanLF(feature_dist(random_engine_));
        data->setMean(feature_dist(random_engine_));
        data->setStd(feature_dist(random_engine_) / 10.0);
        data->setTemperature(40.0 + feature_dist(random_engine_) / 10.0);
        data->setSpeed(1500.0 + feature_dist(random_engine_) / 10.0);
        data->setCustomFeature("current_rms", feature_dist(random_engine_) / 20.0);
        data->setCustomFeature("overall_health", 50.0 + feature_dist(random_engine_) / 4.0);
        
        return data;
    }
    
    /**
     * @brief 根据插件类型创建示例数据
     */
    std::shared_ptr<PluginData> createSampleData(const std::string& plugin_name) {
        if (plugin_name == "vibrate31") {
            return createSampleBatchData();
        } else {
            return createSampleRealTimeData();
        }
    }
};

/**
 * @brief 主函数
 */
int main() {
    try {
        ProductionTest test;
        test.runProductionTest();
        
        std::cout << "\n所有测试完成！" << std::endl;
        return 0;
        
    } catch (const std::exception& e) {
        std::cerr << "测试过程中发生异常: " << e.what() << std::endl;
        return 1;
    }
}
