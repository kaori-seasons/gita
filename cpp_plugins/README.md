# Algorithm Plugins Framework - C++ 重构版本

## 项目概述

这是原有Python算法依赖库的C++重构版本，提供了高性能、稳定的算法插件框架。该框架支持特征提取、状态识别、健康评估、事件处理等多种算法插件类型。

## 主要特性

- **高性能**: 使用C++实现，提供比Python版本更高的执行效率
- **模块化设计**: 支持插件化架构，易于扩展和维护
- **跨平台**: 支持Windows和Linux平台
- **类型安全**: 强类型系统，减少运行时错误
- **内存安全**: 使用智能指针管理内存，避免内存泄漏
- **并发安全**: 支持多线程环境下的安全执行
- **监控支持**: 内置性能监控和错误追踪
- **配置灵活**: 支持JSON配置文件，易于部署和管理

## 系统架构

```
┌─────────────────────────────────────────────────────────────┐
│                    Plugin Manager                          │
├─────────────────────────────────────────────────────────────┤
│  Feature Plugins  │  Decision Plugins  │  Evaluation Plugins │
│  - vibrate31      │  - motor97          │  - comp_realtime_   │
│  - current_feature│  - universal_classify│    health34         │
│  - temperature    │  - nn_classify      │  - error18          │
│  - audio          │                     │  - distance_health  │
├─────────────────────────────────────────────────────────────┤
│  Event Plugins    │  Other Plugins      │  Summary Plugins    │
│  - score_alarm5   │  - autoanalyzer24   │  - summary_result   │
│  - status_alarm4  │  - blockage37       │                     │
├─────────────────────────────────────────────────────────────┤
│                    Data Types Layer                         │
│  - RealTimeData   │  - BatchData        │  - FeatureData     │
│  - StatusData     │  - PluginResult     │  - PluginParameter │
├─────────────────────────────────────────────────────────────┤
│                    Core Framework                          │
│  - Plugin Base    │  - Plugin Manager   │  - Plugin Chain    │
│  - Config Manager │  - Monitor Manager   │  - Serialization   │
└─────────────────────────────────────────────────────────────┘
```

## 插件类型

### 1. 特征提取插件 (Feature Plugins)
- **vibrate31**: 振动特征提取，支持频谱分析和工况分割
- **current_feature_extractor**: 电流特征提取，计算RMS、峰值等
- **temperature_feature_extractor**: 温度特征提取，计算统计量
- **audio_feature_extractor**: 声音特征提取，频谱分析

### 2. 状态识别插件 (Decision Plugins)
- **motor97**: 电机状态识别，基于多特征阈值
- **universal_classify1**: 通用分类器，支持统计量和时序分析
- **nn_classify49**: 神经网络分类器

### 3. 健康评估插件 (Evaluation Plugins)
- **comp_realtime_health34**: 实时健康度评估
- **error18**: 错误检测和异常分析
- **distance_to_health36**: 距离健康度评估

### 4. 事件处理插件 (Event Plugins)
- **score_alarm5**: 分数报警，基于健康度阈值
- **status_alarm4**: 状态报警，基于设备状态

## 快速开始

### 环境要求

- **编译器**: 
  - Windows: Visual Studio 2019 或更高版本
  - Linux: GCC 7.0 或更高版本
- **CMake**: 3.16 或更高版本
- **依赖库**:
  - FFTW3 (可选，用于频谱分析)
  - nlohmann/json (可选，用于JSON处理)

### 构建步骤

#### Windows平台

```cmd
# 使用批处理脚本
build.bat

# 或者手动构建
mkdir build
cd build
cmake .. -G "Visual Studio 16 2019" -A x64
cmake --build . --config Release
```

#### Linux平台

```bash
# 使用Shell脚本
./build.sh

# 或者手动构建
mkdir build
cd build
cmake .. -DCMAKE_BUILD_TYPE=Release
make -j$(nproc)
```

### 运行示例

```bash
# 运行示例程序
./bin/plugin_example

# 运行测试
./bin/plugin_tests

# 运行插件加载器
./bin/plugin_loader
```

## 使用指南

### 1. 创建自定义插件

```cpp
#include "feature_plugin_base.h"

class MyCustomPlugin : public FeaturePluginBase {
public:
    std::string getName() const override {
        return "my_custom_plugin";
    }
    
    std::string getVersion() const override {
        return "1.0.0";
    }
    
    std::string getDescription() const override {
        return "我的自定义插件";
    }
    
    std::vector<std::string> getRequiredParameters() const override {
        return {"param1", "param2"};
    }
    
    std::vector<std::string> getFeatureNames() const override {
        return {"feature1", "feature2"};
    }

protected:
    bool validateParameters() override {
        // 验证参数
        return true;
    }
    
    bool extractFeatures(std::shared_ptr<PluginData> input, 
                        std::shared_ptr<PluginResult> output) override {
        // 实现特征提取逻辑
        output->setData("feature1", 100.0);
        output->setData("feature2", 200.0);
        return true;
    }
};
```

