use std::time::{Duration, Instant};
use eframe::egui;
use eframe::glow::Context;
use sysinfo::{get_current_pid, System, ProcessesToUpdate, ProcessRefreshKind, Pid};
use simple_login::smp::{do_logout, save_config, AuthStatus};
use crate::logger::LogBuffer;
use crate::widgets::*;
use crate::pages::about::AboutPage;
use crate::pages::account::AccountPage;
use crate::pages::home::HomePage;
use crate::pages::logs::LogPage;
use crate::pages::settings::SettingsPage;
use crate::pages::Page;
use crate::theme::AppTheme;
use crate::smp::{SmpConfig, load_config, make_client, do_status};
use reqwest::blocking::Client;
use tray_icon::menu::MenuEvent;
use tray_icon::{MouseButton, MouseButtonState, TrayIconEvent};
use crate::autostart;
use crate::tray::TrayApp;

pub struct MyApp {
    tray_app: TrayApp,
    is_window_visible: bool,
    is_quitting: bool,
    sys: System,
    pid: Pid,
    memory: Option<u64>,
    last_memory_refresh: Instant,
    last_network_refresh: Instant,
    memory_refresh_interval: Duration,
    network_refresh_interval: Duration,
    app_theme: AppTheme,
    smp_config: SmpConfig,
    toast_manager: ToastManager,
    current_page: Page,
    home: HomePage,
    account: AccountPage,
    settings: SettingsPage,
    log: LogPage,
    about: AboutPage,
    log_buffer: LogBuffer,
    client: Client,
    status_rx: Option<std::sync::mpsc::Receiver<AuthStatus>>,
}

impl MyApp {
    pub fn new(
        cc: &eframe::CreationContext,
        fonts: egui::FontDefinitions,
        log_buffer: LogBuffer,
        tray_app: TrayApp,
    ) -> Self {
        let app_theme = cc.storage
            .and_then(|s| eframe::get_value(s, "app_theme"))
            .unwrap_or(AppTheme::System);
        let memory_refresh_interval = cc.storage
            .and_then(|s| eframe::get_value(s, "memory_refresh_interval"))
            .unwrap_or(Duration::from_millis(500));
        let network_refresh_interval = cc.storage
            .and_then(|s| eframe::get_value(s, "network_refresh_interval"))
            .unwrap_or(Duration::from_secs(5));

        let config = load_config();
        cc.egui_ctx.set_theme(match app_theme {
            AppTheme::System => egui::ThemePreference::System,
            AppTheme::Light => egui::ThemePreference::Light,
            AppTheme::Dark => egui::ThemePreference::Dark,
        });
        cc.egui_ctx.set_fonts(fonts);
        cc.egui_ctx.set_zoom_factor(1.2);

        let mut sys = System::new().unwrap();
        let pid = get_current_pid().ok().expect("无法读取当前PID");
        sys.refresh_processes_specifics(
            ProcessesToUpdate::Some(&[pid]),
            true,
            ProcessRefreshKind::nothing().with_memory());
        let memory = sys.process(pid).map(|p| p.memory());
        let client = make_client();

        Self {
            tray_app,
            is_window_visible: true,
            is_quitting: false,
            sys,
            pid,
            memory,
            last_memory_refresh: Instant::now(),
            last_network_refresh: Instant::now(),
            memory_refresh_interval,
            network_refresh_interval,
            app_theme,
            smp_config: config.clone(),
            toast_manager: ToastManager::new(),
            current_page: Page::default(),
            home: HomePage::new(&client),
            account: AccountPage::new(&config),
            settings: SettingsPage::default(),
            log: LogPage::default(),
            about: AboutPage::default(),
            log_buffer,
            client,
            status_rx: None,
        }
    }

    fn refresh_memory_if_needed(&mut self) {
        if self.last_memory_refresh.elapsed() < self.memory_refresh_interval {
            return;
        }
        self.sys.refresh_processes_specifics(
            ProcessesToUpdate::Some(&[self.pid]),
            true,
            ProcessRefreshKind::nothing().with_memory());
        self.memory = self.sys.process(self.pid).map(|p| p.memory());
        self.last_memory_refresh = Instant::now();
    }

