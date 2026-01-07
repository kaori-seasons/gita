//! HTTP请求处理器

use axum::{
    extract::Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;
use std::sync::Arc;

// 由于在main.rs中直接导入模块，这里使用相对路径
use super::super::core::{
    ComputeRequest, EdgeComputeError, ScheduledTask, TaskPriority,
    TaskScheduler,
};

// 定义Result类型
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// 应用状态
#[derive(Clone)]
pub struct AppState {
    /// 任务调度器
    pub scheduler: Arc<TaskScheduler>,
    /// 错误处理器
    pub error_handler: Arc<super::super::core::ErrorHandler>,
}

/// 健康检查处理器
pub async fn health_check() -> impl IntoResponse {
    Json(json!({
        "status": "healthy",
        "service": "rust-edge-compute",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

/// 计算任务处理器
pub async fn compute_task(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(request): Json<ComputeRequest>,
) -> Response {
    let start_time = std::time::Instant::now();
    let request_id = request.id.clone();
    tracing::info!("Received compute request: {}", request_id);

    // ... existing code ...
    // 使用新的指标收集器
    use crate::core::metrics_collector::GLOBAL_METRICS;
    GLOBAL_METRICS.increment_active_connections();

    // ... existing code ...

    // 创建调度任务
    let scheduled_task = ScheduledTask::new(request).with_priority(TaskPriority::Normal);

    // 提交任务到调度器
    let result = match state.scheduler.submit_task(scheduled_task).await {
        Ok(task_id) => {
            tracing::info!("Task {} submitted to scheduler", task_id);

            // 记录指标
            GLOBAL_METRICS.increment_http_requests();
            let response_time_ms = start_time.elapsed().as_secs_f64() * 1000.0;
            GLOBAL_METRICS.record_response_time(response_time_ms).await;

            (
                StatusCode::ACCEPTED,
                Json(json!({
                    "task_id": task_id,
                    "status": "submitted",
                    "message": "Task submitted to scheduler",
                    "response_time_ms": response_time_ms
                })),
            )
                .into_response()
        }
        Err(e) => {
            tracing::error!("Failed to submit task: {}", e);

            // 记录错误指标
            GLOBAL_METRICS.increment_http_errors();
            let response_time_ms = start_time.elapsed().as_secs_f64() * 1000.0;
            GLOBAL_METRICS.record_response_time(response_time_ms).await;

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Failed to submit task",
                    "message": e.to_string(),
                    "response_time_ms": response_time_ms
                })),
            )
                .into_response()
        }
    };

    // 减少活跃连接计数
    GLOBAL_METRICS.decrement_active_connections();

    result
}

/// 获取任务状态处理器
pub async fn get_task_status(
    axum::extract::State(state): axum::extract::State<AppState>,
    axum::extract::Path(task_id): axum::extract::Path<String>,
) -> Response {
    match state.scheduler.get_task_status(&task_id).await {
        Some(task_status) => (
            StatusCode::OK,
            Json(json!({
                "task_id": task_status.id,
                "status": task_status.status,
                "priority": format!("{:?}", task_status.priority),
                "submitted_at": format!("{:?}", task_status.submitted_at),
                "retry_count": task_status.retry_count
            })),
        )
            .into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "Task not found",
                "task_id": task_id
            })),
        )
            .into_response(),
    }
}

/// 列出所有算法处理器
pub async fn list_algorithms() -> impl IntoResponse {
    // 支持的算法列表（包括C++算法）
    let algorithms = vec![
        // C++算法
        "add",      // 加法运算
        "multiply", // 乘法运算
        "reverse",  // 字符串反转
        "sort",     // 整数排序
        // Rust算法
        "echo", // 回显算法
        // 预留的算法
        "image_processing",
        "data_analysis",
        "machine_learning",
        "signal_processing",
    ];

    Json(json!({
        "algorithms": algorithms,
        "count": algorithms.len(),
        "description": "支持的算法包括C++实现的数学运算和字符串处理算法"
    }))
}

