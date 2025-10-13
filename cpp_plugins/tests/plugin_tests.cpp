#include <gtest/gtest.h>
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
 * @brief 测试数据创建辅助类
 */
class TestDataHelper {
public:
    static std::shared_ptr<RealTimeData> createRealTimeData(const std::string& deviceId = "test_device") {
        auto data = std::make_shared<RealTimeData>(deviceId, std::chrono::system_clock::now());
        
        data->setMeanHF(100.0);
        data->setMeanLF(50.0);
        data->setMean(75.0);
        data->setStd(15.0);
        data->setTemperature(45.0);
        data->setSpeed(1500.0);
        data->setCustomFeature("current_rms", 12.0);
        
        return data;
    }
    
    static std::shared_ptr<BatchData> createBatchData(const std::string& deviceId = "test_device") {
        auto data = std::make_shared<BatchData>(deviceId, std::chrono::system_clock::now());
        
        std::vector<double> wave_data(1000, 10.0);
        std::vector<double> speed_data(1000, 1500.0);
        
        data->setWaveData(wave_data);
        data->setSpeedData(speed_data);
        data->setSamplingRate(1000);
        data->setStatus(1);
        
        return data;
    }
    
    static std::shared_ptr<FeatureData> createFeatureData(const std::string& deviceId = "test_device") {
        auto data = std::make_shared<FeatureData>(deviceId, std::chrono::system_clock::now());
        
        data->setFeature("mean_hf", 100.0);
        data->setFeature("current_rms", 12.0);
        data->setFeature("temp_avg", 45.0);
        
        return data;
    }
    
    static std::shared_ptr<StatusData> createStatusData(const std::string& deviceId = "test_device") {
        auto data = std::make_shared<StatusData>(deviceId, std::chrono::system_clock::now());
        
        data->setStatus(1);
        data->setStatusDescription("运行");
        
        std::map<int, std::string> mapping = {{0, "停机"}, {1, "运行"}, {2, "过渡"}};
        data->setStatusMapping(mapping);
        
        return data;
    }
    
    static std::shared_ptr<PluginParameterImpl> createTestParameters() {
        auto params = std::make_shared<PluginParameterImpl>();
        
        params->setInt("sampling_rate", 1000);
        params->setInt("duration_limit", 10);
        params->setDouble("dc_threshold", 500.0);
        params->setStringArray("select_features", {"mean_hf", "current_rms"});
        params->setDoubleArray("threshold", {{0, 100, 200}, {0, 50, 100}});
        
        return params;
    }
};

/**
 * @brief 插件基类测试
 */
class PluginBaseTest : public ::testing::Test {
protected:
    void SetUp() override {
        plugin_manager_ = &PluginManager::getInstance();
        test_params_ = TestDataHelper::createTestParameters();
    }
    
    void TearDown() override {
        plugin_manager_->clearAllPlugins();
    }
    
    PluginManager* plugin_manager_;
    std::shared_ptr<PluginParameterImpl> test_params_;
};

/**
 * @brief 数据类型测试
 */
TEST_F(PluginBaseTest, DataTypesTest) {
    // 测试RealTimeData
    auto realtime_data = TestDataHelper::createRealTimeData();
    EXPECT_EQ(realtime_data->getType(), DataType::REAL_TIME);
    EXPECT_EQ(realtime_data->getDeviceId(), "test_device");
    EXPECT_EQ(realtime_data->getMeanHF(), 100.0);
    EXPECT_EQ(realtime_data->getTemperature(), 45.0);
    EXPECT_EQ(realtime_data->getCustomFeature("current_rms"), 12.0);
    
    // 测试BatchData
    auto batch_data = TestDataHelper::createBatchData();
    EXPECT_EQ(batch_data->getType(), DataType::BATCH_DATA);
    EXPECT_EQ(batch_data->getWaveData().size(), 1000);
    EXPECT_EQ(batch_data->getSamplingRate(), 1000);
    EXPECT_EQ(batch_data->getStatus(), 1);
    
    // 测试FeatureData
    auto feature_data = TestDataHelper::createFeatureData();
    EXPECT_EQ(feature_data->getType(), DataType::FEATURE_DATA);
    EXPECT_TRUE(feature_data->hasFeature("mean_hf"));
    EXPECT_EQ(feature_data->getFeature("mean_hf"), 100.0);
    
    // 测试StatusData
    auto status_data = TestDataHelper::createStatusData();
    EXPECT_EQ(status_data->getType(), DataType::STATUS_DATA);
    EXPECT_EQ(status_data->getStatus(), 1);
    EXPECT_EQ(status_data->getStatusName(1), "运行");
}

/**
 * @brief 插件参数测试
 */