    // 👈 核心优化：将 do_status 移入后台线程，防止卡死
    fn refresh_network_status_async(&mut self, ctx: &egui::Context) {
        // 如果正在登录/登出，暂停自动轮询，避免状态冲突
        if self.home.is_processing {
            return;
        }

        if self.last_network_refresh.elapsed() < self.network_refresh_interval {
            return;
        }

        // 如果没有正在跑的检测，就起一个
        if self.status_rx.is_none() {
            let client = self.client.clone();
            let (tx, rx) = std::sync::mpsc::channel();
            self.status_rx = Some(rx);
            self.last_network_refresh = Instant::now();

            std::thread::spawn(move || {
                let status = do_status(&client);
                let _ = tx.send(status);
            });
        }

        // 轮询是否拿到了结果
        if let Some(rx) = &self.status_rx {
            if let Ok(new_status) = rx.try_recv() {
                self.home.status = Some(new_status);
                self.status_rx = None;
            } else {
                // 等待结果期间，高频重绘
                ctx.request_repaint_after(Duration::from_millis(100));
            }
        }
    }
}

impl eframe::App for MyApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.cumulative_pass_nr() == 1 {
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        }
        let resolved = self.app_theme.get_theme(ctx);
        if ctx.global_style().visuals.dark_mode != matches!(resolved, egui::Theme::Dark) {
            let system_theme = match resolved {
                egui::Theme::Light => egui::viewport::SystemTheme::Light,
                egui::Theme::Dark => egui::viewport::SystemTheme::Dark,
            };
            ctx.set_theme(resolved);
            ctx.send_viewport_cmd(egui::ViewportCommand::SetTheme(system_theme));
        }

        self.refresh_memory_if_needed();
        self.refresh_network_status_async(ctx);

        // 检查是否有“关闭窗口”的请求
        if ctx.input(|i| i.viewport().close_requested()) && !self.is_quitting {
            // 发送 CancelClose，阻止程序退出
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);

            // 隐藏主窗口（实现“最小化到托盘”）
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));

            // 记录窗口状态为隐藏（供托盘逻辑使用）
            self.is_window_visible = false;
        }

        // 处理托盘菜单点击事件
        if let Ok(event) = MenuEvent::receiver().try_recv() {
            if event.id == self.tray_app.quit_item.id() {
                // 点击了“退出”
                self.is_quitting = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close)
            } else if event.id == self.tray_app.show_item.id() {
                // 点击了“显示主界面”
                self.is_window_visible = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            }
        }

        // 处理托盘图标点击事件（左键点击切换窗口显示/隐藏）
        if let Ok(event) = TrayIconEvent::receiver().try_recv() {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                self.is_window_visible = !self.is_window_visible;
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(self.is_window_visible));
                if self.is_window_visible {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                }
            }
        }

        // 保证 100ms 左右有一帧重绘，让界面保持流畅
        ctx.request_repaint_after(Duration::from_millis(100));
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.toast_manager.render(ui.ctx());

        egui::Panel::left("nav_panel")
            .resizable(false)
            .default_size(140.0)
            .show(ui, |ui| {
                ui.style_mut().interaction.selectable_labels = false;
                ui.add_space(6.0);
                ui.heading("简单登录");
                ui.separator();

                sliding_nav(
                    ui,
                    "main_nav",
                    &mut self.current_page,
                    Page::ALL,
                    120.0,
                    28.0,
                    4.0,
                );
            });

        egui::CentralPanel::default().show(ui, |ui| {
            match self.current_page {
                Page::Home => self.home.ui(
                    ui,
                    &self.client,
                    &mut self.toast_manager,
                    &mut self.smp_config,
                    &self.account.username,
                    &self.account.password
                ),
                Page::Account => self.account.ui(ui, &mut self.toast_manager, &mut self.smp_config, &self.home.status),
                Page::Settings => self.settings.ui(ui, &mut self.app_theme, &self.memory, &mut self.toast_manager),
                Page::Logs => self.log.ui(ui, &self.log_buffer),
                Page::About => self.about.ui(ui),
            }
        });
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, "app_theme", &self.app_theme);
        eframe::set_value(storage, "memory_refresh_interval", &self.memory_refresh_interval);
        eframe::set_value(storage, "network_refresh_interval", &self.network_refresh_interval);

        // 👈 修复：避免配置文件被占用或磁盘满时导致程序直接崩溃
        if let Err(e) = save_config(&self.smp_config) {
            tracing::error!("保存 smp_config 失败: {}", e);
        }
    }

    // 关闭窗口时不能卡 UI，丢进后台去发请求
    fn on_exit(&mut self, _gl: Option<&Context>) {
        if self.smp_config.online_fields.is_some() {
            let config_clone = self.smp_config.clone();
            std::thread::spawn(move || {
                // 给关闭时的请求一个极短超时（比如1秒），确保不拖沓
                let short_client = Client::builder()
                    .timeout(Duration::from_secs(1))
                    .no_proxy()
                    .build();

                if let Ok(client) = short_client {
                    let _ = do_logout(&client, &config_clone);
                }
            });
        }
    }

    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        visuals.panel_fill.to_normalized_gamma_f32()
    }
}