use eframe::egui;
use crate::theme::AppTheme;

#[derive(Default)]
pub struct SettingsPage {

}

impl SettingsPage {
    pub fn ui(&mut self, ui: &mut egui::Ui, app_theme: &mut AppTheme, memory: &Option<u64>) {
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