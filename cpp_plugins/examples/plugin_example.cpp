#include <iostream>
#include <memory>
#include <chrono>
#include <vector>
#include <map>

#include "plugin_manager.h"
#include "data_types.h"
#include "feature_plugin_base.h"
#include "decision_plugin_base.h"
#include "evaluation_plugin_base.h"
#include "event_plugin_base.h"

using namespace AlgorithmPlugins;

/**
 * @brief 创建示例数据
 */
std::shared_ptr<RealTimeData> createSampleRealTimeData() {
    auto data = std::make_shared<RealTimeData>("device001", std::chrono::system_clock::now());
    
    // 设置基础特征
    data->setMeanHF(100.5);
    data->setMeanLF(50.2);
    data->setMean(75.3);
    data->setStd(15.8);
    
    // 设置高分辨率特征
    data->setFeature1(120.1);
    data->setFeature2(80.5);
    data->setFeature3(90.2);
    data->setFeature4(110.8);
    
    // 设置其他数据
    data->setTemperature(45.5);
    data->setSpeed(1500.0);
    data->setPeakFreq(25.6);
    data->setPeakPowers(0.85);
    
    // 设置自定义特征
    data->setCustomFeature("current_rms", 12.5);
    data->setCustomFeature("audio_rms", 0.3);
    
    return data;
}

std::shared_ptr<BatchData> createSampleBatchData() {
    auto data = std::make_shared<BatchData>("device001", std::chrono::system_clock::now());
    
    // 生成示例振动数据（1000个采样点）
    std::vector<double> wave_data;
    std::vector<double> speed_data;
    
    for (int i = 0; i < 1000; ++i) {
        double t = static_cast<double>(i) / 1000.0; // 1秒的数据
        double wave = 10.0 * std::sin(2 * M_PI * 25 * t) + 5.0 * std::sin(2 * M_PI * 50 * t);
        wave_data.push_back(wave);
        speed_data.push_back(1500.0 + 100.0 * std::sin(2 * M_PI * 0.1 * t));
    }
    
    data->setWaveData(wave_data);
    data->setSpeedData(speed_data);
    data->setSamplingRate(1000);
    data->setStatus(1); // 运行状态
    
    return data;
}

/**
 * @brief 创建示例参数
 */
std::shared_ptr<PluginParameterImpl> createSampleParameters() {
    auto params = std::make_shared<PluginParameterImpl>();
    
    // 振动特征提取参数
    params->setInt("sampling_rate", 1000);
    params->setInt("duration_limit", 10);
    params->setDouble("dc_threshold", 500.0);
    
    // 状态识别参数
    params->setStringArray("select_features", {"mean_hf", "current_rms", "temp_avg"});
    params->setDoubleArray("threshold", {{0, 100, 200}, {0, 50, 100}, {20, 40, 60}});
    params->setInt("transition_status", 2);
    
    // 健康评估参数
    params->setStringArray("health_define", {"overall_health"});
    params->setIntArray("default_score", {100});
    
    // 事件报警参数
    params->setDoubleArray("alarm_line", {20, 40, 60, 80, 90, 95});
    params->setInt("tolerable_length", 5);
    params->setInt("alarm_interval", 180);
    
    return params;
}

/**
 * @brief 演示特征提取插件
 */
void demonstrateFeaturePlugin() {
    std::cout << "\n=== 特征提取插件演示 ===" << std::endl;
    
    // 创建插件管理器
    auto& manager = PluginManager::getInstance();
    
    // 注册插件工厂（这里需要实际的工厂实现）
    // manager.registerPluginFactory("vibrate31", std::make_shared<Vibrate31PluginFactory>());
    
    // 创建插件
    auto params = createSampleParameters();
    // auto plugin = manager.createPlugin("vibrate31", params);
    
    // 创建输入数据
    auto batch_data = createSampleBatchData();
    auto result = std::make_shared<PluginResultImpl>();
    
    std::cout << "输入数据: " << batch_data->getDeviceId() 
              << ", 时间戳: " << std::chrono::duration_cast<std::chrono::milliseconds>(
                  batch_data->getTimestamp().time_since_epoch()).count() << std::endl;
    
    std::cout << "波形数据长度: " << batch_data->getWaveData().size() << std::endl;
    std::cout << "采样率: " << batch_data->getSamplingRate() << std::endl;
    std::cout << "状态: " << batch_data->getStatus() << std::endl;
    
    // 模拟特征提取
    std::cout << "执行特征提取..." << std::endl;
    
    // 模拟结果
    result->setData("mean_hf", 105.2);
    result->setData("mean_lf", 52.1);
    result->setData("mean", 78.6);
    result->setData("std", 16.3);
    result->setData("peak_freq", 25.8);
    result->setData("peak_power", 0.87);
    result->setData("spectrum_energy", 1250.5);
    
    std::cout << "特征提取结果:" << std::endl;
    std::cout << "  mean_hf: " << result->getDoubleData("mean_hf") << std::endl;
    std::cout << "  mean_lf: " << result->getDoubleData("mean_lf") << std::endl;
    std::cout << "  mean: " << result->getDoubleData("mean") << std::endl;
    std::cout << "  std: " << result->getDoubleData("std") << std::endl;
    std::cout << "  peak_freq: " << result->getDoubleData("peak_freq") << std::endl;
    std::cout << "  peak_power: " << result->getDoubleData("peak_power") << std::endl;
    std::cout << "  spectrum_energy: " << result->getDoubleData("spectrum_energy") << std::endl;
}

