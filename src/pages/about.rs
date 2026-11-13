use std::sync::mpsc::{channel, Receiver};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use eframe::egui;
use shipup::Update;
use crate::updater::run_update_worker;
use crate::widgets::{ToastLevel, ToastManager};

#[derive(Debug, Clone, PartialEq)]
/// 更新状态
pub enum UpdateState {
    /// 空闲
    Idle,
    /// 正在检查更新
    Checking,
    /// 已是最新版本
    UpToDate,
    /// 正在下载 (0.0 - 100.0)
    Downloading { progress: f32 },
    /// 准备重启
    ReadyToRestart,
    /// 出错
    Error(String),
}

/// 传递给后台线程的结果/进度消息
pub enum UpdateMessage {
    State(UpdateState),
    ReadyToRestart(Update),
}

pub struct AboutPage {
    pub update_state: UpdateState,
    pub update_rx: Option<Receiver<UpdateMessage>>,
    pub cancel_flag: Arc<AtomicBool>,
    pending_update: Option<Update>,
    click_count: u32,
    last_click_time: Option<Instant>,
    is_egg_unlocked: bool,
}

impl Default for AboutPage {
    fn default() -> Self {
        Self {
            update_state: UpdateState::Idle,
            update_rx: None,
            cancel_flag: Arc::new(AtomicBool::new(false)),
            pending_update: None,
            click_count: 0,
            last_click_time: None,
            is_egg_unlocked: false,
        }
    }
}

impl AboutPage {
    pub fn ui(&mut self, ui: &mut egui::Ui, toast_manager: &mut ToastManager) {
        ui.heading("关于");
        ui.separator();


        // 将版本号做成可点击的 Label
        let version_text = format!("版本号 v{}", env!("CARGO_PKG_VERSION"));
        let text_color = ui.visuals().text_color();
        let version_label = ui.add(
            egui::Label::new(egui::RichText::new(version_text).color(text_color))
                .sense(egui::Sense::click())
                .selectable(false)
        );

        // 如果点击了版本号
        if version_label.clicked() {
            let now = Instant::now();
            // 如果两次点击间隔超过 1 秒，或者第一次点击，重置计数
            if let Some(last_time) = self.last_click_time {
                if now.duration_since(last_time) > Duration::from_secs(1) {
                    self.click_count = 0;
                }
            }

            self.click_count += 1;
            self.last_click_time = Some(now);

            if self.click_count >= 10 {
                self.click_count = 0;
                self.is_egg_unlocked = true;
                toast_manager.push(ToastLevel::Success, "🎉 恭喜你发现了彩蛋！");
            } else if self.click_count >= 6 {
                toast_manager.push(ToastLevel::Info, format!("还差 {} 次...", 10 - self.click_count));
            }
        };

        ui.horizontal(|ui| {
            ui.add(egui::Label::new("作者:").selectable(false));
            ui.hyperlink_to(
                "星海",
                "https://github.com/xinghai-717"
            );
        });
        ui.horizontal(|ui| {
            ui.add(egui::Label::new("源代码仓库").selectable(false));
            ui.hyperlink_to(
                "Github",
                "https://github.com/xinghai-717/simpleLogin"
            );
        });
        ui.horizontal(|ui| {
            ui.add(egui::Label::new("开源协议:").selectable(false));
            ui.hyperlink_to(
                "GPLv3",
                "https://www.gnu.org/licenses/gpl-3.0.html"
            );
        });

        // 如果彩蛋解锁了，展示特殊内容
        if self.is_egg_unlocked {
            ui.add_space(10.0);
            ui.colored_label(
                egui::Color32::from_rgb(255, 165, 0),
                "✨ You found the secret! ✨"
            );
            ui.label("感谢你使用 SimpleLogin");
        }

        match &self.update_state {
            UpdateState::Idle => {
                if ui.button("检查更新").clicked() {
                    self.update_state = UpdateState::Checking;
                    self.cancel_flag.store(false, Ordering::Relaxed);

                    let (tx, rx) = channel();
                    self.update_rx = Some(rx);

                    // 提取需要的变量传给后台
                    let cancel = self.cancel_flag.clone();

                    std::thread::spawn(move || {
                        // 这就是你刚才问的 shipup 核心逻辑
                        run_update_worker(tx, cancel);
                    });
                }
            }
            UpdateState::Checking => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("正在检查更新...");
                });
            }
            UpdateState::Downloading { progress } => {
                ui.label("正在下载更新并安装...");
                ui.add(egui::ProgressBar::new(*progress / 100.0).show_percentage());
                if ui.button("取消下载").clicked() {
                    self.cancel_flag.store(true, Ordering::Relaxed);
                    self.update_rx = None;
                    self.update_state = UpdateState::Idle;
                    ui.ctx().request_repaint();
                }
            }
            UpdateState::ReadyToRestart => {
                ui.colored_label(egui::Color32::GREEN, "更新已下载并准备就绪！");
                ui.add_space(10.0);
                ui.label("是否立即重启以应用更新？");
                if ui.button("立即重启").clicked() {
                    if let Some(update) = self.pending_update.take() {
                        let _ = update.restart_with(|ctx| {
                            ctx.before_exit(|| {

                            });
                        });
                    }
                }
            }
            UpdateState::UpToDate => {
                ui.label("当前已是最新版本。");
                if ui.button("再次检查").clicked() {
                    self.update_state = UpdateState::Idle;
                }
            }
            UpdateState::Error(e) => {
                ui.colored_label(egui::Color32::RED, format!("更新失败: {}", e));
                if ui.button("重试").clicked() {
                    self.update_state = UpdateState::Idle;
                }
            }
        }

        // === 轮询通道获取后台进度 ===
        if let Some(rx) = &self.update_rx {
            if let Ok(msg) = rx.try_recv() {
                match msg {
                    UpdateMessage::State(state) => {
                        self.update_state = state;
                        if matches!(self.update_state, UpdateState::Error(_) | UpdateState::UpToDate) {
                            self.update_rx = None;
                        }
                    }
                    UpdateMessage::ReadyToRestart(update) => {
                        self.update_state = UpdateState::ReadyToRestart;
                        self.pending_update = Some(update);
                        self.update_rx = None;
                    }
                }
            }
            // 强制界面持续重绘，确保进度条和转圈圈能动
            ui.ctx().request_repaint_after(Duration::from_millis(100));
        }
    }
}