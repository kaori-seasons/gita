//! API路由定义

use axum::{
    routing::{delete, get, post, put},
    Router,
};

use super::auth_middleware::{
    auth_middleware, cors_middleware, get_current_user, login, logout, rate_limit_middleware,
    security_headers_middleware,
};
use super::container_handlers::{
    delete_container, get_container_status, list_containers, stop_container,
};
use super::ffi_handlers::{execute_cpp_algorithm, get_cpp_algorithm_info, list_cpp_algorithms};
use super::handlers::{
    backup_database, cancel_task, cleanup_expired_data, compute_task,
    disable_intelligent_scheduling, disable_metrics, enable_intelligent_scheduling, enable_metrics,
    get_database_stats, get_error_stats, get_intelligent_scheduling_stats,
    get_intelligent_scheduling_status, get_metrics, get_metrics_json, get_metrics_status,
    get_scheduler_status, get_task_status, health_check, list_algorithms,
    AppState,
};

/// 创建API路由
pub fn create_routes(state: AppState) -> Router {
    let api_routes = Router::new()
        // 健康检查
        .route("/health", get(health_check))
        // 认证相关
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
        .route("/auth/me", get(get_current_user))
        // 计算任务
        .route("/compute", post(compute_task))
        .route("/task/:task_id", get(get_task_status))
        .route("/task/:task_id/cancel", put(cancel_task))
        .route("/algorithms", get(list_algorithms))
        // 调度器管理
        .route("/scheduler/status", get(get_scheduler_status))
        .route(
            "/scheduler/intelligent/enable",
            post(enable_intelligent_scheduling),
        )
        .route(
            "/scheduler/intelligent/disable",
            post(disable_intelligent_scheduling),
        )
        .route(
            "/scheduler/intelligent/status",
            get(get_intelligent_scheduling_status),
        )
        .route(
            "/scheduler/intelligent/stats",
            get(get_intelligent_scheduling_stats),
        )
        // 错误监控
        .route("/errors/stats", get(get_error_stats))
        // TODO: 修复reset_error_stats的Handler trait问题
        // .route("/errors/reset", post(reset_error_stats))
        // 数据库管理
        .route("/database/stats", get(get_database_stats))
        .route("/database/backup", post(backup_database))
        .route("/database/cleanup", post(cleanup_expired_data))
        // 指标管理
        .route("/metrics", get(get_metrics))
        .route("/metrics/json", get(get_metrics_json))
        .route("/metrics/enable", post(enable_metrics))
        .route("/metrics/disable", post(disable_metrics))
        .route("/metrics/status", get(get_metrics_status))
        .route("/containers", get(list_containers))
        .route("/containers/:container_id", get(get_container_status))
        .route("/containers/:container_id/stop", put(stop_container))
        .route("/containers/:container_id", delete(delete_container))
        // C++ FFI 算法
        .route("/cpp/algorithms/execute", post(execute_cpp_algorithm))
        .route("/cpp/algorithms", get(list_cpp_algorithms))
        .route(
            "/cpp/algorithms/:algorithm_name",
            get(get_cpp_algorithm_info),
        )
        .with_state(state.clone());

    // 应用中间件
    Router::new()
        .nest("/api/v1", api_routes)
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            rate_limit_middleware,
        ))
        .layer(axum::middleware::from_fn(security_headers_middleware))
        .layer(axum::middleware::from_fn(cors_middleware))
}
