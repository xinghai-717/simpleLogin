use std::sync::mpsc::{channel, Receiver};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use eframe::egui;
use shipup::Update;
use crate::updater::run_update_worker;

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

pub struct UpdateManager {
    pub state: UpdateState,
    pub cancel_flag: Arc<AtomicBool>,
    update_rx: Option<Receiver<UpdateMessage>>,
    pub pending_update: Option<Update>,
    pub is_auto_restart: bool,
}

impl UpdateManager {
    pub fn new() -> Self {
        Self {
            state: UpdateState::Idle,
            cancel_flag: Arc::new(AtomicBool::new(false)),
            update_rx: None,
            pending_update: None,
            is_auto_restart: false,
        }
    }

    pub fn start_check(&mut self, auto_restart: bool) {
        if self.update_rx.is_some() { return; }

        self.is_auto_restart = auto_restart;
        self.state = UpdateState::Checking;
        self.cancel_flag.store(false, Ordering::Relaxed);

        let (tx, rx) = channel();
        self.update_rx = Some(rx);
        let cancel = self.cancel_flag.clone();

        std::thread::spawn(move || {
            run_update_worker(tx, cancel, auto_restart);
        });
    }

    pub fn tick(&mut self, ctx: &egui::Context) {
        if let Some(rx) = &self.update_rx {
            if let Ok(event) = rx.try_recv() {
                match event {
                    UpdateMessage::State(state) => {
                        self.state = state;
                        // 只有 Error 和 UpToDate 才清理通道
                        if matches!(self.state, UpdateState::Error(_) | UpdateState::UpToDate) {
                            self.update_rx = None;
                        }
                    }
                    UpdateMessage::ReadyToRestart(update) => {
                        self.state = UpdateState::ReadyToRestart;
                        self.pending_update = Some(update);
                        self.update_rx = None;
                    }
                }
            } else {
                ctx.request_repaint_after(std::time::Duration::from_millis(100));
            }
        }
    }
}