use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::smp::crypto::SecretFormat;

/// SMP 认证配置。存到用户配置目录下的 smp.toml。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SmpConfig {
    pub userlist: Vec<String>,
    pub username: String,
    pub secret_format: SecretFormat,
    /// 登录成功后保存的在线字段，供 logout 使用
    pub online_fields: Option<HashMap<String, String>>,
    pub auto_login: bool,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_and_load() {
        let cfg = SmpConfig {
            userlist: vec![],
            username: "20230001".into(),
            secret_format: SecretFormat::PwdMac,
            online_fields: Some(HashMap::from([
                ("userIndex".into(), "42".into()),
                ("loginUniqueCodeForOnline".into(), "abc".into()),
            ])),
            auto_login: false,
        };
        save_config(&cfg).expect("saving config error");
        let back: SmpConfig = load_config();
        let path_str = config_path()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();
        std::fs::remove_file(path_str).expect("remove file error");
        assert_eq!(back.username, cfg.username);
        assert_eq!(back.secret_format, SecretFormat::PwdMac);
        assert_eq!(back.online_fields.as_ref().unwrap().get("userIndex").unwrap(), "42");
    }
}