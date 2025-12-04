#include "plugin_base.h"
#include "vibrate31_plugin.h"
#include "feature_plugin_base.h"
#include "decision_plugin_base.h"
#include "evaluation_plugin_base.h"
#include "event_plugin_base.h"
#include "plugin_manager.h"

namespace AlgorithmPlugins {

// 鎸姩鐗瑰緛鎻愬彇鎻掍欢宸ュ巶
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

// 鐢垫祦鐗瑰緛鎻愬彇鎻掍欢宸ュ巶
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

// 娓╁害鐗瑰緛鎻愬彇鎻掍欢宸ュ巶
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

// 澹伴煶鐗瑰緛鎻愬彇鎻掍欢宸ュ巶
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

// 鐢垫満鐘舵€佽瘑鍒彃浠跺伐鍘?
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

// 閫氱敤鍒嗙被鍣ㄦ彃浠跺伐鍘?
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

// 瀹炴椂鍋ュ悍搴﹁瘎浼版彃浠跺伐鍘?
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

// 閿欒妫€娴嬫彃浠跺伐鍘?
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

// 鍒嗘暟鎶ヨ鎻掍欢宸ュ巶
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

// 鐘舵€佹姤璀︽彃浠跺伐鍘?
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

// 鎻掍欢娉ㄥ唽鍑芥暟
void registerAllPlugins() {
    auto& manager = PluginManager::getInstance();
    
    // 娉ㄥ唽鐗瑰緛鎻愬彇鎻掍欢
    manager.registerPluginFactory(std::make_shared<Vibrate31PluginFactory>());
    manager.registerPluginFactory(std::make_shared<CurrentFeaturePluginFactory>());
    manager.registerPluginFactory(std::make_shared<TemperatureFeaturePluginFactory>());
    manager.registerPluginFactory(std::make_shared<AudioFeaturePluginFactory>());
    
    // 娉ㄥ唽鐘舵€佽瘑鍒彃浠?
    manager.registerPluginFactory(std::make_shared<Motor97PluginFactory>());
    manager.registerPluginFactory(std::make_shared<UniversalClassify1PluginFactory>());
    
    // 娉ㄥ唽鍋ュ悍璇勪及鎻掍欢
    manager.registerPluginFactory(std::make_shared<CompRealtimeHealth34PluginFactory>());
    manager.registerPluginFactory(std::make_shared<Error18PluginFactory>());
    
    // 娉ㄥ唽浜嬩欢澶勭悊鎻掍欢
    manager.registerPluginFactory(std::make_shared<ScoreAlarm5PluginFactory>());
    manager.registerPluginFactory(std::make_shared<StatusAlarm4PluginFactory>());
}

} // namespace AlgorithmPlugins

// C鎺ュ彛瀵煎嚭鍑芥暟
extern "C" {
    void register_algorithm_plugins() {
        AlgorithmPlugins::registerAllPlugins();
    }
}
