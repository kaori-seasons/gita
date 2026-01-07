@0xb3e5629b22d4e3c1;  # 全局唯一ID (使用随机生成)

# ============================================================================
# 核心数据结构
# ============================================================================

# 算法请求
struct AlgorithmRequest {
  id @0 :Text;                      # 请求唯一标识
  algorithmName @1 :Text;          # 算法/插件名称 (如 "vibrate31")
  pluginType @2 :PluginType;       # 插件类型 (FEATURE/DECISION/EVALUATION/EVENT)
  parametersJson @3 :Text;         # 参数 (JSON格式)
  deviceId @4 :Text;               # 设备标识
  timestampMs @5 :UInt64;          # 请求时间戳 (毫秒)
  priority @6 :UInt8;              # 优先级 (0-255)
}

# 算法响应
struct AlgorithmResponse {
  id @0 :Text;                      # 对应的请求ID
  success @1 :Bool;                # 执行是否成功
  resultJson @2 :Text;             # 结果 (JSON格式)
  errorMessage @3 :Text;           # 错误信息 (success=false时)
  executionTimeMs @4 :UInt64;      # 执行耗时
  memoryUsedBytes @5 :UInt64;      # 内存使用量
}

# ============================================================================
# 插件元信息
# ============================================================================

enum PluginType {
  feature @0;          # 特征提取 (Vibrate31, CurrentFeature等)
  decision @1;         # 状态识别 (Motor97, MotionType等)
  evaluation @2;       # 健康评估 (Error18, StatusCheck等)
  event @3;            # 事件处理 (ScoreAlarm5, EventDetect等)
  unknown @4;          # 未知类型
}

struct PluginMetadata {
  name @0 :Text;
  version @1 :Text;
  description @2 :Text;
  type @3 :PluginType;
  requiredParameters :group {
    names @4 :List(Text);
  }
  optionalParameters :group {
    names @5 :List(Text);
  }
  features @6 :List(Text);
}

# ============================================================================
# 系统状态和诊断
# ============================================================================

struct SystemMetrics {
  timestamp @0 :UInt64;
  totalCalls @1 :UInt64;
  successfulCalls @2 :UInt64;
  failedCalls @3 :UInt64;
  avgExecutionTimeMs @4 :Float32;
  maxExecutionTimeMs @5 :Float32;
  minExecutionTimeMs @6 :Float32;
  totalMemoryBytes @7 :UInt64;
  cpuUsagePercent @8 :Float32;
}

struct PluginStats {
  pluginName @0 :Text;
  executionCount @1 :UInt64;
  successCount @2 :UInt64;
  errorCount @3 :UInt64;
  avgExecutionTimeMs @4 :Float32;
  lastExecutionTimeMs @5 :UInt64;
  lastErrorMessage @6 :Text;
}

# ============================================================================
# RPC 接口定义
# ============================================================================

interface AlgorithmService {
  # 执行算法
  # 参数: AlgorithmRequest
  # 返回: AlgorithmResponse
  execute @0 (request :AlgorithmRequest) -> (response :AlgorithmResponse);
  
  # 获取所有可用插件列表
  # 返回: 插件元信息列表
  listPlugins @1 () -> (plugins :List(PluginMetadata));
  
  # 获取特定插件的详细信息
  # 参数: 插件名称
  # 返回: 插件元信息
  getPluginInfo @2 (pluginName :Text) -> (metadata :PluginMetadata, found :Bool);
  
  # 加载插件（动态加载）
  # 参数: 插件名称
  # 返回: 是否成功, 错误信息
  loadPlugin @3 (pluginName :Text) -> (success :Bool, error :Text);
  
  # 卸载插件
  # 参数: 插件名称
  # 返回: 是否成功
  unloadPlugin @4 (pluginName :Text) -> (success :Bool, error :Text);
  
  # 获取系统指标
  # 返回: 系统性能指标
  getSystemMetrics @5 () -> (metrics :SystemMetrics);
  
  # 获取特定插件的统计信息
  # 参数: 插件名称
  # 返回: 插件执行统计
  getPluginStats @6 (pluginName :Text) -> (stats :PluginStats, found :Bool);
  
  # 健康检查
  # 返回: 是否健康
  healthCheck @7 () -> (healthy :Bool, message :Text);
  
  # 批量执行（可选）
  # 参数: 多个请求
  # 返回: 多个响应
  executeBatch @8 (requests :List(AlgorithmRequest)) -> 
    (responses :List(AlgorithmResponse));
}
