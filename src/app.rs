use std::time::{Duration, Instant};
use eframe::egui;
use sysinfo::{get_current_pid, System, ProcessesToUpdate, ProcessRefreshKind, Pid};
use crate::pages::about::AboutPage;
use crate::pages::home::HomePage;
use crate::pages::settings::SettingsPage;
use crate::pages::Page;
use crate::theme::AppTheme;

pub struct MyApp {
    sys: System,
    pid: Pid,
    memory: Option<u64>,
    last_refresh: Instant,
    refresh_interval: Duration,
    app_theme: AppTheme,
    current_page: Page,
    home: HomePage,
    settings: SettingsPage,
    about: AboutPage,
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
            current_page: Page::default(),
            home: HomePage::default(),
            settings: SettingsPage::default(),
            about: AboutPage::default(),
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
        egui::Panel::left("nav_panel")
            .resizable(false)
            .default_size(140.0) // 默认左侧宽度
            .show(ui, |ui| {
                ui.style_mut().interaction.selectable_labels = false;
                ui.add_space(10.0);
                ui.heading("简单登录");
                ui.separator();

                for &(page, label) in Page::ALL {
                    let response = ui.add_sized(
                        egui::vec2(120.0, 28.0),
                        egui::Button::selectable(self.current_page == page, label),
                    );
                    if response.clicked() {self.current_page = page;}
                    ui.add_space(4.0);
                }
            });

        // 中央内容区
        egui::CentralPanel::default().show(ui, |ui| {
            match self.current_page {
                Page::Home => self.home.ui(ui),
                Page::Settings => self.settings.ui(ui, &mut self.app_theme, self.memory),
                Page::About => self.about.ui(ui),
            }
        });
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, "app_theme", &self.app_theme);
    }

    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        visuals.panel_fill.to_normalized_gamma_f32()
    }
}