use std::time::{Duration, Instant};
use eframe::egui;
use sysinfo::{get_current_pid, System, ProcessesToUpdate, ProcessRefreshKind, Pid};
use crate::theme::AppTheme;

pub struct MyApp {
    sys: System,
    pid: Pid,
    memory: Option<u64>,
    last_refresh: Instant,
    refresh_interval: Duration,
    app_theme: AppTheme,
}

impl MyApp {
    pub fn new(cc: &eframe::CreationContext, fonts: egui::FontDefinitions) -> Self {
        let app_theme = cc.storage
            .and_then(|s| eframe::get_value(s, "app_theme"))
            .unwrap_or(AppTheme::System);
        cc.egui_ctx.set_theme(match app_theme {
            AppTheme::System => {egui::ThemePreference::System}
            AppTheme::Light => {egui::ThemePreference::Light}
            AppTheme::Dark => {egui::ThemePreference::Dark}
        });
        cc.egui_ctx.set_fonts(fonts);

        let mut sys = System::new().unwrap();
        let pid = get_current_pid().ok().expect("无法读取当前PID");
        sys.refresh_processes_specifics(
            ProcessesToUpdate::Some(&[pid]),
            true,
            ProcessRefreshKind::nothing().with_memory());
        let memory = sys.process(pid).map(|p| p.memory());

        Self {
            sys,
            pid,
            memory,
            last_refresh: Instant::now(),
            refresh_interval: Duration::from_millis(500),
            app_theme,
        }
    }

    fn refresh_if_needed(&mut self) {
        if self.last_refresh.elapsed() < self.refresh_interval {
            return;
        }
        self.sys.refresh_processes_specifics(
            ProcessesToUpdate::Some(&[self.pid]),
            true,
            ProcessRefreshKind::nothing().with_memory());
        self.memory = self.sys.process(self.pid).map(|p| p.memory());
        self.last_refresh = Instant::now();
    }
}

impl eframe::App for MyApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.cumulative_pass_nr() == 1 {
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        }
        let resolved = self.app_theme.get_theme(ctx);
        if ctx.global_style().visuals.dark_mode != matches!(resolved,egui::Theme::Dark) {
            let system_theme = match resolved {
                egui::Theme::Light => egui::viewport::SystemTheme::Light,
                egui::Theme::Dark => egui::viewport::SystemTheme::Dark,
            };
            ctx.set_theme(resolved);
            ctx.send_viewport_cmd(egui::ViewportCommand::SetTheme(system_theme));
        }
        self.refresh_if_needed();
        ctx.request_repaint_after(self.refresh_interval);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let mb = self.memory
            .map(|b| format!("{:.1} MB", b as f64 / 1024.0 /1024.0))
            .unwrap_or_else(|| "N/A".to_string());
        ui.label(format!("内存占用: {mb}"));
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, "app_theme", &self.app_theme);
    }

    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        visuals.panel_fill.to_normalized_gamma_f32()
    }
}