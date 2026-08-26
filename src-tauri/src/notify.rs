//! 系统通知与前端事件：续期结果、到期提醒、手动检查反馈
//!
//! 防骚扰策略：
//! - 到期提醒按「证书 + 级别」去重（notifications 表），同一证书同一级别只提醒一次
//! - 同级别多张证书聚合为一条通知
//! - 手动检查：窗口可见只发事件（前端 toast），窗口隐藏才发系统通知，避免双重反馈

use std::collections::HashMap;

use tauri::{AppHandle, Emitter};

use crate::state::AppState;
use crate::storage::Db;

/// 通知标题
const APP_TITLE: &str = "Tossl 免费SSL证书管理工具";

/// 到期提醒级别：剩余天数 <= 阈值时提醒（级别随时间单调递增：30 → 7 → 1 → expired）
/// 注意按阈值升序排列，保证先命中更紧急的级别
const EXPIRY_LEVELS: [(&str, i64); 3] = [("1", 1), ("7", 7), ("30", 30)];

/// 发送系统通知（调用方负责开关判断）
fn system(app: &AppHandle, body: &str) {
    use tauri_plugin_notification::NotificationExt;
    let _ = app.notification().builder().title(APP_TITLE).body(body).show();
}

/// RFC3339 → 可读日期（YYYY-MM-DD，解析失败返回原串）
fn fmt_date(rfc: &str) -> String {
    chrono::DateTime::parse_from_rfc3339(rfc)
        .map(|dt| dt.with_timezone(&chrono::Utc).format("%Y-%m-%d").to_string())
        .unwrap_or_else(|_| rfc.to_string())
}

/// 到期级别：expired / "30" / "7" / "1" / None（解析失败或不在 30 天窗口内）
fn expiry_level(expires_at: &str) -> Option<String> {
    let Ok(dt) = chrono::DateTime::parse_from_rfc3339(expires_at) else {
        return None;
    };
    let utc = dt.with_timezone(&chrono::Utc);
    let now = chrono::Utc::now();
    if utc < now {
        return Some("expired".into());
    }
    let days = (utc - now).num_days();
    for (level, threshold) in EXPIRY_LEVELS {
        if days <= threshold {
            return Some(level.to_string());
        }
    }
    None
}

/// 续期成功：系统通知 + renewal://renewed 事件（受 notify_renew_success 开关控制）
pub fn renew_success(app: &AppHandle, db: &Db, domain: &str, new_expiry: &str) {
    if !crate::storage::settings::get_bool(&db.lock(), "notify_renew_success", true) {
        return;
    }
    let body = format!("证书续期成功：{domain}，新到期日 {}", fmt_date(new_expiry));
    system(app, &body);
    let _ = app.emit(
        "renewal://renewed",
        serde_json::json!({ "domain": domain, "expires_at": new_expiry }),
    );
}

/// 续期失败：系统通知 + renewal://failed 事件（受 notify_renew_failed 开关控制）
/// streak 为连续失败次数：Some(n) 且 n >= 2 时追加升级提示；None 表示同步失败（未进入任务流程）
pub fn renew_failed(app: &AppHandle, db: &Db, domain: &str, message: &str, streak: Option<i64>) {
    if !crate::storage::settings::get_bool(&db.lock(), "notify_renew_failed", true) {
        return;
    }
    let mut body = format!("证书续期失败：{domain}（{message}）");
    if let Some(n) = streak {
        if n >= 2 {
            body.push_str(&format!("，已连续失败 {n} 次，请查看日志"));
        }
    }
    system(app, &body);
    let _ = app.emit(
        "renewal://failed",
        serde_json::json!({ "domain": domain, "message": message, "streak": streak }),
    );
}

