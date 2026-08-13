//! 应用内日志系统：内存环形缓冲 + 文件落盘
//!
//! 替代 tauri-plugin-log，前端可通过 get_logs command 查看日志，便于用户反馈问题。

use std::collections::VecDeque;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::sync::{Mutex, OnceLock};

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    pub time: String,
    pub level: String,
    pub msg: String,
}

pub struct AppLogger {
    buf: Mutex<VecDeque<LogEntry>>,
    file: Mutex<Option<std::fs::File>>,
    max: usize,
}

fn logger() -> &'static AppLogger {
    static LOGGER: OnceLock<AppLogger> = OnceLock::new();
    LOGGER.get_or_init(|| AppLogger {
        buf: Mutex::new(VecDeque::new()),
        file: Mutex::new(None),
        max: 1000,
    })
}

impl AppLogger {
    fn push(&self, level: &str, msg: &str) {
        let entry = LogEntry {
            time: chrono::Local::now().format("%H:%M:%S").to_string(),
            level: level.to_string(),
            msg: msg.to_string(),
        };
        {
            let mut buf = self.buf.lock().unwrap();
            buf.push_back(entry.clone());
            while buf.len() > self.max {
                buf.pop_front();
            }
        }
        if let Ok(mut file) = self.file.lock() {
            if let Some(f) = file.as_mut() {
                let _ = writeln!(f, "[{}][{}] {}", entry.time, entry.level, entry.msg);
            }
        }
    }
}

impl log::Log for AppLogger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }
    fn log(&self, record: &log::Record) {
        self.push(record.level().as_str(), &record.args().to_string());
    }
    fn flush(&self) {}
}

/// 初始化全局 logger（main 最早调用；幂等）
pub fn init() {
    if log::set_logger(logger()).is_ok() {
        log::set_max_level(log::LevelFilter::Info);
    }
}

/// 设置日志文件（应用数据目录，AppState 初始化后调用）
pub fn set_file(dir: &Path) {
    let _ = std::fs::create_dir_all(dir);
    let path = dir.join("app.log");
    // 简单轮转：超过 5MB 时归档为 app.log.1，避免日志无限增长
    if let Ok(meta) = std::fs::metadata(&path) {
        if meta.len() > 5 * 1024 * 1024 {
            let backup = dir.join("app.log.1");
            let _ = std::fs::remove_file(&backup);
            let _ = std::fs::rename(&path, &backup);
        }
    }
    let file = OpenOptions::new().create(true).append(true).open(&path).ok();
    *logger().file.lock().unwrap() = file;
}

/// 获取日志（最新的在前）
pub fn get(limit: usize) -> Vec<LogEntry> {
    logger().buf.lock().unwrap().iter().rev().take(limit).cloned().collect()
}

/// 清空日志
pub fn clear() {
    logger().buf.lock().unwrap().clear();
    if let Ok(mut file) = logger().file.lock() {
        if let Some(f) = file.as_mut() {
            let _ = f.set_len(0);
            let _ = f.flush();
        }
    }
}
