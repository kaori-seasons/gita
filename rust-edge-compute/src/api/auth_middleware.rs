//! 认证和授权中间件

use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

use rust_edge_compute_core::UserSession;

use super::handlers::AppState;

/// 认证中间件
pub async fn auth_middleware(
    State(_state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    // 获取请求头中的授权信息
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    if let Some(auth_value) = auth_header {
        if let Some(token) = auth_value.strip_prefix("Bearer ") {
            // 移除 "Bearer " 前缀

            // 这里应该验证JWT token
            // 暂时只做简单的token检查
            if validate_token(token).await {
                // 将用户信息添加到请求扩展中
                let user_session = create_mock_session();
                request.extensions_mut().insert(user_session);
            } else {
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(json!({
                        "error": "Invalid token",
                        "message": "The provided authentication token is invalid"
                    })),
                )
                    .into_response();
            }
        } else {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "error": "Invalid authorization header",
                    "message": "Authorization header must start with 'Bearer '"
                })),
            )
                .into_response();
        }
    } else {
        // 如果没有提供token，创建匿名会话
        let anonymous_session = create_anonymous_session();
        request.extensions_mut().insert(anonymous_session);
    }

    // 继续处理请求
    next.run(request).await
}

/// 速率限制中间件
pub async fn rate_limit_middleware(
    State(_state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    // 获取客户端IP
    let _client_ip = extract_client_ip(&request);

    // 这里应该检查速率限制
    // 暂时跳过速率限制检查

    next.run(request).await
}

/// 安全头中间件
pub async fn security_headers_middleware(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;

    // 添加安全头
    let headers = response.headers_mut();
    // 这些是常量安全值，在构造时不会失败
    headers.insert(
        "X-Content-Type-Options",
        "nosniff"
            .parse()
            .expect("X-Content-Type-Options header value is always valid"),
    );
    headers.insert(
        "X-Frame-Options",
        "DENY"
            .parse()
            .expect("X-Frame-Options header value is always valid"),
    );
    headers.insert(
        "X-XSS-Protection",
        "1; mode=block"
            .parse()
            .expect("X-XSS-Protection header value is always valid"),
    );
    headers.insert(
        "Strict-Transport-Security",
        "max-age=31536000; includeSubDomains"
            .parse()
            .expect("Strict-Transport-Security header value is always valid"),
    );
    headers.insert(
        "Content-Security-Policy",
        "default-src 'self'"
            .parse()
            .expect("Content-Security-Policy header value is always valid"),
    );

    response
}

/// 验证JWT token（模拟实现）
async fn validate_token(_token: &str) -> bool {
    // 这里应该实现真正的JWT验证
    // 暂时返回true，允许所有token
    true
}

/// 创建模拟用户会话
fn create_mock_session() -> UserSession {
    UserSession::new("mock_user_123".to_string(), "mock_user".to_string())
}

/// 创建匿名用户会话
fn create_anonymous_session() -> UserSession {
    UserSession::new("anonymous".to_string(), "anonymous".to_string())
}

/// 从请求中提取客户端IP
fn extract_client_ip(request: &Request) -> Option<String> {
    // 尝试从X-Forwarded-For头获取
    if let Some(forwarded_for) = request.headers().get("X-Forwarded-For") {
        if let Ok(forwarded_str) = forwarded_for.to_str() {
            return Some(forwarded_str.split(',').next()?.trim().to_string());
        }
    }

    // 尝试从X-Real-IP头获取
    if let Some(real_ip) = request.headers().get("X-Real-IP") {
        if let Ok(real_ip_str) = real_ip.to_str() {
            return Some(real_ip_str.to_string());
        }
    }

    // 从连接信息获取（在实际部署中可能不可用）
    None
}

/// CORS中间件
pub async fn cors_middleware(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;

    let headers = response.headers_mut();
    // 这些是常量安全值，在构造时不会失败
    headers.insert(
        "Access-Control-Allow-Origin",
        "*".parse()
            .expect("CORS origin header value is always valid"),
    );
    headers.insert(
        "Access-Control-Allow-Methods",
        "GET, POST, PUT, DELETE, OPTIONS"
            .parse()
            .expect("CORS methods header value is always valid"),
    );
    headers.insert(
        "Access-Control-Allow-Headers",
        "Content-Type, Authorization"
            .parse()
            .expect("CORS headers value is always valid"),
    );

    response
}

/// 登录处理器
pub async fn login(
    State(_state): State<AppState>,
    axum::extract::Json(payload): axum::extract::Json<serde_json::Value>,
) -> impl IntoResponse {
    // 简单的登录实现
    let username = payload
        .get("username")
        .and_then(|v| v.as_str())
        .unwrap_or("anonymous");

    let password = payload
        .get("password")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    // 这里应该验证用户名和密码
    // 暂时接受任何用户名/密码组合

    if !username.is_empty() && !password.is_empty() {
        // 生成JWT token（模拟）
        let token = format!("jwt_token_for_{}", username);

        (
            StatusCode::OK,
            Json(json!({
                "token": token,
                "user": {
                    "id": "user123",
                    "username": username,
                    "roles": ["user"],
                    "permissions": ["execute:add", "execute:multiply"]
                },
                "expires_in": 3600
            })),
        )
            .into_response()
    } else {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "Invalid credentials",
                "message": "Username and password are required"
            })),
        )
            .into_response()
    }
}

/// 获取当前用户信息
pub async fn get_current_user(
    State(_state): State<AppState>,
    axum::extract::Extension(session): axum::extract::Extension<UserSession>,
) -> impl IntoResponse {
    Json(json!({
        "user": {
            "id": session.user_id,
            "username": session.username,
            "roles": session.roles,
            "permissions": session.permissions
        },
        "session": {
            "created_at": session.created_at,
            "expires_at": session.expires_at
        }
    }))
}

/// 注销处理器
pub async fn logout() -> impl IntoResponse {
    Json(json!({
        "message": "Logged out successfully"
    }))
}
