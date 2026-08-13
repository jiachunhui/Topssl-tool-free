//! 续期调度器：启动时检查 + 每 12h 周期检查

use std::time::Duration;

use tauri::{Emitter, Manager};

pub fn spawn_scheduler(app: tauri::AppHandle) {
    // 启动 10 秒后首次检查（避免与初始化冲突）
    let app2 = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(10)).await;
        run_check(&app2);
    });

    // 每 12 小时周期检查
    let app3 = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(12 * 3600)).await;
            run_check(&app3);
        }
    });
}

fn run_check(app: &tauri::AppHandle) {
    let Some(state) = app.try_state::<crate::state::AppState>() else { return };
    let auto_renew = crate::storage::settings::get_bool(&state.db.lock(), "auto_renew", true);
    if !auto_renew {
        return;
    }
    log::info!("scheduler: checking renewals");
    match crate::commands::renewal::check_renewals_impl(false, &state, app.clone()) {
        Ok(results) => {
            for r in &results {
                if !r.ok && !r.message.contains("进行中") {
                    // 续期失败 → 系统通知
                    let _ = app.emit(
                        "renewal://failed",
                        serde_json::json!({ "domain": r.domain, "message": r.message }),
                    );
                    use tauri_plugin_notification::NotificationExt;
                    let _ = app
                        .notification()
                        .builder()
                        .title("证书续期失败")
                        .body(format!("{}：{}", r.domain, r.message))
                        .show();
                }
            }
        }
        Err(e) => {
            log::error!("renewal check failed: {e}");
        }
    }
}
