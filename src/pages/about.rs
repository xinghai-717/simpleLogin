use std::time::{Duration, Instant};
use eframe::egui;
use crate::update_manager::{UpdateManager, UpdateState};
use crate::widgets::{ToastLevel, ToastManager};

pub struct AboutPage {
    click_count: u32,
    last_click_time: Option<Instant>,
    is_egg_unlocked: bool,
}

impl Default for AboutPage {
    fn default() -> Self {
        Self {
            click_count: 0,
            last_click_time: None,
            is_egg_unlocked: false,
        }
    }
}

impl AboutPage {
    pub fn ui(&mut self, ui: &mut egui::Ui, toast_manager: &mut ToastManager, update_manager: &mut UpdateManager) {
        ui.heading("关于");
        ui.separator();

        // --- 彩蛋逻辑 ---
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
            ui.hyperlink_to("星海", "https://github.com/xinghai-717");
        });
        ui.horizontal(|ui| {
            ui.add(egui::Label::new("源代码仓库:").selectable(false));
            ui.hyperlink_to("Github", "https://github.com/xinghai-717/simpleLogin");
        });
        ui.horizontal(|ui| {
            ui.add(egui::Label::new("开源协议:").selectable(false));
            ui.hyperlink_to("GPLv3", "https://www.gnu.org/licenses/gpl-3.0.html");
        });

        // 如果彩蛋解锁了，展示特殊内容
        if self.is_egg_unlocked {
            ui.add_space(10.0);
            ui.colored_label(egui::Color32::from_rgb(255, 165, 0), "✨ You found the secret! ✨");
            ui.label("感谢你使用 SimpleLogin");
        }

        ui.add_space(10.0);

        match &update_manager.state {
            UpdateState::Idle => {
                if ui.button("检查更新").clicked() {
                    // 手动模式传 false
                    update_manager.start_check(false);
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
                    update_manager.cancel_flag.store(true, std::sync::atomic::Ordering::Relaxed);
                }
            }
            UpdateState::ReadyToRestart => {
                ui.colored_label(egui::Color32::GREEN, "更新已下载完成！");
                ui.add_space(10.0);

                // 区分自动模式与手动模式
                if update_manager.is_auto_restart {
                    // 自动模式：主线程正在倒计时，这里只显示转圈，绝不提供按钮，防止抢占 pending_update
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label("程序将在 3 秒后自动重启...");
                    });
                } else {
                    // 手动模式：等待用户主动点击
                    ui.label("是否立即重启以应用更新？");
                    if ui.button("立即重启").clicked() {
                        if let Some(update) = update_manager.pending_update.take() {
                            let _ = update.restart_with(|ctx| {
                                ctx.before_exit(|| {});
                            });
                        }
                    }
                }
            }
            UpdateState::UpToDate => {
                ui.label("当前已是最新版本。");
                if ui.button("再次检查").clicked() {
                    update_manager.state = UpdateState::Idle;
                }
            }
            UpdateState::Error(e) => {
                ui.colored_label(egui::Color32::RED, format!("更新失败: {}", e));
                if ui.button("重试").clicked() {
                    update_manager.state = UpdateState::Idle;
                }
            }
        }
    }
}