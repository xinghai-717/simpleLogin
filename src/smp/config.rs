use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use crate::smp::crypto::SecretFormat;

/// SMP 认证配置。存到用户配置目录下的 smp.toml。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmpConfig {
    pub userlist: Vec<String>,
    pub username: String,
    pub secret_format: SecretFormat,
    /// 登录成功后保存的在线字段，供 logout 使用
    pub online_fields: Option<HashMap<String, String>>,
    pub auto_login: bool,
    /// 内存刷新时间间隔
    pub memory_refresh_interval_ms: u64,
    /// 网络状态刷新时间间隔
    pub network_refresh_interval_ms: u64,
}

impl Default for SmpConfig {
    fn default() -> Self {
        Self {
            userlist: vec![],
            username: "".to_string(),
            secret_format: Default::default(),
            online_fields: None,
            auto_login: true,
            memory_refresh_interval_ms: 500,
            network_refresh_interval_ms: 5000,
        }
    }
}

/// 配置文件路径
pub fn config_path() -> Option<PathBuf> {
    let dirs = directories::ProjectDirs::from("", "SimpleLogin", "runtime")?;
    Some(dirs.config_dir().join("smp.toml"))
}

/// 读取配置
pub fn load_config() -> SmpConfig {
    let Some(path) = config_path() else {
        tracing::warn!("无法获取配置文件路径，使用默认配置");
        return SmpConfig::default();
    };

    let Ok(text) = std::fs::read_to_string(&path) else {
        tracing::info!("配置文件不存在，使用默认配置: {:?}", path);
        return SmpConfig::default();
    };

    toml::from_str(&text).unwrap_or_else(|e| {
        tracing::error!("配置文件解析失败，请检查 TOML 格式: {}", e);
        SmpConfig::default()
    })
}

/// 保存配置
pub fn save_config(config: &SmpConfig) -> std::io::Result<()>{
    let path = config_path()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "无法确定配置目录"))?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let text = toml::to_string_pretty(config)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(&path, text)
}