TEST_F(PluginBaseTest, PluginParameterTest) {
    EXPECT_EQ(test_params_->getInt("sampling_rate"), 1000);
    EXPECT_EQ(test_params_->getInt("duration_limit"), 10);
    EXPECT_EQ(test_params_->getDouble("dc_threshold"), 500.0);
    
    auto select_features = test_params_->getStringArray("select_features");
    EXPECT_EQ(select_features.size(), 2);
    EXPECT_EQ(select_features[0], "mean_hf");
    EXPECT_EQ(select_features[1], "current_rms");
    
    auto thresholds = test_params_->getDoubleArray("threshold");
    EXPECT_EQ(thresholds.size(), 2);
    EXPECT_EQ(thresholds[0].size(), 3);
    EXPECT_EQ(thresholds[1].size(), 3);
}

/**
 * @brief 插件结果测试
 */
TEST_F(PluginBaseTest, PluginResultTest) {
    auto result = std::make_shared<PluginResultImpl>();
    
    // 设置数据
    result->setData("test_string", "hello");
    result->setData("test_double", 123.45);
    result->setData("test_int", 42);
    
    // 获取数据
    EXPECT_EQ(result->getStringData("test_string"), "hello");
    EXPECT_EQ(result->getDoubleData("test_double"), 123.45);
    EXPECT_EQ(result->getIntData("test_int"), 42);
    
    // 检查数据存在性
    EXPECT_TRUE(result->hasData("test_string"));
    EXPECT_TRUE(result->hasData("test_double"));
    EXPECT_TRUE(result->hasData("test_int"));
    EXPECT_FALSE(result->hasData("non_existent"));
}

/**
 * @brief 插件管理器测试
 */
TEST_F(PluginBaseTest, PluginManagerTest) {
    // 测试插件注册（需要实际的插件工厂）
    // auto factory = std::make_shared<TestPluginFactory>();
    // EXPECT_TRUE(plugin_manager_->registerPluginFactory("test_plugin", factory));
    // EXPECT_TRUE(plugin_manager_->isPluginAvailable("test_plugin"));
    
    // 测试插件创建
    // auto plugin = plugin_manager_->createPlugin("test_plugin", test_params_);
    // EXPECT_NE(plugin, nullptr);
    // EXPECT_TRUE(plugin->isInitialized());
    
    // 测试插件信息查询
    // EXPECT_EQ(plugin_manager_->getPluginType("test_plugin"), PluginType::FEATURE);
    // EXPECT_FALSE(plugin_manager_->getPluginVersion("test_plugin").empty());
    // EXPECT_FALSE(plugin_manager_->getPluginDescription("test_plugin").empty());
}

/**
 * @brief 插件链管理器测试
 */
TEST_F(PluginBaseTest, PluginChainManagerTest) {
    PluginChainManager chain_manager;
    
    // 创建插件链配置
    PluginChainManager::ChainConfig config;
    config.chain_name = "test_chain";
    config.plugin_names = {"plugin1", "plugin2"};
    config.plugin_params = {test_params_, test_params_};
    
    // 测试插件链创建
    EXPECT_TRUE(chain_manager.createChain(config));
    EXPECT_TRUE(chain_manager.isChainAvailable("test_chain"));
    
    // 测试插件链查询
    auto plugins = chain_manager.getChainPlugins("test_chain");
    EXPECT_EQ(plugins.size(), 2);
    EXPECT_EQ(plugins[0], "plugin1");
    EXPECT_EQ(plugins[1], "plugin2");
    
    // 测试插件链清理
    EXPECT_TRUE(chain_manager.clearChain("test_chain"));
    EXPECT_FALSE(chain_manager.isChainAvailable("test_chain"));
}

/**
 * @brief 插件配置管理器测试
 */
TEST_F(PluginBaseTest, PluginConfigManagerTest) {
    PluginConfigManager config_manager;
    
    // 测试插件配置
    EXPECT_TRUE(config_manager.setPluginConfig("test_plugin", test_params_));
    auto retrieved_config = config_manager.getPluginConfig("test_plugin");
    EXPECT_NE(retrieved_config, nullptr);
    EXPECT_EQ(retrieved_config->getInt("sampling_rate"), 1000);
    
    // 测试场景配置
    std::map<std::string, std::string> scene_config = {{"param1", "value1"}, {"param2", "value2"}};
    EXPECT_TRUE(config_manager.setSceneConfig("test_scene", scene_config));
    auto retrieved_scene = config_manager.getSceneConfig("test_scene");
    EXPECT_EQ(retrieved_scene.size(), 2);
    EXPECT_EQ(retrieved_scene["param1"], "value1");
    
    // 测试全局配置
    EXPECT_TRUE(config_manager.setGlobalConfig("global_param", "global_value"));
    EXPECT_EQ(config_manager.getGlobalConfig("global_param"), "global_value");
    EXPECT_EQ(config_manager.getGlobalConfig("non_existent", "default"), "default");
}

