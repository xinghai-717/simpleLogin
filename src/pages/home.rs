use eframe::egui;
use reqwest::blocking::Client;
use simple_login::smp::{AuthStatus, do_status, do_login, do_logout, LoginResult, LogoutResult, LoginParams, SmpConfig};
use tracing::{info, error, warn};
use crate::widgets::{ToastLevel, ToastManager};
use crate::smp::save_config;
use std::sync::mpsc::{channel, Receiver};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkOperation { Online, Offline }

impl Default for NetworkOperation {
    fn default() -> Self { NetworkOperation::Offline }
}

// 👈 用一个枚举统一接收后台线程发回来的结果
enum NetOpResult {
    Login(LoginResult),
    Logout(LogoutResult),
}

pub struct HomePage {
    pub status: Option<AuthStatus>,
    selected_op: NetworkOperation,
    pub is_processing: bool,
    result_rx: Option<Receiver<NetOpResult>>,
}

impl HomePage {
    pub fn new(client: &Client) -> Self {
        let status = do_status(client);
        Self {
            status: Some(status),
            selected_op: NetworkOperation::default(),
            is_processing: false,
            result_rx: None,
        }
    }

    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        client: &Client,
        toast_manager: &mut ToastManager,
        smp_config: &mut SmpConfig,
        username: &str,
        password: &str,
    ) {
        ui.add_space(6.0);
        let status_str = match &self.status {
            Some(AuthStatus::Online) => "在线",
            Some(AuthStatus::Offline { .. }) => "离线",
            Some(AuthStatus::Failed(e)) => {
                ui.label(format!("状态获取失败: {}", e));
                return;
            },
            None => "检测中/未知",
        };
        ui.label(format!("当前状态: {}", status_str));

        ui.separator();
        ui.label("网络操作:");

        ui.radio_value(&mut self.selected_op, NetworkOperation::Online, "上线");
        ui.radio_value(&mut self.selected_op, NetworkOperation::Offline, "下线");

        ui.add_space(10.0);

        // 👈 处理中：显示加载圈，禁用按钮
        if self.is_processing {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("正在请求中...");
            });
        } else {
            if ui.button("执行操作").clicked() {
                // 1. 前置拦截：根据当前状态防止无效操作
                match (&self.selected_op, &self.status) {
                    (NetworkOperation::Online, Some(AuthStatus::Online)) => {
                        toast_manager.push(ToastLevel::Warning, "您当前已经在线了");
                        return;
                    }
                    (NetworkOperation::Offline, Some(AuthStatus::Offline { .. })) => {
                        toast_manager.push(ToastLevel::Warning, "您当前已经离线");
                        return;
                    }
                    (_, Some(AuthStatus::Failed(_))) => {
                        toast_manager.push(ToastLevel::Warning, "网络状态异常");
                        return;
                    }
                    (_, None) => {
                        toast_manager.push(ToastLevel::Warning, "正在检测状态");
                        return;
                    }
                    _ => {} // 状态合法，允许操作
                }

                // 2. 校验必填项
                if self.selected_op == NetworkOperation::Online && (username.is_empty() || password.is_empty()) {
                    toast_manager.push(ToastLevel::Warning, "请先在账号页填写账号和密码");
                    return;
                }

                info!("开始尝试网络操作...");
                self.is_processing = true;
                let (tx, rx) = channel();
                self.result_rx = Some(rx);

                // 3. 准备传给后台线程的数据（需要克隆为拥有所有权的数据）
                let client_clone = client.clone();
                let op = self.selected_op;

                // 构造 LoginParams (do_login 需要)
                let params = LoginParams {
                    username: username.to_string(),
                    password: password.to_string(),
                    secret_format: smp_config.secret_format.clone(),
                };

                // 克隆配置 (do_logout 需要)
                let config_clone = smp_config.clone();

                // 4. 启动后台线程
                std::thread::spawn(move || {
                    let result = match op {
                        NetworkOperation::Online => {
                            NetOpResult::Login(do_login(&client_clone, params))
                        }
                        NetworkOperation::Offline => {
                            NetOpResult::Logout(do_logout(&client_clone, &config_clone))
                        }
                    };
                    let _ = tx.send(result);
                });
            }
        }

        // 5. 轮询后台线程是否发回了结果
        if let Some(rx) = &self.result_rx {
            if let Ok(result) = rx.try_recv() {
                self.is_processing = false;
                self.result_rx = None; // 清理通道

                match result {
                    NetOpResult::Login(login_res) => {
                        match login_res {
                            LoginResult::AlreadyOnline => {
                                toast_manager.push(ToastLevel::Warning, "您已上线");
                                self.status = Some(AuthStatus::Online);
                            }
                            LoginResult::Success { online_fields, .. } => {
                                smp_config.online_fields = Some(online_fields);
                                if save_config(smp_config).is_err() {
                                    toast_manager.push(ToastLevel::Error, "保存登录信息失败");
                                } else {
                                    toast_manager.push(ToastLevel::Success, "登录成功");
                                }
                                self.status = Some(AuthStatus::Online); // 👈 实时更新界面状态
                            }
                            LoginResult::Failed { visible, title } => {
                                let reason = if !visible.is_empty() { visible } else { title };
                                toast_manager.push(ToastLevel::Error, format!("登录失败: {}", reason));
                            }
                            LoginResult::NetworkError(e) => {
                                toast_manager.push(ToastLevel::Error, "无法完成登录：网络错误");
                                error!("网络错误: {}", e);
                            }
                            LoginResult::MissingModule => {
                                toast_manager.push(ToastLevel::Error, "无法完成登录：模数丢失");
                            }
                        }
                    }
                    NetOpResult::Logout(logout_res) => {
                        match logout_res {
                            LogoutResult::AlreadyOffline => {
                                toast_manager.push(ToastLevel::Warning, "已经退出登录了");
                                self.status = Some(AuthStatus::Offline { intercept_url: String::new() });
                            }
                            LogoutResult::Success => {
                                toast_manager.push(ToastLevel::Success, "退出登录成功");
                                self.status = Some(AuthStatus::Offline { intercept_url: String::new() });
                            }
                            LogoutResult::Failed { title, visible } => {
                                let reason = if !visible.is_empty() { visible } else { title };
                                toast_manager.push(ToastLevel::Error, format!("退出登录失败: {}", reason));
                            }
                            LogoutResult::NoOnlineInfo => {
                                toast_manager.push(ToastLevel::Warning, "没有缓存在线信息，无法登出");
                                warn!("没有缓存在线信息，无法登出");
                            }
                            LogoutResult::NetworkError(e) => {
                                toast_manager.push(ToastLevel::Error, "无法退出登录：网络错误");
                                error!("网络错误: {}", e);
                            }
                        }
                    }
                }
            } else {
                // 如果后台还在跑，请求 egui 保持高频刷新，让加载圈转起来
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(50));
            }
        }
    }
}