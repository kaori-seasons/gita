#include "include/vibrate31_plugin.h"
#include "include/evaluation_plugin_base.h"
#include <iostream>
#include <memory>

using namespace AlgorithmPlugins;

int main() {
    std::cout << "Testing plugin instantiation..." << std::endl;
    
    // 测试Vibrate31Plugin
    try {
        auto vibrate_plugin = std::make_shared<Vibrate31Plugin>();
        std::cout << "✓ Vibrate31Plugin created: " << vibrate_plugin->getName() << std::endl;
    } catch (const std::exception& e) {
        std::cout << "✗ Vibrate31Plugin failed: " << e.what() << std::endl;
    }
    
    // 测试Error18Plugin
    try {
        auto error_plugin = std::make_shared<Error18Plugin>();
        std::cout << "✓ Error18Plugin created: " << error_plugin->getName() << std::endl;
    } catch (const std::exception& e) {
        std::cout << "✗ Error18Plugin failed: " << e.what() << std::endl;
    }
    
    // 测试CompRealtimeHealth34Plugin
    try {
        auto health_plugin = std::make_shared<CompRealtimeHealth34Plugin>();
        std::cout << "✓ CompRealtimeHealth34Plugin created: " << health_plugin->getName() << std::endl;
    } catch (const std::exception& e) {
        std::cout << "✗ CompRealtimeHealth34Plugin failed: " << e.what() << std::endl;
    }
    
    return 0;
}
