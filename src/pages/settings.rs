use eframe::egui;
use tracing::error;
use simple_login::smp::{save_config, SmpConfig};
use crate::autostart;
use crate::theme::AppTheme;
use crate::widgets::{ToastLevel, ToastManager};

pub struct SettingsPage {
    autostart_enabled: bool,
    auto_login_enabled: bool,
}

impl SettingsPage {
    pub fn new(auto_login_enabled: bool) -> Self {
        Self {
            autostart_enabled: autostart::is_autostart_enabled().unwrap_or(false),
            auto_login_enabled
        }
    }
    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        app_theme: &mut AppTheme,
        memory: &Option<u64>,
        toast_manager: &mut ToastManager,
        smp_config: &mut SmpConfig,
    ) {
        let mb = memory
            .map(|b| format!("{:.1} MB", b as f64 / 1024.0 /1024.0))
            .unwrap_or_else(|| "N/A".to_string());

        ui.add(
            egui::Label::new(egui::RichText::new("设置").heading()).selectable(false),
        );
        // 主题选择
        ui.separator();
        ui.horizontal(|ui| {
            ui.label("主题:");
            egui::ComboBox::from_id_salt("theme_combo")
                .selected_text(theme_label(*app_theme))
                .show_ui(ui, |ui| {
                    ui.selectable_value(app_theme, AppTheme::System, "跟随系统");
                    ui.selectable_value(app_theme, AppTheme::Light, "亮色");
                    ui.selectable_value(app_theme, AppTheme::Dark, "暗色");
                });
        });

        if ui.checkbox(&mut self.autostart_enabled, "开机时自动打开").changed() {
            let result = if self.autostart_enabled {
                autostart::enable_autostart()
            } else {
                autostart::disable_autostart()
            };

            match result {
                Ok(_) => {
                    let msg = if self.autostart_enabled { "已开启开机自启" } else { "已关闭开机自启" };
                    toast_manager.push(ToastLevel::Success, msg);
                }
                Err(e) => {
                    let error_msg = format!("设置开机自启失败: {}", e);
                    toast_manager.push(ToastLevel::Error, error_msg);

                    // 如果操作失败，将 UI 状态恢复，防止界面与实际状态不符
                    self.autostart_enabled = !self.autostart_enabled;
                }
            }
        }

        if ui.checkbox(&mut self.auto_login_enabled, "状态守护").changed() {
            smp_config.auto_login = self.auto_login_enabled;
            match save_config(smp_config) {
                Ok(_) => {
                    toast_manager.push(ToastLevel::Success, if self.auto_login_enabled { "状态守护开启成功" } else { "状态守护关闭成功" })
                }
                Err(e) => {
                    toast_manager.push(ToastLevel::Error,if self.auto_login_enabled { "状态守护开启失败" } else { "状态守护关闭失败" });
                    self.auto_login_enabled = !self.auto_login_enabled;
                    error!("守护状态保存失败: {}", e)
                }
            }
        }

        ui.horizontal(|ui| {
            ui.label("内存监控间隔:");

            // 把毫秒转换为秒
            let mut mem_secs = smp_config.memory_refresh_interval_ms as f32 / 1000.0;

            let slider = egui::Slider::new(&mut mem_secs, 0.5..=10.0)
                .step_by(0.5)
                .suffix(" 秒")
                .logarithmic(true);

            if ui.add(slider).changed() {
                smp_config.memory_refresh_interval_ms = (mem_secs * 1000.0) as u64;
                let _ = save_config(smp_config);
            }
        });

        ui.horizontal(|ui| {
            ui.label("网络检测间隔:");

            let mut net_secs = smp_config.network_refresh_interval_ms as f32 / 1000.0;

            let slider = egui::Slider::new(&mut net_secs, 1.0..=60.0)
                .step_by(1.0)
                .suffix(" 秒");

            if ui.add(slider).changed() {
                smp_config.network_refresh_interval_ms = (net_secs * 1000.0) as u64;
                let _ = save_config(smp_config);
            }
        });

        ui.separator();
        ui.label(format!("内存占用: {mb}"));
    }
}

fn theme_label(t: AppTheme) -> &'static str {
    match t {
        AppTheme::System => "跟随系统",
        AppTheme::Light => "亮色",
        AppTheme::Dark => "暗色",
    }
}