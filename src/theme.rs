use eframe::egui;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppTheme {
    System,
    Light,
    Dark,
}

impl AppTheme {
    pub fn get_theme(&self,ctx: &egui::Context) -> egui::Theme{
        match self {
            AppTheme::System => ctx.system_theme().unwrap_or(egui::Theme::Light),
            AppTheme::Light => egui::Theme::Light,
            AppTheme::Dark => egui::Theme::Dark,
        }
    }
}