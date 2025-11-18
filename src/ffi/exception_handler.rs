//! 异常处理模块
//!
//! 提供跨语言异常捕获、翻译和处理功能

use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// 异常处理器
pub struct ExceptionHandler {
    /// 异常记录
    exceptions: Arc<RwLock<HashMap<String, ExceptionRecord>>>,
    /// 错误翻译器
    error_translator: Arc<ErrorTranslator>,
    /// 结果处理器
    result_processor: Arc<ResultProcessor>,
    /// 统计信息
    stats: Arc<RwLock<ExceptionStats>>,
}

#[derive(Debug, Clone)]
pub struct ExceptionRecord {
    /// 异常ID
    pub id: String,
    /// 异常类型
    pub exception_type: String,
    /// 原始错误信息
    pub original_message: String,
    /// 翻译后的错误信息
    pub translated_message: String,
    /// 时间戳
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// 是否已处理
    pub is_handled: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ExceptionStats {
    /// 总异常数
    pub total_exceptions: usize,
    /// 已处理异常数
    pub handled_exceptions: usize,
    /// 未处理异常数
    pub unhandled_exceptions: usize,
    /// 异常处理成功率
    pub success_rate: f64,
}

impl ExceptionHandler {
    /// 创建新的异常处理器
    pub fn new() -> Self {
        Self {
            exceptions: Arc::new(RwLock::new(HashMap::new())),
            error_translator: ErrorTranslator::new(),
            result_processor: ResultProcessor::new(),
            stats: Arc::new(RwLock::new(ExceptionStats::default())),
        }
    }

    /// 捕获C++异常
    pub async fn catch_cpp_exception(&self, exception: &str) -> Result<String, String> {
        let exception_id = format!("cpp_exception_{}", chrono::Utc::now().timestamp_millis());

        let record = ExceptionRecord {
            id: exception_id.clone(),
            exception_type: "CppException".to_string(),
            original_message: exception.to_string(),
            translated_message: String::new(),
            timestamp: chrono::Utc::now(),
            is_handled: false,
        };

        // 翻译异常信息
        let translated = self.error_translator.translate_cpp_error(exception).await?;

        let mut record = record;
        record.translated_message = translated.clone();

        // 记录异常
        let mut exceptions = self.exceptions.write().await;
        exceptions.insert(exception_id.clone(), record);

        // 更新统计信息
        let mut stats = self.stats.write().await;
        stats.total_exceptions += 1;

        tracing::warn!("Caught C++ exception: {}", exception);
        Ok(translated)
    }

    /// 获取错误翻译器
    pub fn get_error_translator(&self) -> &ErrorTranslator {
        &self.error_translator
    }

    /// 获取结果处理器
    pub fn get_result_processor(&self) -> &ResultProcessor {
        &self.result_processor
    }

    /// 处理异常并生成结果
    pub async fn handle_exception(&self, exception_id: &str) -> Result<ExceptionResult, String> {
        let mut exceptions = self.exceptions.write().await;

        if let Some(record) = exceptions.get_mut(exception_id) {
            if record.is_handled {
                return Err(format!("Exception {} already handled", exception_id));
            }

            record.is_handled = true;

            // 使用结果处理器生成处理结果
            let result = self.result_processor.process_error_result(&record.translated_message).await?;

            // 更新统计信息
            let mut stats = self.stats.write().await;
            stats.handled_exceptions += 1;
            stats.success_rate = stats.handled_exceptions as f64 / stats.total_exceptions as f64;

            tracing::info!("Handled exception: {}", exception_id);
            Ok(result)
        } else {
            Err(format!("Exception {} not found", exception_id))
        }
    }

    /// 获取异常统计信息
    pub async fn get_exception_stats(&self) -> ExceptionStats {
        self.stats.read().await.clone()
    }

    /// 获取所有异常记录
    pub async fn get_all_exceptions(&self) -> HashMap<String, ExceptionRecord> {
        let exceptions = self.exceptions.read().await;
        exceptions.clone()
    }

    /// 清理已处理的异常
    pub async fn cleanup_handled_exceptions(&self) -> usize {
        let mut exceptions = self.exceptions.write().await;
        let mut to_remove = Vec::new();

        for (id, record) in exceptions.iter() {
            if record.is_handled {
                to_remove.push(id.clone());
            }
        }

        let removed_count = to_remove.len();
        for id in to_remove {
            exceptions.remove(&id);
        }

        tracing::info!("Cleaned up {} handled exceptions", removed_count);
        removed_count
    }
}

/// 错误翻译器
pub struct ErrorTranslator {
    /// 翻译映射表
    translation_map: Arc<RwLock<HashMap<String, String>>>,
}

impl ErrorTranslator {
    /// 创建新的错误翻译器
    pub fn new() -> Self {
        let mut map = HashMap::new();

        // 初始化常见的C++错误翻译
        map.insert("std::bad_alloc".to_string(), "内存分配失败".to_string());
        map.insert("std::out_of_range".to_string(), "数组索引超出范围".to_string());
        map.insert("std::invalid_argument".to_string(), "无效的参数".to_string());
        map.insert("std::runtime_error".to_string(), "运行时错误".to_string());
        map.insert("std::logic_error".to_string(), "逻辑错误".to_string());

        Self {
            translation_map: Arc::new(RwLock::new(map)),
        }
    }

