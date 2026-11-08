use eframe::egui;
use crate::widgets::{ToastLevel,ToastManager};

/// 主页自己的状态。将来可以装登录表单、状态显示等。
#[derive(Default)]
pub struct AccountPage {
    pub username: String,
    pub password: String,
    pub status_text: String,
}

impl AccountPage {
    pub fn ui(&mut self, ui: &mut egui::Ui, toasts: &mut ToastManager) {
        ui.heading("账号");
        ui.separator();

        // 垂直居中容器
        ui.vertical_centered(|ui| {
            ui.add_space(40.0);

            // 表单最大宽度
            let form_width = 280.0;

            // 固定这块区域宽度，并左对齐
            ui.allocate_ui_with_layout(
                egui::vec2(form_width, ui.available_height()),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    egui::Grid::new("login_grid")
                        .num_columns(2)
                        .spacing([4.0, 12.0])
                        .show(ui, |ui| {
                            ui.label("账号:");
                            ui.add_sized(
                                [180.0, 26.0],
                                egui::TextEdit::singleline(&mut self.username).margin(egui::vec2(8.0, 4.0))
                            );
                            ui.end_row();

                            ui.label("密码:");
                            ui.add_sized(
                                [180.0, 26.0],
                                egui::TextEdit::singleline(&mut self.password)
                                    .password(true)
                                    .font(egui::TextStyle::Monospace)
                                    .margin(egui::vec2(8.0, 4.0))
                            );
                            ui.end_row();
                        });

                    ui.add_space(16.0); // 按钮和输入框之间留点空隙

                    if ui.add_sized([form_width, 32.0], egui::Button::new("保存账号")).clicked() {
                        if self.username.is_empty() {
                            toasts.push(ToastLevel::Warning, "账号不能为空");
                            return;
                        }
                        if self.password.is_empty() {
                            toasts.push(ToastLevel::Warning, "密码不能为空");
                            return;
                        }
                        toasts.push(ToastLevel::Success, "账号保存成功");
                    }
                },
            );
        });
    }
}