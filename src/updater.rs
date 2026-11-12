use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use shipup::{Updater, UpdateEvent};
use std::time::Duration;
use crate::pages::about::{UpdateMessage, UpdateState};

fn get_manifest_url() -> &'static str {
    if cfg!(debug_assertions) {
        "http://localhost:8080/last.json"
    } else {
        "https://simplelogin.wrh6.qzz.io/last.json"
    }
}
pub fn run_update_worker(tx: std::sync::mpsc::Sender<UpdateMessage>, cancel_flag: Arc<AtomicBool>) {
    let _ = tx.send(UpdateMessage::State(UpdateState::Checking));

    let builder = match Updater::builder().current_version(env!("CARGO_PKG_VERSION")) {
        Ok(b) => b,
        Err(e) => {
            let _ = tx.send(UpdateMessage::State(UpdateState::Error(e.to_string())));
            return;
        }
    };
    let mut updater = match builder.manifest_url(get_manifest_url())
        .public_key(include_str!(".././keys/ed25519.pub"))
        .timeout(Duration::from_secs(30))
        .dangerous_insecure_transport_protocol(true)
        .build()
    {
        Ok(u) => u,
        Err(e) => {
            let _ = tx.send(UpdateMessage::State(UpdateState::Error(e.to_string())));
            return;
        }
    };

    match updater.check() {
        Ok(Some(update)) => {
            // 发现新版本
            let _ = tx.send(UpdateMessage::State(UpdateState::Downloading { progress: 0.0 }));

            // 使用带进度回调的安装
            let tx_clone = tx.clone();
            let result = update.download_and_install_with_cancellation(
                Some(cancel_flag.clone()),
                move |event| match event {
                    UpdateEvent::DownloadProgress { percent, .. } => {
                        if let Some(p) = percent {
                            let _ = tx_clone.send(UpdateMessage::State(UpdateState::Downloading { progress: p }));
                        }
                    }
                    _ => {}
                },
            );
            match result {
                Ok(_) => {
                    use std::sync::atomic::Ordering;
                    if cancel_flag.load(Ordering::Relaxed) {
                        let _ = tx.send(UpdateMessage::State(UpdateState::Idle));
                    } else {
                        let _ = tx.send(UpdateMessage::ReadyToRestart(update));
                    }
                }
                Err(e) => {
                    use std::sync::atomic::Ordering;
                    if cancel_flag.load(Ordering::Relaxed) {
                        // 是因为点了取消而产生的错误，静默处理即可
                        let _ = tx.send(UpdateMessage::State(UpdateState::Idle));
                    } else {
                        // 真正的网络错误
                        let _ = tx.send(UpdateMessage::State(UpdateState::Error(e.to_string())));
                    }
                }
            }
        }
        Ok(None) => {
            let _ = tx.send(UpdateMessage::State(UpdateState::UpToDate));
        }
        Err(e) => {
            let _ = tx.send(UpdateMessage::State(UpdateState::Error(e.to_string())));
        }
    }
}