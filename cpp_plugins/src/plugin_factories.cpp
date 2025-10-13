#include "plugin_base.h"
#include "feature_plugin_base.h"
#include "decision_plugin_base.h"
#include "evaluation_plugin_base.h"
#include "event_plugin_base.h"

namespace AlgorithmPlugins {

// 振动特征提取插件工厂
class Vibrate31PluginFactory : public IPluginFactory {
public:
    std::shared_ptr<IPlugin> createPlugin() override {
        return std::make_shared<Vibrate31Plugin>();
    }
    
    std::string getPluginName() const override {
        return "vibrate31";
    }
    
    PluginType getPluginType() const override {
        return PluginType::FEATURE;
    }
};

// 电流特征提取插件工厂
class CurrentFeaturePluginFactory : public IPluginFactory {
public:
    std::shared_ptr<IPlugin> createPlugin() override {
        return std::make_shared<CurrentFeaturePlugin>();
    }
    
    std::string getPluginName() const override {
        return "current_feature_extractor";
    }
    
    PluginType getPluginType() const override {
        return PluginType::FEATURE;
    }
};

// 温度特征提取插件工厂
class TemperatureFeaturePluginFactory : public IPluginFactory {
public:
    std::shared_ptr<IPlugin> createPlugin() override {
        return std::make_shared<TemperatureFeaturePlugin>();
    }
    
    std::string getPluginName() const override {
        return "temperature_feature_extractor";
    }
    
    PluginType getPluginType() const override {
        return PluginType::FEATURE;
    }
};

// 声音特征提取插件工厂
class AudioFeaturePluginFactory : public IPluginFactory {
public:
    std::shared_ptr<IPlugin> createPlugin() override {
        return std::make_shared<AudioFeaturePlugin>();
    }
    
    std::string getPluginName() const override {
        return "audio_feature_extractor";
    }
    
    PluginType getPluginType() const override {
        return PluginType::FEATURE;
    }
};

// 电机状态识别插件工厂
class Motor97PluginFactory : public IPluginFactory {
public:
    std::shared_ptr<IPlugin> createPlugin() override {
        return std::make_shared<Motor97Plugin>();
    }
    
    std::string getPluginName() const override {
        return "motor97";
    }
    
    PluginType getPluginType() const override {
        return PluginType::DECISION;
    }
};

// 通用分类器插件工厂
class UniversalClassify1PluginFactory : public IPluginFactory {
public:
    std::shared_ptr<IPlugin> createPlugin() override {
        return std::make_shared<UniversalClassify1Plugin>();
    }
    
    std::string getPluginName() const override {
        return "universal_classify1";
    }
    
    PluginType getPluginType() const override {
        return PluginType::DECISION;
    }
};

// 实时健康度评估插件工厂
class CompRealtimeHealth34PluginFactory : public IPluginFactory {
public:
    std::shared_ptr<IPlugin> createPlugin() override {
        return std::make_shared<CompRealtimeHealth34Plugin>();
    }
    
    std::string getPluginName() const override {
        return "comp_realtime_health34";
    }
    
    PluginType getPluginType() const override {
        return PluginType::EVALUATION;
    }
};

// 错误检测插件工厂
class Error18PluginFactory : public IPluginFactory {
public:
    std::shared_ptr<IPlugin> createPlugin() override {
        return std::make_shared<Error18Plugin>();
    }
    
    std::string getPluginName() const override {
        return "error18";
    }
    
    PluginType getPluginType() const override {
        return PluginType::EVALUATION;
    }
};

// 分数报警插件工厂
class ScoreAlarm5PluginFactory : public IPluginFactory {
public:
    std::shared_ptr<IPlugin> createPlugin() override {
        return std::make_shared<ScoreAlarm5Plugin>();
    }
    
    std::string getPluginName() const override {
        return "score_alarm5";
    }
    
    PluginType getPluginType() const override {
        return PluginType::EVENT;
    }
};

// 状态报警插件工厂
class StatusAlarm4PluginFactory : public IPluginFactory {
public:
    std::shared_ptr<IPlugin> createPlugin() override {
        return std::make_shared<StatusAlarm4Plugin>();
    }
    
    std::string getPluginName() const override {
        return "status_alarm4";
    }
    
    PluginType getPluginType() const override {
        return PluginType::EVENT;
    }
};

// 插件注册函数
void registerAllPlugins() {
    auto& manager = PluginManager::getInstance();
    
    // 注册特征提取插件
    manager.registerPluginFactory(std::make_shared<Vibrate31PluginFactory>());
    manager.registerPluginFactory(std::make_shared<CurrentFeaturePluginFactory>());
    manager.registerPluginFactory(std::make_shared<TemperatureFeaturePluginFactory>());
    manager.registerPluginFactory(std::make_shared<AudioFeaturePluginFactory>());
    
    // 注册状态识别插件
    manager.registerPluginFactory(std::make_shared<Motor97PluginFactory>());
    manager.registerPluginFactory(std::make_shared<UniversalClassify1PluginFactory>());
    
    // 注册健康评估插件
    manager.registerPluginFactory(std::make_shared<CompRealtimeHealth34PluginFactory>());
    manager.registerPluginFactory(std::make_shared<Error18PluginFactory>());
    
    // 注册事件处理插件
    manager.registerPluginFactory(std::make_shared<ScoreAlarm5PluginFactory>());
    manager.registerPluginFactory(std::make_shared<StatusAlarm4PluginFactory>());
}

} // namespace AlgorithmPlugins

// C接口导出函数
extern "C" {
    void register_algorithm_plugins() {
        AlgorithmPlugins::registerAllPlugins();
    }
}