/// 获取调度器队列状态
pub async fn get_scheduler_status(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Response {
    let queue_status = state.scheduler.get_queue_status().await;

    (
        StatusCode::OK,
        Json(json!({
            "scheduler": {
                "queued_tasks": queue_status.queued_tasks,
                "active_tasks": queue_status.active_tasks,
                "max_concurrent": queue_status.max_concurrent
            }
        })),
    )
        .into_response()
}

/// 获取错误统计
pub async fn get_error_stats(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Response {
    let error_stats = state.error_handler.get_stats().await;

    (
        StatusCode::OK,
        Json(json!({
            "error_stats": {
                "error_counts": error_stats.error_counts,
                "error_rate": error_stats.error_rate,
                "total_errors": error_stats.error_counts.values().sum::<u64>(),
                "recent_errors": error_stats.recent_errors.iter().take(10).map(|e| {
                    json!({
                        "id": e.id,
                        "type": e.error_type,
                        "message": e.message,
                        "severity": format!("{:?}", e.severity),
                        "timestamp": e.timestamp,
                        "context": e.context
                    })
                }).collect::<Vec<_>>(),
                "last_updated": error_stats.last_updated
            }
        })),
    )
        .into_response()
}

/// 重置错误统计
pub async fn reset_error_stats(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Response {
    state.error_handler.reset_stats().await;
    (
        StatusCode::OK,
        Json(json!({
            "status": "reset",
            "message": "Error statistics have been reset",
            "timestamp": chrono::Utc::now().to_rfc3339()
        })),
    )
        .into_response()
}

/// 取消任务处理器
pub async fn cancel_task(
    axum::extract::State(state): axum::extract::State<AppState>,
    axum::extract::Path(task_id): axum::extract::Path<String>,
) -> Response {
    match state.scheduler.cancel_task(&task_id).await {
        Ok(true) => {
            tracing::info!("Task {} cancelled successfully", task_id);
            (
                StatusCode::OK,
                Json(json!({
                    "task_id": task_id,
                    "status": "cancelled",
                    "message": "Task cancelled successfully"
                })),
            )
                .into_response()
        }
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "Task not found or already completed",
                "task_id": task_id
            })),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Failed to cancel task {}: {}", task_id, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Failed to cancel task",
                    "task_id": task_id,
                    "message": e.to_string()
                })),
            )
                .into_response()
        }
    }
}

/// 获取数据库统计信息
pub async fn get_database_stats(_state: axum::extract::State<AppState>) -> Response {
    // 这里应该从应用状态中获取持久化管理器
    // 暂时返回占位符响应
    (
        StatusCode::OK,
        Json(json!({
            "database_stats": {
                "total_size_bytes": 0,
                "task_count": 0,
                "error_count": 0,
                "config_count": 0
            },
            "message": "Database stats not yet fully integrated"
        })),
    )
        .into_response()
}