    /// 翻译C++错误信息
    pub async fn translate_cpp_error(&self, error: &str) -> Result<String, String> {
        let translations = self.translation_map.read().await;

        // 首先尝试精确匹配
        if let Some(translation) = translations.get(error) {
            return Ok(translation.clone());
        }

        // 尝试模糊匹配
        for (pattern, translation) in translations.iter() {
            if error.contains(pattern) {
                return Ok(format!("{}: {}", translation, error));
            }
        }

        // 如果没有找到匹配，使用默认翻译
        Ok(format!("C++错误: {}", error))
    }

    /// 添加自定义翻译规则
    pub async fn add_translation(&self, cpp_error: &str, translated_error: &str) {
        let mut translations = self.translation_map.write().await;
        translations.insert(cpp_error.to_string(), translated_error.to_string());
    }

    /// 获取所有翻译规则
    pub async fn get_all_translations(&self) -> HashMap<String, String> {
        let translations = self.translation_map.read().await;
        translations.clone()
    }
}

/// 结果处理器
pub struct ResultProcessor {
    /// 处理统计
    stats: Arc<RwLock<ProcessorStats>>,
}

#[derive(Debug, Clone, Default)]
pub struct ProcessorStats {
    /// 总处理次数
    pub total_processed: usize,
    /// 成功处理次数
    pub successful_processed: usize,
    /// 平均处理时间
    pub avg_processing_time_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExceptionResult {
    /// 结果类型
    pub result_type: String,
    /// 错误信息
    pub error_message: String,
    /// 建议的操作
    pub suggested_action: String,
    /// 是否可重试
    pub can_retry: bool,
    /// 处理时间戳
    pub processed_at: chrono::DateTime<chrono::Utc>,
}

impl ResultProcessor {
    /// 创建新的结果处理器
    pub fn new() -> Self {
        Self {
            stats: Arc::new(RwLock::new(ProcessorStats::default())),
        }
    }

    /// 处理错误结果
    pub async fn process_error_result(&self, error_message: &str) -> Result<ExceptionResult, String> {
        let start_time = std::time::Instant::now();

        let result = if error_message.contains("内存分配失败") {
            ExceptionResult {
                result_type: "MemoryError".to_string(),
                error_message: error_message.to_string(),
                suggested_action: "检查系统内存使用情况，释放不必要的内存".to_string(),
                can_retry: true,
                processed_at: chrono::Utc::now(),
            }
        } else if error_message.contains("无效的参数") {
            ExceptionResult {
                result_type: "ArgumentError".to_string(),
                error_message: error_message.to_string(),
                suggested_action: "检查输入参数的有效性".to_string(),
                can_retry: false,
                processed_at: chrono::Utc::now(),
            }
        } else if error_message.contains("数组索引超出范围") {
            ExceptionResult {
                result_type: "IndexError".to_string(),
                error_message: error_message.to_string(),
                suggested_action: "检查数组访问的边界条件".to_string(),
                can_retry: false,
                processed_at: chrono::Utc::now(),
            }
        } else {
            ExceptionResult {
                result_type: "GenericError".to_string(),
                error_message: error_message.to_string(),
                suggested_action: "请查看详细的错误日志".to_string(),
                can_retry: true,
                processed_at: chrono::Utc::now(),
            }
        };

        let processing_time = start_time.elapsed().as_millis() as f64;

        // 更新统计信息
        let mut stats = self.stats.write().await;
        stats.total_processed += 1;
        stats.successful_processed += 1;
        stats.avg_processing_time_ms = (stats.avg_processing_time_ms * (stats.total_processed as f64 - 1.0) + processing_time) / stats.total_processed as f64;

        Ok(result)
    }

    /// 处理成功结果
    pub async fn process_success_result(&self, data: serde_json::Value) -> Result<ExceptionResult, String> {
        let start_time = std::time::Instant::now();

        let result = ExceptionResult {
            result_type: "Success".to_string(),
            error_message: "".to_string(),
            suggested_action: "操作成功完成".to_string(),
            can_retry: false,
            processed_at: chrono::Utc::now(),
        };

        let processing_time = start_time.elapsed().as_millis() as f64;

        let mut stats = self.stats.write().await;
        stats.total_processed += 1;
        stats.successful_processed += 1;
        stats.avg_processing_time_ms = (stats.avg_processing_time_ms * (stats.total_processed as f64 - 1.0) + processing_time) / stats.total_processed as f64;

        Ok(result)
    }

    /// 获取处理器统计信息
    pub async fn get_processor_stats(&self) -> ProcessorStats {
        self.stats.read().await.clone()
    }
}

impl Default for ExceptionHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for ErrorTranslator {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for ResultProcessor {
    fn default() -> Self {
        Self::new()
    }
}
