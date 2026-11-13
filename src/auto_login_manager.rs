use std::time::Instant;
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use reqwest::blocking::Client;
use simple_login::smp::{do_login, LoginResult, LoginParams};
use crate::smp::SmpConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutoLoginState {
    /// 空闲状态：在线，静静等待下一次掉线
    Idle,
    /// 重连中：发现掉线，正在尝试自动登录
    Reconnecting,
    /// 已挂起：连续失败次数过多，不再自动重试
    Suspended,
}

/// 自动登录管理器抛出给主线程的事件
pub enum AutoLoginEvent {
    ReconnectSuccess(std::collections::HashMap<String, String>),
    ReconnectFailed(String),
    RetriesExhausted,
}

pub struct AutoLoginManager {
    pub state: AutoLoginState,
    last_auto_login_attempt: Instant,
    try_login_attempts: u32,
    result_rx: Option<Receiver<LoginResult>>,
}

impl AutoLoginManager {
    pub fn new() -> Self {
        Self {
            state: AutoLoginState::Idle,
            last_auto_login_attempt: Instant::now(),
            try_login_attempts: 0,
            result_rx: None,
        }
    }

    /// 用户手动登录成功时，外部需要调用此方法唤醒
    pub fn resume(&mut self) {
        self.state = AutoLoginState::Idle;
        self.try_login_attempts = 0;
        self.last_auto_login_attempt = Instant::now();
        self.result_rx = None;
    }

    pub fn tick(
        &mut self,
        client: &Client,
        config: &SmpConfig,
        is_offline: bool,
        username: &str,
        password: &str,
    ) -> Vec<AutoLoginEvent> {
        let mut events = Vec::new();

        // 如果网络自己恢复了（或者用户手动登录了），立刻打断重连并重置状态
        if !is_offline {
            if self.state == AutoLoginState::Reconnecting {
                // 放弃后台线程的结果
                self.result_rx = None;
            }
            if self.state != AutoLoginState::Suspended {
                self.state = AutoLoginState::Idle;
            }
            self.try_login_attempts = 0;
            // 刷新计时器，防止刚掉线时因为计时器太旧而立刻发起请求
            self.last_auto_login_attempt = Instant::now();
            return events;
        }

        match self.state {
            AutoLoginState::Idle => {
                // 冷却 3 秒，且判断失败次数上限
                if self.last_auto_login_attempt.elapsed().as_secs() >= 3 {
                    if self.try_login_attempts >= 5 {
                        self.state = AutoLoginState::Suspended;
                        events.push(AutoLoginEvent::RetriesExhausted);
                        return events;
                    }

                    self.last_auto_login_attempt = Instant::now();
                    self.state = AutoLoginState::Reconnecting;

                    let client_clone = client.clone();
                    let params = LoginParams {
                        username: username.to_string(),
                        password: password.to_string(),
                        secret_format: config.secret_format.clone(),
                    };
                    let (tx, rx) = channel();
                    self.result_rx = Some(rx);

                    std::thread::spawn(move || {
                        let _ = tx.send(do_login(&client_clone, params));
                    });
                }
            }

            AutoLoginState::Reconnecting => {
                if let Some(rx) = &self.result_rx {
                    match rx.try_recv() {
                        Ok(result) => {
                            self.result_rx = None;
                            self.state = AutoLoginState::Idle;

                            match result {
                                LoginResult::AlreadyOnline => {
                                    self.try_login_attempts = 0;
                                }
                                LoginResult::Success { online_fields, .. } => {
                                    self.try_login_attempts = 0;
                                    events.push(AutoLoginEvent::ReconnectSuccess(online_fields));
                                }
                                LoginResult::Failed { title, visible } => {
                                    self.try_login_attempts += 1;
                                    let reason = if !visible.is_empty() { visible } else { title };
                                    events.push(AutoLoginEvent::ReconnectFailed(reason));
                                }
                                LoginResult::NetworkError(e) => {
                                    self.try_login_attempts += 1;
                                    events.push(AutoLoginEvent::ReconnectFailed(e.to_string()));
                                }
                                LoginResult::MissingModule => {
                                    self.try_login_attempts += 1;
                                    events.push(AutoLoginEvent::ReconnectFailed("模数丢失".to_string()));
                                }
                            }

                            // 请求结束后，如果失败次数达到 5 次，立刻挂起
                            if self.try_login_attempts >= 5 {
                                self.state = AutoLoginState::Suspended;
                                events.push(AutoLoginEvent::RetriesExhausted);
                            }
                        }
                        Err(TryRecvError::Empty) => {}
                        Err(TryRecvError::Disconnected) => {
                            // 线程崩溃：更新计时器，防止立刻发起下一次重试
                            self.result_rx = None;
                            self.state = AutoLoginState::Idle;
                            self.try_login_attempts += 1;
                            self.last_auto_login_attempt = Instant::now();
                        }
                    }
                }
            }

            AutoLoginState::Suspended => {}
        }
        events
    }
}