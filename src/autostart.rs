use auto_launch::{AutoLaunch, AutoLaunchBuilder, Result};
use std::env;
use std::path::PathBuf;

/// 获取 AutoLaunch 实例
fn get_auto_launch() -> Result<AutoLaunch> {
    // 获取当前可执行文件的绝对路径
    let exe_path: PathBuf = env::current_exe()?;
    let app_name = "SimpleLogin";

    // 构建 AutoLaunch 实例
    let auto = AutoLaunchBuilder::new()
        .set_app_name(app_name)
        .set_app_path(exe_path.to_str().unwrap())
        .set_macos_launch_mode(auto_launch::MacOSLaunchMode::LaunchAgent)
        .build()?;

    Ok(auto)
}

/// 启用开机自启
pub fn enable_autostart() -> Result<()> {
    let auto = get_auto_launch()?;
    auto.enable()
}

/// 禁用开机自启
pub fn disable_autostart() -> Result<()> {
    let auto = get_auto_launch()?;
    auto.disable()
}

/// 检查当前是否已开启自启
pub fn is_autostart_enabled() -> Result<bool> {
    let auto = get_auto_launch()?;
    auto.is_enabled()
}