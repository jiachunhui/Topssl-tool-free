//! 申请任务 command：start_issue / cancel_issue / get_job_status
//!
//! start_issue 立即返回 job_id，后台在 tokio 任务中执行完整 ACME 流程。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tauri::State;

use crate::acme::flow::{self, FlowCtx};
use crate::acme::model::{IssueRequest, IssueStage, JobState, JobStatus};
use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// 启动申请任务
#[tauri::command]
pub fn start_issue(
    req: IssueRequest,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> AppResult<String> {
    spawn_issue_job(state.inner(), app, req, None)
}

/// 启动任务（内部共用：新申请 / 续期）
pub fn spawn_issue_job(
    state: &AppState,
    app: tauri::AppHandle,
    req: IssueRequest,
    renew_cert_id: Option<i64>,
) -> AppResult<String> {
    // 校验域名
    let domain = crate::util::domain::validate_domain(&req.domain)?;
    let mut req = req;
    req.domain = domain.clone();

    // SAN 校验
    let mut alt_names = Vec::new();
    for n in &req.alt_names {
        let d = crate::util::domain::validate_domain(n)?;
        if d.starts_with("*.") {
            return Err(AppError::new(
                crate::error::ErrorCode::InvalidDomain,
                "多域名证书不支持通配符（请使用 *. 主域名申请）",
            ));
        }
        alt_names.push(d);
    }
    req.alt_names = alt_names;

    // 通配符主域名下，SAN 不能是通配符覆盖的一级子域（LE 会以 malformed 拒绝）
    if let Some(base) = req.domain.strip_prefix("*.") {
        for n in &req.alt_names {
            let suffix = format!(".{base}");
            if let Some(prefix) = n.strip_suffix(&suffix) {
                // 恰好一级子域（如 b.wzjs100.com 之于 *.wzjs100.com）即冗余；
                // 多级子域（www.b.wzjs100.com）不在通配符覆盖内，合法
                if !prefix.is_empty() && !prefix.contains('.') {
                    return Err(AppError::new(
                        crate::error::ErrorCode::InvalidDomain,
                        format!("{n} 已被通配符 *.{base} 覆盖，无需重复添加"),
                    ));
                }
            }
        }
    }

    // 通配符必须 DNS-01
    if req.domain.starts_with("*.") && req.challenge_type != "dns01" {
        return Err(AppError::new(
            crate::error::ErrorCode::ChallengeUnsupported,
            "通配符证书只能使用 DNS 验证",
        ));
    }

    // 护栏：冷却检查（续期任务跳过重复证书拦截）
    crate::acme::limits::preflight(&state.db, &req, renew_cert_id.is_some())?;

    // DNS-01 API 模式时预构建 provider（手动模式不需要；失败时不产生任何任务状态）
    let provider = if req.challenge_type == "dns01" && !req.dns_manual {
        let conn = state.db.lock();
        let pid = req.provider_id.ok_or_else(|| {
            AppError::new(crate::error::ErrorCode::DnsProviderApi, "请选择 DNS 服务商")
        })?;
        let row = crate::storage::providers::get(&conn, pid)?
            .ok_or_else(|| AppError::new(crate::error::ErrorCode::DnsProviderApi, "DNS 服务商不存在"))?;
        drop(conn);
        Some(crate::dns::build_provider(&row, &state.secrets)?)
    } else {
        None
    };

    // 向导未填邮箱时使用设置里的默认邮箱
    let email_default = crate::storage::settings::get_string(&state.db.lock(), "contact_email", "");
    if req.contact_email.trim().is_empty() {
        req.contact_email = email_default;
    }

    let job_id = uuid::Uuid::new_v4().to_string();
    let cancel_flag = Arc::new(AtomicBool::new(false));
    state.cancels.lock().unwrap_or_else(|e| e.into_inner()).insert(job_id.clone(), cancel_flag.clone());
    // DNS 手动模式确认标记
    let txt_confirmed_flag = Arc::new(AtomicBool::new(false));
    state.txt_confirms.lock().unwrap_or_else(|e| e.into_inner()).insert(job_id.clone(), txt_confirmed_flag.clone());

    // 预置任务状态。续期场景在锁内完成「该证书是否已有进行中任务」检查 + 插入，
    // 调度器与手动「立即续期」两个入口共用，消除并发重复续期竞态（B3）。
    {
        let mut jobs = state.jobs.lock().unwrap_or_else(|e| e.into_inner());
        // 续期去重：该证书已有进行中任务（按 cert_id 匹配）
        if let Some(cid) = renew_cert_id {
            if jobs.values().any(|j| {
                j.cert_id == Some(cid)
                    && (j.state == JobState::Running || j.state == JobState::Pending)
            }) {
                drop(jobs);
                // 回滚已插入的取消/确认标记，避免残留
                state.cancels.lock().unwrap_or_else(|e| e.into_inner()).remove(&job_id);
                state.txt_confirms.lock().unwrap_or_else(|e| e.into_inner()).remove(&job_id);
                return Err(AppError::new(
                    crate::error::ErrorCode::DuplicateCert,
                    "该证书正在续期中，请稍候",
                ));
            }
        }
        // 同域名并发申请防护：同一域名已有进行中任务时拒绝重复发起（M12）
        if jobs.values().any(|j| {
            j.domain.as_deref() == Some(domain.as_str())
                && (j.state == JobState::Running || j.state == JobState::Pending)
        }) {
            drop(jobs);
            state.cancels.lock().unwrap_or_else(|e| e.into_inner()).remove(&job_id);
            state.txt_confirms.lock().unwrap_or_else(|e| e.into_inner()).remove(&job_id);
            return Err(AppError::new(
                crate::error::ErrorCode::DuplicateCert,
                "该域名已有申请/续期任务进行中，请稍候",
            ));
        }
        jobs.insert(
            job_id.clone(),
            JobStatus {
                job_id: job_id.clone(),
                state: JobState::Pending,
                stage: Some(IssueStage::InputValidated),
                percent: 2,
                message: Some("任务已创建".into()),
                error_code: None,
                error_detail: None,
                cert_id: renew_cert_id,
                domain: Some(domain.clone()),
            },
        );
    }

    // 记录任务日志（失败时回滚已插入的任务标记，避免泄漏）
    let log_id = match crate::storage::logs::start(&state.db.lock(), "issue", Some(&domain)) {
        Ok(id) => id,
        Err(e) => {
            state.jobs.lock().unwrap_or_else(|e| e.into_inner()).remove(&job_id);
            state.cancels.lock().unwrap_or_else(|e| e.into_inner()).remove(&job_id);
            state.txt_confirms.lock().unwrap_or_else(|e| e.into_inner()).remove(&job_id);
            return Err(AppError::from(e));
        }
    };

    log::info!(
        "issue started: job={job_id} domain={domain} challenge={} directory={} dns_manual={} renew={}",
        req.challenge_type,
        req.directory,
        req.dns_manual,
        renew_cert_id.is_some()
    );

    // 拷贝所需数据
    let db = state.db.clone();
    let secrets = state.secrets.clone();
    let jobs = state.jobs.clone();
    let certs_root = state.certs_root.clone();
    // 任务结束后立即清理取消/确认标记（jobs 保留给前端查询，由兜底清理回收）
    let cancel_map = state.cancels.clone();
    let confirm_map = state.txt_confirms.clone();
    let cancel_map2 = cancel_map.clone();
    let confirm_map2 = confirm_map.clone();

    // 后台执行（整个流程在同一个 spawn_blocking 闭包内，保证 ctx 全程可用）
    let job_id_task = job_id.clone();
    let job_id_cleanup = job_id.clone();
    let db_task = db.clone();
    let app_task = app.clone();
    tauri::async_runtime::spawn(async move {
        let ctx = FlowCtx {
            db: db_task.clone(),
            secrets,
            app: app_task.clone(),
            req,
            job_id: job_id_task.clone(),
            jobs,
            cancel: cancel_flag.clone(),
            txt_confirmed: txt_confirmed_flag.clone(),
            provider,
            cert_id: renew_cert_id,
        };

        let _ = tauri::async_runtime::spawn_blocking(move || {
            let result = flow::run_order_job(&ctx);

            match result {
                Ok(bundle) => {
                    let write = crate::cert::store::write_bundle(
                        &certs_root,
                        &ctx.req.domain,
                        &bundle.fullchain_pem,
                        &bundle.private_key_pem,
                    );
                    match write {
                        Ok((chain, key)) => {
                            // 生成 IIS 格式（PFX）+ 说明记事本。
                            // PFX 生成失败仅告警不影响 PEM 结果；此时说明文件不再写
                            // 「cert.pfx / 密码 123456」段落，避免文件与实际不符（M11）
                            let pfx_ok = if let Some(dir) = chain.parent() {
                                match crate::cert::store::write_pfx(
                                    dir,
                                    &bundle.fullchain_pem,
                                    &bundle.private_key_pem,
                                    crate::cert::store::IIS_PFX_PASSWORD,
                                ) {
                                    Ok(_) => true,
                                    Err(e) => {
                                        log::warn!("pfx generation failed for {}: {e}", ctx.req.domain);
                                        false
                                    }
                                }
                            } else {
                                false
                            };
                            if let Some(dir) = chain.parent() {
                                if let Err(e) = crate::cert::store::write_readme(
                                    dir,
                                    &ctx.req.domain,
                                    crate::cert::store::IIS_PFX_PASSWORD,
                                    pfx_ok,
                                ) {
                                    log::warn!("readme generation failed for {}: {e}", ctx.req.domain);
                                }
                            }
                            let renew_after = parse_not_after(&bundle.not_after)
                                .map(|dt| (dt - chrono::Duration::days(30)).to_rfc3339());
                            let conn = db_task.lock();
                            if let Some(cid) = renew_cert_id {
                                // 续期：更新现有记录（DB 更新失败视为任务失败，不再静默吞掉）
                                match crate::storage::certificates::update_after_renew(
                                    &conn,
                                    cid,
                                    &chain.to_string_lossy(),
                                    &key.to_string_lossy(),
                                    &bundle.not_after,
                                    &renew_after.unwrap_or_else(|| bundle.not_after.clone()),
                                ) {
                                    Ok(_) => {
                                        drop(conn);
                                        flow::finish_job(&ctx, JobState::Completed, None, Some(cid));
                                        let _ = crate::storage::logs::finish(&db_task.lock(), log_id, "completed", None, None);
                                        // 续期成功通知（内部会再加锁读设置，需先释放上面的锁）
                                        crate::notify::renew_success(&ctx.app, &db_task, &ctx.req.domain, &bundle.not_after);
                                    }
                                    Err(e) => {
                                        let ae = AppError::from(e);
                                        drop(conn);
                                        let _ = crate::storage::logs::finish(&db_task.lock(), log_id, "failed", Some(ae.code.as_str()), Some(&ae.message));
                                        flow::finish_job(&ctx, JobState::Failed, Some(&ae), None);
                                    }
                                }
                            } else {
                                // 新申请：插入记录
                                let row = crate::storage::certificates::CertRow {
                                    id: 0,
                                    domain: ctx.req.domain.clone(),
                                    alt_names: ctx.req.alt_names.clone(),
                                    challenge_type: ctx.req.challenge_type.clone(),
                                    provider_id: ctx.req.provider_id,
                                    directory: ctx.req.directory.clone(),
                                    status: crate::storage::certificates::CertStatus::Issued,
                                    cert_chain_path: chain.to_string_lossy().into(),
                                    private_key_path: key.to_string_lossy().into(),
                                    issuer: Some(bundle.issuer.clone()),
                                    issued_at: chrono::Utc::now().to_rfc3339(),
                                    expires_at: bundle.not_after.clone(),
                                    renew_after: renew_after.clone(),
                                    last_renewal_at: None,
                                    last_error: None,
                                    fail_streak: 0,
                                    order_url: None,
                                    contact_email: Some(ctx.req.contact_email.clone()),
                                };
                                match crate::storage::certificates::insert(&conn, &row) {
                                    Ok(new_id) => {
                                        flow::finish_job(&ctx, JobState::Completed, None, Some(new_id));
                                        let _ = crate::storage::logs::finish(&conn, log_id, "completed", None, None);
                                    }
                                    Err(e) => {
                                        let ae = AppError::from(e);
                                        flow::finish_job(&ctx, JobState::Failed, Some(&ae), None);
                                        let _ = crate::storage::logs::finish(&conn, log_id, "failed", Some(ae.code.as_str()), Some(&ae.message));
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            let _ = crate::storage::logs::finish(&db_task.lock(), log_id, "failed", Some(e.code.as_str()), Some(&e.message));
                            flow::finish_job(&ctx, JobState::Failed, Some(&e), None);
                        }
                    }
                }
                Err(e) => {
                    if e.code == crate::error::ErrorCode::Canceled {
                        let _ = crate::storage::logs::finish(&db_task.lock(), log_id, "canceled", Some(e.code.as_str()), Some(&e.message));
                        // 取消续期任务时恢复证书状态，避免卡在 renewing（按钮隐藏 + 调度器跳过）
                        if let Some(cid) = renew_cert_id {
                            let conn = db_task.lock();
                            let _ = crate::storage::certificates::update_status(
                                &conn,
                                cid,
                                crate::storage::certificates::CertStatus::Issued,
                                None,
                            );
                            drop(conn);
                        }
                        flow::finish_job(&ctx, JobState::Canceled, None, None);
                    } else {
                        let conn = db_task.lock();
                        let fail_streak = renew_cert_id.map(|cid| {
                            let _ = crate::storage::certificates::update_status(
                                &conn,
                                cid,
                                crate::storage::certificates::CertStatus::Issued,
                                Some(&format!("{}: {}", e.code.as_str(), e.message)),
                            );
                            crate::storage::certificates::bump_fail_streak(&conn, cid).unwrap_or(1)
                        });
                        drop(conn);
                        if let Some(streak) = fail_streak {
                            // 续期失败通知（取消的任务不通知；新申请失败由向导页展示）
                            crate::notify::renew_failed(
                                &ctx.app,
                                &db_task,
                                &ctx.req.domain,
                                &format!("{}: {}", e.code.as_str(), e.message),
                                Some(streak),
                            );
                        }
                        let _ = crate::storage::logs::finish(&db_task.lock(), log_id, "failed", Some(e.code.as_str()), Some(&e.message));
                        flow::finish_job(&ctx, JobState::Failed, Some(&e), None);
                    }
                }
            }
        })
        .await;
        // 任务结束（无论成败/取消），立即清理取消与确认标记，防止残留
        cancel_map.lock().unwrap_or_else(|e| e.into_inner()).remove(&job_id_task);
        confirm_map.lock().unwrap_or_else(|e| e.into_inner()).remove(&job_id_task);
    });

    // 兜底清理 jobs 等标记（防内存增长）。
    // 长任务（手动 DNS 等待 / 多域名验证等可能超过 20 分钟）期间不清理，
    // 仅在任务已结束时回收，避免前端进度与并发防护失效（轻微问题 5）。
    let jobs_map = state.jobs.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(1200)).await;
            let still_running = jobs_map
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .get(&job_id_cleanup)
                .map(|j| j.state == JobState::Running || j.state == JobState::Pending)
                .unwrap_or(false);
            if !still_running {
                cancel_map2.lock().unwrap_or_else(|e| e.into_inner()).remove(&job_id_cleanup);
                confirm_map2.lock().unwrap_or_else(|e| e.into_inner()).remove(&job_id_cleanup);
                jobs_map.lock().unwrap_or_else(|e| e.into_inner()).remove(&job_id_cleanup);
                break;
            }
        }
    });

    Ok(job_id)
}

