//! API路由定义

use axum::{
    routing::{delete, get, post, put},
    Router,
};

use super::handlers::{health_check, compute_task, get_task_status, list_algorithms, get_scheduler_status, cancel_task, get_error_stats, reset_error_stats, get_database_stats, backup_database, cleanup_expired_data, enable_intelligent_scheduling, disable_intelligent_scheduling, get_intelligent_scheduling_status, get_intelligent_scheduling_stats, AppState};
use super::auth_middleware::{login, get_current_user, logout, auth_middleware, rate_limit_middleware, security_headers_middleware, cors_middleware};
use super::container_handlers::{create_container, get_container_status, stop_container, delete_container, list_containers};

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
        .route("/scheduler/intelligent/enable", post(enable_intelligent_scheduling))
        .route("/scheduler/intelligent/disable", post(disable_intelligent_scheduling))
        .route("/scheduler/intelligent/status", get(get_intelligent_scheduling_status))
        .route("/scheduler/intelligent/stats", get(get_intelligent_scheduling_stats))

        // 错误监控
        .route("/errors/stats", get(get_error_stats))
        .route("/errors/reset", post(reset_error_stats))

        // 数据库管理
        .route("/database/stats", get(get_database_stats))
        .route("/database/backup", post(backup_database))
        .route("/database/cleanup", post(cleanup_expired_data))

        // 容器管理
        .route("/containers", post(create_container))
        .route("/containers", get(list_containers))
        .route("/containers/:container_id", get(get_container_status))
        .route("/containers/:container_id/stop", put(stop_container))
        .route("/containers/:container_id", delete(delete_container))

        .with_state(state);

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
        .layer(
            tower_http::trace::TraceLayer::new_for_http()
                .make_span_with(
                    tower_http::trace::DefaultMakeSpan::new()
                        .level(tracing::Level::INFO),
                )
                .on_response(
                    tower_http::trace::DefaultOnResponse::new()
                        .level(tracing::Level::INFO),
                ),
        )
}
