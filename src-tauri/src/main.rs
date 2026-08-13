// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // 最早初始化日志系统（全局 logger）
    ssl_cert_desktop_lib::logs::init();
    ssl_cert_desktop_lib::run();
}
