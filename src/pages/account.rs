use eframe::egui;
use crate::widgets::{ToastLevel,ToastManager};
use crate::smp::{SmpConfig, load_password, save_password, delete_password};

#[derive(Default)]
pub struct AccountPage {
    pub username: String,
    pub password: String,
    pub singleline_username: String,
    pub singleline_password: String,
}

impl AccountPage {
    pub fn new(smp_config: &SmpConfig) -> Self {
        let mut page = Self::default();
        page.username = smp_config.username.clone();
        page.singleline_username = smp_config.username.clone();

        if !page.username.is_empty() {
            if let Ok(Some(pwd)) = load_password(&page.username) {
                page.password = pwd.clone();
                page.singleline_password = pwd.clone();
            }
        }
        page
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, toasts: &mut ToastManager, smp_config: &mut SmpConfig) {
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
                                egui::TextEdit::singleline(&mut self.singleline_username).margin(egui::vec2(8.0, 4.0))
                            );
                            ui.end_row();

                            ui.label("密码:");
                            ui.add_sized(
                                [180.0, 26.0],
                                egui::TextEdit::singleline(&mut self.singleline_password)
                                    .password(true)
                                    .font(egui::TextStyle::Monospace)
                                    .margin(egui::vec2(8.0, 4.0))
                            );
                            ui.end_row();
                        });

                    ui.add_space(16.0); // 按钮和输入框之间留点空隙

                    // 两个按钮横向布局
                    ui.horizontal(|ui| {
                        // 调整两个按钮之间的间距，如果不需要可以删掉
                        // ui.spacing_mut().item_spacing.x = 8.0;
                        let btn_width = (form_width - 8.0) / 2.0;

                        if ui.add_sized([btn_width, 32.0], egui::Button::new("保存账号")).clicked() {
                            if self.singleline_username.is_empty() || self.singleline_password.is_empty() {
                                if !self.username.is_empty() {
                                    match delete_password(&self.username) {
                                        Ok(_) => {
                                            self.singleline_username.clear();
                                            self.singleline_password.clear();
                                            self.username.clear();
                                            self.password.clear();
                                            smp_config.username.clear();
                                            toasts.push(ToastLevel::Success, "账号记录清空成功");
                                        }
                                        Err(_) => {
                                            toasts.push(ToastLevel::Error, "清空账号时发生错误");
                                            // TODO 写日志
                                        }
                                    }
                                    return;
                                }
                                toasts.push(ToastLevel::Warning, "账号或密码不能为空");
                                return;
                            }
                            match save_password(&self.singleline_username, &self.singleline_password, &smp_config.username) {
                                Ok(_) => {
                                    smp_config.username = self.singleline_username.clone();
                                    self.username = self.singleline_username.clone();
                                    self.password = self.singleline_password.clone();
                                    toasts.push(ToastLevel::Success, "账号保存成功");
                                }
                                Err(e) => {
                                    toasts.push(ToastLevel::Error, format!("保存失败: {}", e));
                                    // TODO 写日志
                                }
                            }
                        }

                        if ui.add_sized([btn_width, 32.0], egui::Button::new("取消")).clicked() {
                            self.singleline_username = self.username.clone();
                            self.singleline_password = self.password.clone();
                        }
                    });
                },
            );
        });
    }
}