//! 应用装配：插件注册、状态初始化、IPC 命令、托盘、调度器

pub mod acme;
pub mod cert;
pub mod commands;
pub mod dns;
pub mod error;
pub mod http01;
pub mod logs;
pub mod notify;
pub mod scheduler;
pub mod secret;
pub mod state;
pub mod storage;
pub mod util;

use tauri::Manager;
use tauri_plugin_autostart::MacosLauncher;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default()
        // 单实例
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        // 打开路径 / 外部链接
        .plugin(tauri_plugin_opener::init())
        // 系统通知（续期结果）
        .plugin(tauri_plugin_notification::init())
        // 剪贴板
        .plugin(tauri_plugin_clipboard_manager::init())
        // 开机自启
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec![]),
        ))
        // 初始化应用状态
        .setup(|app| {
            let app_data_dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."));
            // 日志文件写入应用数据目录
            logs::set_file(&app_data_dir);
            let platform = std::env::consts::OS.to_string();
            let state = state::AppState::new(app_data_dir, platform)
                .map_err(|e| {
                    log::error!("failed to init app state: {e}");
                    std::io::Error::other(e.to_string())
                })?;
            app.manage(state);

            // 开机自启状态与设置同步
            setup_autostart(app);

            // 托盘
            setup_tray(app)?;

            // 续期调度器
            scheduler::spawn_scheduler(app.handle().clone());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::system::get_app_info,
            commands::system::get_platform_info,
            commands::system::probe_port80,
            commands::system::open_path,
            commands::system::open_url,
            commands::system::copy_to_clipboard,
            commands::settings::get_settings,
            commands::settings::set_setting,
            commands::settings::set_settings,
            commands::certificates::list_certificates,
            commands::certificates::get_certificate,
            commands::certificates::delete_certificate,
            commands::certificates::get_usage_guide,
            commands::certificates::check_duplicate,
            commands::certificates::renew_now,
            commands::providers::list_providers,
            commands::providers::save_provider,
            commands::providers::test_provider,
            commands::providers::delete_provider,
            commands::issue::start_issue,
            commands::issue::cancel_issue,
            commands::issue::get_job_status,
            commands::issue::confirm_txt,
            commands::renewal::check_renewals,
            commands::system::get_logs,
            commands::system::clear_logs,
            commands::system::frontend_log,
        ]);

    let app = builder
        // 关闭主窗口时仅隐藏到托盘，保持后台自动续期；真正退出走托盘菜单
        .on_window_event(|window, event| {
            if window.label() == "main" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    let _ = window.hide();
                    api.prevent_close();
                }
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|_app, _event| {});
}

fn setup_autostart(app: &tauri::App) {
    use tauri_plugin_autostart::ManagerExt;
    let auto = app.autolaunch();
    let enabled = crate::storage::settings::get_bool(
        &app.state::<state::AppState>().db.lock(),
        "run_at_login",
        true,
    );
    let _ = auto.enable();
    if !enabled {
        let _ = auto.disable();
    }
}

fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
    use tauri::tray::{TrayIconBuilder, TrayIconEvent};

    let show_i = MenuItem::with_id(app, "show", "打开 TopSSL 免费证书助手", true, None::<&str>)?;
    let check_i = MenuItem::with_id(app, "check", "立即检查续期", true, None::<&str>)?;
    let quit_i = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&show_i, &check_i, &sep, &quit_i])?;

    let tray = TrayIconBuilder::with_id("main-tray")
        .icon(app.default_window_icon().cloned().unwrap_or_else(|| {
            // 无图标时生成 1x1 PNG
            tauri::image::Image::new_owned(vec![0u8; 4], 1, 1)
        }))
        .menu(&menu)
        .tooltip("TopSSL 免费证书助手")
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.unminimize();
                    let _ = window.set_focus();
                }
            }
            "check" => {
                // 立即检查续期：窗口可见 → 前端 toast；窗口隐藏 → 系统通知
                let visible = app
                    .get_webview_window("main")
                    .and_then(|w| w.is_visible().ok())
                    .unwrap_or(false);
                if let Some(state) = app.try_state::<state::AppState>() {
                    match crate::commands::renewal::check_renewals_impl(true, &state, app.clone()) {
                        Ok(results) => crate::notify::manual_check_summary(app, &results, visible),
                        Err(e) => crate::notify::manual_check_error(app, &e.message, visible),
                    }
                }
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: tauri::tray::MouseButton::Left,
                button_state: tauri::tray::MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.unminimize();
                    let _ = window.set_focus();
                }
            }
        });

    tray.build(app)?;
    Ok(())
}