/**
 * @brief 演示状态识别插件
 */
void demonstrateDecisionPlugin() {
    std::cout << "\n=== 状态识别插件演示 ===" << std::endl;
    
    // 创建特征数据
    auto feature_data = std::make_shared<FeatureData>("device001", std::chrono::system_clock::now());
    feature_data->setFeature("mean_hf", 105.2);
    feature_data->setFeature("current_rms", 12.5);
    feature_data->setFeature("temp_avg", 45.5);
    
    auto result = std::make_shared<PluginResultImpl>();
    
    std::cout << "输入特征数据:" << std::endl;
    std::cout << "  mean_hf: " << feature_data->getFeature("mean_hf") << std::endl;
    std::cout << "  current_rms: " << feature_data->getFeature("current_rms") << std::endl;
    std::cout << "  temp_avg: " << feature_data->getFeature("temp_avg") << std::endl;
    
    // 模拟状态识别
    std::cout << "执行状态识别..." << std::endl;
    
    // 模拟结果
    result->setData("status", 1);
    result->setData("status_name", "运行");
    result->setData("confidence", 0.95);
    
    std::cout << "状态识别结果:" << std::endl;
    std::cout << "  状态值: " << result->getIntData("status") << std::endl;
    std::cout << "  状态名称: " << result->getStringData("status_name") << std::endl;
    std::cout << "  置信度: " << result->getDoubleData("confidence") << std::endl;
}

/**
 * @brief 演示健康评估插件
 */
void demonstrateEvaluationPlugin() {
    std::cout << "\n=== 健康评估插件演示 ===" << std::endl;
    
    // 创建输入数据
    auto realtime_data = createSampleRealTimeData();
    auto result = std::make_shared<PluginResultImpl>();
    
    std::cout << "输入实时数据:" << std::endl;
    std::cout << "  设备ID: " << realtime_data->getDeviceId() << std::endl;
    std::cout << "  温度: " << realtime_data->getTemperature() << std::endl;
    std::cout << "  转速: " << realtime_data->getSpeed() << std::endl;
    std::cout << "  电流RMS: " << realtime_data->getCustomFeature("current_rms") << std::endl;
    
    // 模拟健康评估
    std::cout << "执行健康评估..." << std::endl;
    
    // 模拟结果
    result->setData("overall_health", 85.5);
    result->setData("temperature_health", 90.0);
    result->setData("current_health", 80.0);
    result->setData("vibration_health", 88.0);
    
    std::cout << "健康评估结果:" << std::endl;
    std::cout << "  整体健康度: " << result->getDoubleData("overall_health") << std::endl;
    std::cout << "  温度健康度: " << result->getDoubleData("temperature_health") << std::endl;
    std::cout << "  电流健康度: " << result->getDoubleData("current_health") << std::endl;
    std::cout << "  振动健康度: " << result->getDoubleData("vibration_health") << std::endl;
}

/**
 * @brief 演示事件处理插件
 */
void demonstrateEventPlugin() {
    std::cout << "\n=== 事件处理插件演示 ===" << std::endl;
    
    // 创建输入数据
    auto feature_data = std::make_shared<FeatureData>("device001", std::chrono::system_clock::now());
    feature_data->setFeature("overall_health", 75.0); // 触发报警的分数
    
    auto result = std::make_shared<PluginResultImpl>();
    
    std::cout << "输入健康度数据:" << std::endl;
    std::cout << "  整体健康度: " << feature_data->getFeature("overall_health") << std::endl;
    
    // 模拟事件处理
    std::cout << "执行事件处理..." << std::endl;
    
    // 模拟结果
    result->setData("event_type", static_cast<int>(EventType::SCORE_ALARM));
    result->setData("event_name", "健康度报警");
    result->setData("alarm_level", 3);
    result->setData("alarm_message", "设备健康度下降，需要关注");
    result->setData("health_score", 75.0);
    
    std::cout << "事件处理结果:" << std::endl;
    std::cout << "  事件类型: " << result->getIntData("event_type") << std::endl;
    std::cout << "  事件名称: " << result->getStringData("event_name") << std::endl;
    std::cout << "  报警级别: " << result->getIntData("alarm_level") << std::endl;
    std::cout << "  报警消息: " << result->getStringData("alarm_message") << std::endl;
    std::cout << "  健康度分数: " << result->getDoubleData("health_score") << std::endl;
}