fn parse_not_after(s: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    chrono::DateTime::parse_from_rfc3339(s).ok().map(|d| d.with_timezone(&chrono::Utc))
}

/// 取消任务
#[tauri::command(rename_all = "camelCase")]
pub fn cancel_issue(job_id: String, state: State<'_, AppState>) -> AppResult<()> {
    let cancels = state.cancels.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(flag) = cancels.get(&job_id) {
        flag.store(true, Ordering::SeqCst);
        log::info!("issue canceled: job={job_id}");
    }
    Ok(())
}

/// 查询任务状态
#[tauri::command(rename_all = "camelCase")]
pub fn get_job_status(job_id: String, state: State<'_, AppState>) -> AppResult<Option<JobStatus>> {
    let jobs = state.jobs.lock().unwrap_or_else(|e| e.into_inner());
    Ok(jobs.get(&job_id).cloned())
}

/// 用户确认已手动添加 DNS TXT 记录（DNS 手动模式）
#[tauri::command(rename_all = "camelCase")]
pub fn confirm_txt(job_id: String, state: State<'_, AppState>) -> AppResult<()> {
    let confirms = state.txt_confirms.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(flag) = confirms.get(&job_id) {
        flag.store(true, Ordering::SeqCst);
        log::info!("dns manual confirmed: job={job_id}");
    }
    Ok(())
}
