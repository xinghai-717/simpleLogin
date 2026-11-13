use eframe::egui;
use crate::autostart;
use crate::theme::AppTheme;
use crate::widgets::{ToastLevel, ToastManager};

pub struct SettingsPage {
    autostart_enabled: bool,
}

impl Default for SettingsPage {
    fn default() -> Self {
        Self {
            autostart_enabled: autostart::is_autostart_enabled().unwrap_or(false),
        }
    }
}

impl SettingsPage {
    pub fn ui(&mut self, ui: &mut egui::Ui, app_theme: &mut AppTheme, memory: &Option<u64>, toast_manager: &mut ToastManager) {
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