### 2. 使用插件管理器

```cpp
#include "plugin_manager.h"

// 获取插件管理器实例
auto& manager = PluginManager::getInstance();

// 注册插件
manager.registerPluginFactory("my_plugin", std::make_shared<MyPluginFactory>());

// 创建插件
auto params = std::make_shared<PluginParameterImpl>();
params->setString("param1", "value1");
auto plugin = manager.createPlugin("my_plugin", params);

// 使用插件
auto input_data = std::make_shared<RealTimeData>("device001", std::chrono::system_clock::now());
auto output_result = std::make_shared<PluginResultImpl>();
plugin->process(input_data, output_result);
```

### 3. 配置插件链

```cpp
#include "plugin_chain_manager.h"

PluginChainManager chain_manager;

// 创建插件链配置
PluginChainManager::ChainConfig config;
config.chain_name = "my_chain";
config.plugin_names = {"plugin1", "plugin2", "plugin3"};
config.plugin_params = {params1, params2, params3};

// 创建插件链
chain_manager.createChain(config);

// 执行插件链
auto input_data = std::make_shared<BatchData>("device001", std::chrono::system_clock::now());
auto output_result = std::make_shared<PluginResultImpl>();
chain_manager.executeChain("my_chain", input_data, output_result);
```

### 4. 配置文件使用

```json
{
  "device_configs": {
    "device001": {
      "description": "测试设备001",
      "device_type": "motor",
      "plugin_chain": "vibration_monitoring",
      "parameters": {
        "sampling_rate": 1000,
        "device_id": "device001"
      }
    }
  },
  "plugin_chains": {
    "vibration_monitoring": {
      "plugins": [
        {"name": "vibrate31", "type": "feature"},
        {"name": "motor97", "type": "decision"},
        {"name": "comp_realtime_health34", "type": "evaluation"},
        {"name": "score_alarm5", "type": "event"}
      ]
    }
  }
}
```

## 性能优化

### 1. 内存管理
- 使用智能指针自动管理内存
- 避免不必要的数据拷贝
- 使用对象池减少内存分配

### 2. 并发处理
- 支持多线程安全执行
- 使用线程局部存储
- 避免锁竞争

### 3. 算法优化
- 使用高效的数学库（如FFTW）
- 优化数据结构访问模式
- 减少函数调用开销

## 测试

### 运行测试

```bash
# 运行所有测试
./bin/plugin_tests

# 运行特定测试
./bin/plugin_tests --gtest_filter="PluginBaseTest.*"

# 生成测试报告
./bin/plugin_tests --gtest_output=xml:test_results.xml
```

### 测试覆盖率

```bash
# 使用gcov生成覆盖率报告
cmake .. -DCMAKE_BUILD_TYPE=Debug -DENABLE_COVERAGE=ON
make
./bin/plugin_tests
gcov src/*.cpp
```

## 部署

### 1. 安装

```bash
# 安装到系统目录
make install

# 或者使用包管理器
cpack
```

### 2. 配置

```bash
# 复制配置文件
cp config/plugin_config.json /etc/algorithm_plugins/

# 设置环境变量
export ALGORITHM_PLUGINS_CONFIG=/etc/algorithm_plugins/plugin_config.json
```

### 3. 服务化部署

```bash
# 创建systemd服务文件
sudo cp scripts/algorithm_plugins.service /etc/systemd/system/
sudo systemctl enable algorithm_plugins
sudo systemctl start algorithm_plugins
```

## 故障排除

### 常见问题

1. **编译错误**
   - 检查CMake版本是否满足要求
   - 确认编译器支持C++17标准
   - 检查依赖库是否正确安装

2. **运行时错误**
   - 检查配置文件路径是否正确
   - 确认插件库文件存在
   - 查看日志文件获取详细错误信息

3. **性能问题**
   - 使用性能分析工具（如perf、valgrind）
   - 检查内存使用情况
   - 优化算法实现

### 调试模式

```bash
# 启用调试模式
cmake .. -DCMAKE_BUILD_TYPE=Debug -DENABLE_DEBUG=ON
make
./bin/plugin_example --debug
```

## 贡献指南

### 开发流程

1. Fork项目
2. 创建特性分支
3. 编写代码和测试
4. 提交Pull Request

### 代码规范

- 遵循Google C++ Style Guide
- 使用clang-format格式化代码
- 编写完整的单元测试
- 添加详细的文档注释

### 提交规范

```
feat: 添加新功能
fix: 修复bug
docs: 更新文档
style: 代码格式调整
refactor: 代码重构
test: 添加测试
chore: 构建过程或辅助工具的变动
```

## 许可证

本项目采用MIT许可证，详见LICENSE文件。

## 联系方式

- 项目维护者: Algorithm Team
- 邮箱: algorithm@company.com
- 问题反馈: 请使用GitHub Issues

## 更新日志

### v1.0.0 (2024-01-01)
- 初始版本发布
- 支持基础插件框架
- 实现核心算法插件
- 提供完整的测试和文档
