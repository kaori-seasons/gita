//! API模块
//!
//! 提供RESTful API接口，基于Axum框架实现

pub mod handlers;
pub mod routes;
pub mod server;

pub use server::*;