/**
 * @brief 插件监控管理器测试
 */
TEST_F(PluginBaseTest, PluginMonitorManagerTest) {
    PluginMonitorManager monitor_manager;
    
    // 开始监控
    monitor_manager.startMonitoring("test_plugin");
    EXPECT_TRUE(monitor_manager.getMonitoredPlugins().size() > 0);
    
    // 记录执行
    for (int i = 0; i < 10; ++i) {
        bool success = (i % 10 != 7); // 90%成功率
        double execution_time = 10.0 + (i % 50);
        monitor_manager.recordExecution("test_plugin", success, execution_time);
    }
    
    // 获取监控数据
    auto metrics = monitor_manager.getPluginMetrics("test_plugin");
    EXPECT_EQ(metrics.execution_count, 10);
    EXPECT_EQ(metrics.success_count, 9);
    EXPECT_EQ(metrics.error_count, 1);
    EXPECT_GT(metrics.avg_execution_time_ms, 0.0);
    
    // 测试性能统计
    EXPECT_EQ(monitor_manager.getExecutionCount("test_plugin"), 10);
    EXPECT_EQ(monitor_manager.getSuccessRate("test_plugin"), 0.9);
    
    // 停止监控
    monitor_manager.stopMonitoring("test_plugin");
}

/**
 * @brief 数据序列化测试
 */
TEST_F(PluginBaseTest, DataSerializationTest) {
    // 测试RealTimeData序列化
    auto realtime_data = TestDataHelper::createRealTimeData();
    std::string serialized = realtime_data->serialize();
    EXPECT_FALSE(serialized.empty());
    
    auto deserialized_data = std::make_shared<RealTimeData>("", std::chrono::system_clock::now());
    EXPECT_TRUE(deserialized_data->deserialize(serialized));
    EXPECT_EQ(deserialized_data->getMeanHF(), realtime_data->getMeanHF());
    EXPECT_EQ(deserialized_data->getTemperature(), realtime_data->getTemperature());
    
    // 测试BatchData序列化
    auto batch_data = TestDataHelper::createBatchData();
    std::string batch_serialized = batch_data->serialize();
    EXPECT_FALSE(batch_serialized.empty());
    
    auto deserialized_batch = std::make_shared<BatchData>("", std::chrono::system_clock::now());
    EXPECT_TRUE(deserialized_batch->deserialize(batch_serialized));
    EXPECT_EQ(deserialized_batch->getWaveData().size(), batch_data->getWaveData().size());
    EXPECT_EQ(deserialized_batch->getSamplingRate(), batch_data->getSamplingRate());
    
    // 测试FeatureData序列化
    auto feature_data = TestDataHelper::createFeatureData();
    std::string feature_serialized = feature_data->serialize();
    EXPECT_FALSE(feature_serialized.empty());
    
    auto deserialized_feature = std::make_shared<FeatureData>("", std::chrono::system_clock::now());
    EXPECT_TRUE(deserialized_feature->deserialize(feature_serialized));
    EXPECT_EQ(deserialized_feature->getFeature("mean_hf"), feature_data->getFeature("mean_hf"));
}

/**
 * @brief 性能测试
 */
TEST_F(PluginBaseTest, PerformanceTest) {
    const int iterations = 1000;
    
    // 测试数据创建性能
    auto start_time = std::chrono::high_resolution_clock::now();
    
    for (int i = 0; i < iterations; ++i) {
        auto data = TestDataHelper::createRealTimeData();
        auto result = std::make_shared<PluginResultImpl>();
        result->setData("test", static_cast<double>(i));
    }
    
    auto end_time = std::chrono::high_resolution_clock::now();
    auto duration = std::chrono::duration_cast<std::chrono::microseconds>(end_time - start_time);
    
    std::cout << "创建 " << iterations << " 个数据对象耗时: " 
              << duration.count() << " 微秒" << std::endl;
    
    // 测试序列化性能
    auto test_data = TestDataHelper::createRealTimeData();
    
    start_time = std::chrono::high_resolution_clock::now();
    
    for (int i = 0; i < iterations; ++i) {
        std::string serialized = test_data->serialize();
        auto deserialized = std::make_shared<RealTimeData>("", std::chrono::system_clock::now());
        deserialized->deserialize(serialized);
    }
    
    end_time = std::chrono::high_resolution_clock::now();
    duration = std::chrono::duration_cast<std::chrono::microseconds>(end_time - start_time);
    
    std::cout << "序列化/反序列化 " << iterations << " 次耗时: " 
              << duration.count() << " 微秒" << std::endl;
}

/**
 * @brief 主测试函数
 */
int main(int argc, char** argv) {
    ::testing::InitGoogleTest(&argc, argv);
    return RUN_ALL_TESTS();
}
