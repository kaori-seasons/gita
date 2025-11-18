//! 配置管理模块

pub mod settings;

pub use settings::*;

use std::path::Path;

/// 加载配置文件
pub fn load_config<P: AsRef<Path>>(path: P) -> Result<Settings, config::ConfigError> {
    let mut builder = config::Config::builder()
        .add_source(config::File::from(path.as_ref()))
        .add_source(config::Environment::with_prefix("EDGE_COMPUTE"));

    // 如果存在 .env 文件，也加载它
    if Path::new(".env").exists() {
        builder = builder.add_source(config::File::with_name(".env"));
    }

    builder.build()?.try_deserialize()
}

/// 加载默认配置
pub fn load_default_config() -> Result<Settings, config::ConfigError> {
    let builder = config::Config::builder()
        .add_source(config::Environment::with_prefix("EDGE_COMPUTE"));

    builder.build()?.try_deserialize()
}
