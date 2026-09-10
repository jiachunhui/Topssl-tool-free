//! 应用装配：插件注册、状态初始化、IPC 命令、托盘、调度器

pub mod acme;
pub mod backup;
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
pub mod updater;
pub mod util;

use tauri::{Emitter, Manager};
use tauri_plugin_autostart::MacosLauncher;

/// 开机自启启动时由注册表命令行写入的标记（见下方 autostart 插件初始化参数）
const AUTOSTART_FLAG: &str = "--autostart";

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default()
        // 单实例
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // 应用已驻留托盘时用户再次启动：把窗口显示出来
            show_main_window(app);
        }))
        // 打开路径 / 外部链接
        .plugin(tauri_plugin_opener::init())
        // 系统通知（续期结果）
        .plugin(tauri_plugin_notification::init())
        // 剪贴板
        .plugin(tauri_plugin_clipboard_manager::init())
        // 开机自启（带 --autostart 标记，供启动时区分「自启」与「用户双击」）
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec![AUTOSTART_FLAG]),
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
            let by_autostart = launched_by_autostart();
            let state = state::AppState::new(app_data_dir, platform, by_autostart)
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

            // 主窗口在 tauri.conf.json 中默认隐藏（visible=false）：
            // 开机自启时保持静默、只驻留托盘，用户手动启动才显示窗口
            if !by_autostart {
                show_main_window(app.handle());
            }

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
            commands::certificates::export_deploy_package,
            commands::certificates::check_duplicate,
            commands::certificates::renew_now,
            commands::backup::export_backup_package,
            commands::backup::import_backup_package,
            commands::iis::iis_status,
            commands::iis::iis_deploy_cert,
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
            updater::check_update,
            updater::download_update,
            updater::cancel_update_download,
            updater::install_update,
            updater::dismiss_update,
            updater::get_dismissed_update_version,
            updater::open_release_page,
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
    let enabled = crate::storage::settings::get_bool(
        &app.state::<state::AppState>().db.lock(),
        "run_at_login",
        true,
    );
    sync_autostart(app.handle(), enabled);
}

/// 把「开机自启」设置同步到系统注册表项
///
/// 开发构建只允许关闭：把 target/debug 下的临时可执行文件注册成开机自启，
/// 会每次开机弹出一个控制台窗口；但仍允许关闭，便于清理历史误注册。
pub(crate) fn sync_autostart(app: &tauri::AppHandle, enabled: bool) {
    use tauri_plugin_autostart::ManagerExt;

    if enabled && cfg!(debug_assertions) {
        log::warn!("autostart: 开发构建跳过注册，避免把 target/debug 可执行文件写入开机自启");
        return;
    }
    let autolaunch = app.autolaunch();
    let result = if enabled { autolaunch.enable() } else { autolaunch.disable() };
    match result {
        Ok(()) => log::info!("autostart: {}", if enabled { "enabled" } else { "disabled" }),
        Err(e) => log::warn!(
            "autostart: {} failed: {e}",
            if enabled { "enable" } else { "disable" }
        ),
    }
}

/// 本次启动是否来自开机自启（注册表命令行带 --autostart 标记）
fn launched_by_autostart() -> bool {
    std::env::args().any(|arg| arg == AUTOSTART_FLAG)
}

/// 显示主窗口：托盘「打开」、图标左键、用户再次启动统一走这里
fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
    // 前端据此补上「开机自启时推迟的更新检查」（见 src/stores/update.ts）
    let _ = app.emit("window://shown", ());
}

fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
    use tauri::tray::{TrayIconBuilder, TrayIconEvent};

    let show_i = MenuItem::with_id(app, "show", "打开 ToSSL 免费SSL证书管理工具", true, None::<&str>)?;
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
        .tooltip("ToSSL 免费SSL证书管理工具")
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => show_main_window(app),
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
                show_main_window(app);
            }
        });

    tray.build(app)?;
    Ok(())
}
