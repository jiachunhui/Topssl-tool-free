// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // 最早初始化日志系统（全局 logger）
    topssl_free_cert_assistant_lib::logs::init();
    topssl_free_cert_assistant_lib::run();
}