/// 到期提醒：分级（30/7/1 天 + 已过期），按证书+级别去重，同级别聚合为一条
pub fn expiring(app: &AppHandle, state: &AppState) {
    if !crate::storage::settings::get_bool(&state.db.lock(), "notify_expiring", true) {
        return;
    }
    let conn = state.db.lock();
    let due = match crate::storage::certificates::list_due_renewal(&conn, chrono::Utc::now()) {
        Ok(d) => d,
        Err(e) => {
            log::error!("expiry notify: list due failed: {e}");
            return;
        }
    };

    // 按级别分组（去重后）：level -> domains
    let mut buckets: HashMap<String, Vec<String>> = HashMap::new();
    for cert in &due {
        // 测试环境证书不打扰用户（轻微问题 14）
        if cert.directory == "staging" {
            continue;
        }
        let Some(level) = expiry_level(&cert.expires_at) else {
            continue;
        };
        // 同一证书同一级别只提醒一次（级别单调递增，不会漏掉后续更紧急的提醒）
        if crate::storage::notifications::exists(&conn, "expiry", cert.id, &level).unwrap_or(false) {
            continue;
        }
        let _ = crate::storage::notifications::record(&conn, "expiry", cert.id, &level);
        buckets.entry(level).or_default().push(cert.domain.clone());
    }
    drop(conn);

    for level in ["30", "7", "1", "expired"] {
        let Some(domains) = buckets.get(level) else {
            continue;
        };
        let count = domains.len();
        let shown: Vec<&str> = domains.iter().map(|s| s.as_str()).take(3).collect();
        let list = if count > 3 {
            format!("{} 等 {} 张", shown.join("、"), count)
        } else {
            shown.join("、")
        };
        let body = if level == "expired" {
            format!("{count} 张证书已过期：{list}，请立即处理")
        } else {
            format!("{count} 张证书将在 {level} 天内到期：{list}")
        };
        system(app, &body);
        let _ = app.emit(
            "renewal://expiring",
            serde_json::json!({ "level": level, "count": count, "domains": domains }),
        );
        log::info!("expiry notify: level={level} count={count}");
    }
}

/// 手动检查反馈：窗口可见 → 仅事件（前端 toast）；窗口隐藏 → 系统通知
pub fn manual_check_summary(
    app: &AppHandle,
    results: &[crate::commands::renewal::RenewalResult],
    window_visible: bool,
) {
    let triggered = results
        .iter()
        .filter(|r| r.ok && !r.message.contains("手动 DNS"))
        .count();
    let failed = results
        .iter()
        .filter(|r| !r.ok && !r.message.contains("进行中"))
        .count();
    // 其余计入 skipped（如手动 DNS 证书跳过自动续期、已在续期中的证书）
    let skipped = results.len().saturating_sub(triggered + failed);
    let summary = if results.is_empty() {
        "检查完成：所有证书均未到期，无需续期".to_string()
    } else if failed == 0 {
        format!("检查完成：已为 {triggered} 张证书触发续期")
    } else {
        format!("检查完成：{triggered} 张已触发续期，{failed} 张失败（详见日志）")
    };
    let _ = app.emit(
        "renewal://check-done",
        serde_json::json!({ "summary": summary, "triggered": triggered, "failed": failed, "skipped": skipped }),
    );
    if !window_visible {
        system(app, &summary);
    }
    log::info!("manual renewal check: triggered={triggered} failed={failed} skipped={skipped}");
}

/// 手动检查执行出错（如数据库异常）
pub fn manual_check_error(app: &AppHandle, message: &str, window_visible: bool) {
    let summary = format!("续期检查失败：{message}");
    let _ = app.emit(
        "renewal://check-done",
        serde_json::json!({ "summary": summary, "triggered": 0, "failed": 0, "skipped": 0 }),
    );
    if !window_visible {
        system(app, &summary);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 回归：级别必须取最紧急的一级（5 天 → "7"，不能落到 "30"）
    #[test]
    fn expiry_level_picks_most_urgent() {
        let in5 = (chrono::Utc::now() + chrono::Duration::days(5)).to_rfc3339();
        assert_eq!(expiry_level(&in5).as_deref(), Some("7"));

        let in25 = (chrono::Utc::now() + chrono::Duration::days(25)).to_rfc3339();
        assert_eq!(expiry_level(&in25).as_deref(), Some("30"));

        // 今天到期（数小时后）→ 1 天级别
        let in3h = (chrono::Utc::now() + chrono::Duration::hours(3)).to_rfc3339();
        assert_eq!(expiry_level(&in3h).as_deref(), Some("1"));

        // 已过期 → expired
        let ago = (chrono::Utc::now() - chrono::Duration::hours(1)).to_rfc3339();
        assert_eq!(expiry_level(&ago).as_deref(), Some("expired"));

        // 30 天窗口外 / 解析失败 → 不提醒
        let in40 = (chrono::Utc::now() + chrono::Duration::days(40)).to_rfc3339();
        assert_eq!(expiry_level(&in40), None);
        assert_eq!(expiry_level("not-a-date"), None);
    }
}
