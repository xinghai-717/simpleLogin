use eframe::egui;
use crate::widgets::{ToastLevel, ToastManager};
use tracing::{error, info};
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
                page.singleline_password = pwd;
            }
        }
        page
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, toasts: &mut ToastManager, smp_config: &mut SmpConfig) {
        ui.heading("账号");
        ui.separator();

        // 垂直居中容器
        ui.vertical_centered(|ui| {
            ui.add_space(20.0);

            // 表单最大宽度
            let form_width = 280.0;

            // 固定这块区域宽度，并左对齐
            ui.allocate_ui_with_layout(
                egui::vec2(form_width, ui.available_height()),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    let mut changed = false;
                    egui::ComboBox::from_id_salt("account_combo")
                        .selected_text(if self.singleline_username.is_empty() { "选择或新建账号" } else { &self.singleline_username })
                        .show_ui(ui, |ui| {
                            for name in &smp_config.userlist {
                                if ui.selectable_value(&mut self.singleline_username, name.clone(), name).changed() {
                                    changed = true;
                                }
                            }
                            if ui.selectable_value(&mut self.singleline_username, "".to_string(), "新建账户").changed() {
                                changed = true;
                            }
                        });

                    if changed {
                        if self.singleline_username.is_empty() {
                            self.singleline_password.clear();
                        } else {
                            match load_password(&self.singleline_username) {
                                Ok(Some(pwd)) => self.singleline_password = pwd,
                                Ok(None) => self.singleline_password.clear(),
                                Err(e) => {
                                    toasts.push(ToastLevel::Error, "读取密码失败");
                                    error!("读取密码失败: {}", e);
                                }
                            }
                        }
                    }

                    ui.add_space(12.0);

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

                    ui.add_space(16.0);

                    // 两个按钮横向布局
                    ui.horizontal(|ui| {
                        let btn_width = (form_width - 8.0) / 2.0;

                        if ui.add_sized([btn_width, 32.0], egui::Button::new("保存账号")).clicked() {
                            if self.singleline_username.is_empty() || self.singleline_password.is_empty() {
                                toasts.push(ToastLevel::Warning, "账号或密码不能为空");
                                return;
                            }

                            match save_password(&self.singleline_username, &self.singleline_password) {
                                Ok(_) => {
                                    if !smp_config.userlist.contains(&self.singleline_username) {
                                        smp_config.userlist.push(self.singleline_username.clone());
                                    }

                                    smp_config.username = self.singleline_username.clone();
                                    self.username = self.singleline_username.clone();
                                    self.password = self.singleline_password.clone();
                                    toasts.push(ToastLevel::Success, "账号保存成功");
                                }
                                Err(e) => {
                                    toasts.push(ToastLevel::Error, "保存失败");
                                    error!("保存失败: {}", e);
                                }
                            }
                        }

                        if ui.add_sized([btn_width, 32.0], egui::Button::new("删除账号")).clicked() {
                            if self.username.is_empty() {
                                toasts.push(ToastLevel::Warning, "当前没有保存的账号");
                                return;
                            }

                            match delete_password(&self.username) {
                                Ok(_) => {
                                    smp_config.userlist.retain(|x| x != &self.username);
                                    self.singleline_username.clear();
                                    self.singleline_password.clear();
                                    self.username.clear();
                                    self.password.clear();
                                    smp_config.username.clear();
                                    toasts.push(ToastLevel::Success, "账号记录清空成功");
                                }
                                Err(e) => {
                                    toasts.push(ToastLevel::Error, "清空账号时发生错误");
                                    error!("清空账号时发生错误: {}", e);
                                }
                            }
                        }
                    });

                    ui.add_space(6.0);

                    if ui.add_sized([form_width, 24.0], egui::Button::new("取消").small()).clicked() {
                        self.singleline_username = self.username.clone();
                        self.singleline_password = self.password.clone();
                    }
                },
            );
        });
    }
}