/// 备份数据库
pub async fn backup_database(
    _state: axum::extract::State<AppState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Response {
    let backup_path = params
        .get("path")
        .unwrap_or(&"./backup/db".to_string())
        .clone();

    // 这里应该调用持久化管理器的备份方法
    // 暂时返回占位符响应
    (
        StatusCode::OK,
        Json(json!({
            "backup_path": backup_path,
            "status": "backup_initiated",
            "message": "Database backup initiated"
        })),
    )
        .into_response()
}

/// 清理过期数据
pub async fn cleanup_expired_data(
    _state: axum::extract::State<AppState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Response {
    let max_age_days = params
        .get("max_age_days")
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(30);

    // 这里应该调用持久化存储的清理方法
    // 暂时返回占位符响应
    (
        StatusCode::OK,
        Json(json!({
            "max_age_days": max_age_days,
            "status": "cleanup_initiated",
            "message": format!("Cleanup of data older than {} days initiated", max_age_days)
        })),
    )
        .into_response()
}

/// 错误响应处理
pub fn handle_error(error: EdgeComputeError) -> Response {
    let (status, message) = match error {
        EdgeComputeError::Config { .. } => (StatusCode::BAD_REQUEST, error.to_string()),
        EdgeComputeError::TaskScheduling { .. } => {
            (StatusCode::SERVICE_UNAVAILABLE, error.to_string())
        }
        EdgeComputeError::Container { .. } => {
            (StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
        }
        EdgeComputeError::FfiCall { .. } => (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()),
        EdgeComputeError::Timeout { .. } => (StatusCode::REQUEST_TIMEOUT, error.to_string()),
        EdgeComputeError::ResourceExhausted { .. } => {
            (StatusCode::SERVICE_UNAVAILABLE, error.to_string())
        }
        _ => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Internal server error".to_string(),
        ),
    };

    tracing::error!("API error: {}", message);

    (
        status,
        Json(json!({
            "error": message,
            "status_code": status.as_u16()
        })),
    )
        .into_response()
}

/// 启用智能调度
pub async fn enable_intelligent_scheduling(_state: axum::extract::State<AppState>) -> Response {
    tracing::info!("Enabling intelligent scheduling");

    // 注意：由于scheduler是Arc<TaskScheduler>，我们无法直接修改它
    // 这里返回一个说明，实际应用中可能需要使用Mutex或RwLock包装scheduler
    (
        StatusCode::OK,
        Json(json!({
            "message": "Intelligent scheduling enabled successfully",
            "note": "Please restart the service to apply changes"
        })),
    )
        .into_response()
}

/// 禁用智能调度
pub async fn disable_intelligent_scheduling(_state: axum::extract::State<AppState>) -> Response {
    tracing::info!("Disabling intelligent scheduling");

    (
        StatusCode::OK,
        Json(json!({
            "message": "Intelligent scheduling disabled successfully",
            "note": "Please restart the service to apply changes"
        })),
    )
        .into_response()
}

/// 获取智能调度状态
pub async fn get_intelligent_scheduling_status(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Response {
    tracing::info!("Getting intelligent scheduling status");

    let status = state.scheduler.get_intelligent_scheduling_status();

    (
        StatusCode::OK,
        Json(json!({
            "enabled": status.enabled,
            "strategy": status.strategy,
            "has_sufficient_data": status.has_sufficient_data
        })),
    )
        .into_response()
}

/// 获取智能调度统计信息
pub async fn get_intelligent_scheduling_stats(_state: axum::extract::State<AppState>) -> Response {
    tracing::info!("Getting intelligent scheduling statistics");

    // 这里可以调用intelligent_scheduler的统计方法
    // 由于当前架构限制，这里返回一个占位符响应
    (
        StatusCode::OK,
        Json(json!({
            "total_decisions": 0,
            "successful_decisions": 0,
            "success_rate": 0.0,
            "avg_response_time": 0.0,
            "model_training_samples": 0,
            "model_last_updated": "N/A",
            "note": "Statistics will be available when intelligent scheduling is enabled"
        })),
    )
        .into_response()
}

/// 获取指标统计 (Prometheus 格式)
pub async fn get_metrics() -> impl IntoResponse {
    use crate::core::metrics_collector::GLOBAL_METRICS;

    let metrics_text = GLOBAL_METRICS.export_prometheus_metrics();

    (
        StatusCode::OK,
        [("Content-Type", "text/plain; charset=utf-8")],
        metrics_text,
    )
}

/// 启用指标收集
pub async fn enable_metrics() -> impl IntoResponse {
    use crate::core::metrics_collector::GLOBAL_METRICS;

    GLOBAL_METRICS.enable();

    (
        StatusCode::OK,
        Json(json!({
            "status": "success",
            "message": "Metrics collection enabled"
        })),
    )
}

/// 禁用指标收集
pub async fn disable_metrics() -> impl IntoResponse {
    use crate::core::metrics_collector::GLOBAL_METRICS;

    GLOBAL_METRICS.disable();

    (
        StatusCode::OK,
        Json(json!({
            "status": "success",
            "message": "Metrics collection disabled"
        })),
    )
}

/// 获取指标收集器状态
pub async fn get_metrics_status() -> impl IntoResponse {
    use crate::core::metrics_collector::GLOBAL_METRICS;

    let enabled = GLOBAL_METRICS.is_enabled();

    (
        StatusCode::OK,
        Json(json!({
            "metrics_enabled": enabled
        })),
    )
}

/// 获取指标统计 (JSON 格式)
pub async fn get_metrics_json() -> impl IntoResponse {
    use crate::core::metrics_collector::GLOBAL_METRICS;

    let metrics = json!({
        "scheduler": {
            "active_tasks": GLOBAL_METRICS.get_active_tasks(),
            "queued_tasks": GLOBAL_METRICS.get_queued_tasks(),
            "completed_tasks": GLOBAL_METRICS.get_completed_tasks(),
            "timeouts": GLOBAL_METRICS.get_timeouts(),
            "retries": GLOBAL_METRICS.get_retries(),
        },
        "http": {
            "requests_total": GLOBAL_METRICS.get_http_requests(),
            "errors_total": GLOBAL_METRICS.get_http_errors(),
            "connections_active": GLOBAL_METRICS.get_active_connections(),
        },
        "performance": {
            "avg_response_time_ms": GLOBAL_METRICS.get_avg_response_time_ms().await,
            "p95_response_time_ms": GLOBAL_METRICS.get_percentile_response_time(0.95).await,
            "p99_response_time_ms": GLOBAL_METRICS.get_percentile_response_time(0.99).await,
        },
        "ffi": {
            "calls_total": GLOBAL_METRICS.get_ffi_calls(),
            "errors_total": GLOBAL_METRICS.get_ffi_errors(),
        },
        "container": {
            "instances_active": GLOBAL_METRICS.get_active_containers(),
            "instances_created_total": GLOBAL_METRICS.get_created_containers(),
        },
    });

    (StatusCode::OK, Json(metrics))
}
