//! 续期调度器：启动时检查 + 动态周期检查（到期临近时提高检查频率）

use std::time::Duration;

use tauri::Manager;

pub fn spawn_scheduler(app: tauri::AppHandle) {
    // 启动 10 秒后首次检查（避免与初始化冲突）
    let app2 = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(10)).await;
        run_check(&app2);
    });

    // 周期检查：到期前 7 天每 6 小时、前 1 天每 2 小时，其余每 12 小时
    let app3 = app.clone();
    tauri::async_runtime::spawn(async move {
        let mut interval = Duration::from_secs(12 * 3600);
        loop {
            tokio::time::sleep(interval).await;
            run_check(&app3);
            interval = next_interval(&app3);
        }
    });
}

fn run_check(app: &tauri::AppHandle) {
    let Some(state) = app.try_state::<crate::state::AppState>() else { return };

    // 到期提醒独立于自动续期开关：关闭自动续期时更需要提醒用户
    crate::notify::expiring(app, &state);

    let auto_renew = crate::storage::settings::get_bool(&state.db.lock(), "auto_renew", true);
    if !auto_renew {
        return;
    }
    log::info!("scheduler: checking renewals");
    match crate::commands::renewal::check_renewals_impl(false, &state, app.clone()) {
        Ok(results) => {
            for r in &results {
                if !r.ok && !r.message.contains("进行中") {
                    // 同步触发失败（未进入任务流程）；异步失败在 issue 任务内通知，二者互斥不重复
                    crate::notify::renew_failed(app, &state.db, &r.domain, &r.message, None);
                }
            }
        }
        Err(e) => {
            log::error!("renewal check failed: {e}");
        }
    }
}

/// 根据最早到期的证书计算下次检查间隔（无证书或解析失败时回退 12 小时）
fn next_interval(app: &tauri::AppHandle) -> Duration {
    let Some(state) = app.try_state::<crate::state::AppState>() else {
        return Duration::from_secs(12 * 3600);
    };
    let conn = state.db.lock();
    let days = crate::storage::certificates::min_expires_at(&conn)
        .ok()
        .flatten()
        .and_then(|expires| {
            let dt = chrono::DateTime::parse_from_rfc3339(&expires).ok()?;
            Some((dt.with_timezone(&chrono::Utc) - chrono::Utc::now()).num_days())
        });
    drop(conn);
    match days {
        Some(d) if d <= 1 => Duration::from_secs(2 * 3600),
        Some(d) if d <= 7 => Duration::from_secs(6 * 3600),
        _ => Duration::from_secs(12 * 3600),
    }
}
