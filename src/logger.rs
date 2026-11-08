use directories::BaseDirs;
use std::collections::VecDeque;
use std::io::Write;
use std::sync::{Arc, Mutex};
use tracing_appender::rolling;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

pub type LogBuffer = Arc<Mutex<VecDeque<String>>>;
#[derive(Clone)]
struct MemoryWriter(LogBuffer);

impl Write for MemoryWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let text = String::from_utf8_lossy(buf).to_string();
        let mut buffer = self.0.lock().unwrap();
        buffer.push_back(text);
        // 限制最大行数，防止内存溢出
        if buffer.len() > 500 {
            buffer.pop_front();
        }
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub fn init_logging(log_buffer: LogBuffer) -> tracing_appender::non_blocking::WorkerGuard{
    let log_dir = BaseDirs::new().unwrap().data_dir().join("SimpleLogin").join("logs");

    std::fs::create_dir_all(&log_dir).unwrap();

    // 按天滚动文件输出
    let file_appender = rolling::daily(&log_dir, "simplelogin.log");

    // 设置终端输出
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let filter = EnvFilter::from_default_env()
        .add_directive(tracing::Level::INFO.into())
        .add_directive("simple_login=debug".parse().unwrap());

    let memory_writer = MemoryWriter(log_buffer);

    tracing_subscriber::registry()
        .with(fmt::layer().with_writer(std::io::stdout)) // 输出到控制台
        .with(fmt::layer().with_ansi(false).with_writer(non_blocking))    // 输出到文件
        .with(fmt::layer().with_ansi(true).with_writer(move || memory_writer.clone()))
        .with(filter)
        .init();

    tracing::info!("日志系统就绪");

    guard
}