/**
 * @brief 演示插件链管理
 */
void demonstratePluginChain() {
    std::cout << "\n=== 插件链管理演示 ===" << std::endl;
    
    // 创建插件链管理器
    PluginChainManager chain_manager;
    
    // 创建插件链配置
    PluginChainManager::ChainConfig config;
    config.chain_name = "device_monitoring_chain";
    config.plugin_names = {"vibrate31", "motor97", "comp_realtime_health34", "score_alarm5"};
    
    // 创建插件参数
    auto params = createSampleParameters();
    config.plugin_params = {params, params, params, params};
    
    // 设置数据映射
    config.data_mappings["vibrate31->motor97"] = "features";
    config.data_mappings["motor97->comp_realtime_health34"] = "status";
    config.data_mappings["comp_realtime_health34->score_alarm5"] = "health_scores";
    
    // 创建插件链
    if (chain_manager.createChain(config)) {
        std::cout << "插件链创建成功: " << config.chain_name << std::endl;
        
        // 获取插件链信息
        auto plugins = chain_manager.getChainPlugins(config.chain_name);
        std::cout << "插件链包含插件: ";
        for (const auto& plugin : plugins) {
            std::cout << plugin << " ";
        }
        std::cout << std::endl;
        
        // 执行插件链
        auto input_data = createSampleBatchData();
        auto output_result = std::make_shared<PluginResultImpl>();
        
        std::cout << "执行插件链..." << std::endl;
        if (chain_manager.executeChain(config.chain_name, input_data, output_result)) {
            std::cout << "插件链执行成功" << std::endl;
        } else {
            std::cout << "插件链执行失败" << std::endl;
        }
    } else {
        std::cout << "插件链创建失败" << std::endl;
    }
}

/**
 * @brief 演示插件监控
 */
void demonstratePluginMonitoring() {
    std::cout << "\n=== 插件监控演示 ===" << std::endl;
    
    // 创建插件监控管理器
    PluginMonitorManager monitor_manager;
    
    // 开始监控插件
    monitor_manager.startMonitoring("vibrate31");
    monitor_manager.startMonitoring("motor97");
    
    // 模拟插件执行
    for (int i = 0; i < 10; ++i) {
        bool success = (i % 10 != 7); // 模拟90%成功率
        double execution_time = 10.0 + (rand() % 50); // 10-60ms
        std::string error_message = success ? "" : "模拟错误";
        
        monitor_manager.recordExecution("vibrate31", success, execution_time, error_message);
        monitor_manager.recordExecution("motor97", success, execution_time * 0.8, error_message);
    }
    
    // 获取监控数据
    auto vibrate_metrics = monitor_manager.getPluginMetrics("vibrate31");
    auto motor_metrics = monitor_manager.getPluginMetrics("motor97");
    
    std::cout << "vibrate31插件监控数据:" << std::endl;
    std::cout << "  执行次数: " << vibrate_metrics.execution_count << std::endl;
    std::cout << "  成功次数: " << vibrate_metrics.success_count << std::endl;
    std::cout << "  错误次数: " << vibrate_metrics.error_count << std::endl;
    std::cout << "  平均执行时间: " << vibrate_metrics.avg_execution_time_ms << "ms" << std::endl;
    std::cout << "  成功率: " << monitor_manager.getSuccessRate("vibrate31") * 100 << "%" << std::endl;
    
    std::cout << "\nmotor97插件监控数据:" << std::endl;
    std::cout << "  执行次数: " << motor_metrics.execution_count << std::endl;
    std::cout << "  成功次数: " << motor_metrics.success_count << std::endl;
    std::cout << "  错误次数: " << motor_metrics.error_count << std::endl;
    std::cout << "  平均执行时间: " << motor_metrics.avg_execution_time_ms << "ms" << std::endl;
    std::cout << "  成功率: " << monitor_manager.getSuccessRate("motor97") * 100 << "%" << std::endl;
}

/**
 * @brief 主函数
 */
int main() {
    std::cout << "Algorithm Plugins Framework - 示例程序" << std::endl;
    std::cout << "=====================================" << std::endl;
    
    try {
        // 演示各种插件
        demonstrateFeaturePlugin();
        demonstrateDecisionPlugin();
        demonstrateEvaluationPlugin();
        demonstrateEventPlugin();
        demonstratePluginChain();
        demonstratePluginMonitoring();
        
        std::cout << "\n=== 演示完成 ===" << std::endl;
        std::cout << "所有插件演示成功完成！" << std::endl;
        
    } catch (const std::exception& e) {
        std::cerr << "演示过程中发生异常: " << e.what() << std::endl;
        return 1;
    }
    
    return 0;
}
