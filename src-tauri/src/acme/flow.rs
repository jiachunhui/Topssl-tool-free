//! ACME 申请流程状态机驱动
//!
//! 整个订单生命周期在一个 tokio::task::spawn_blocking 闭包内运行
//! （acme-micro 是同步 API，且中间对象非 Send，不能跨线程拆分）。
//! async 副作用（DNS Provider / 自检）通过 tokio Handle::block_on 桥接。

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use acme_micro::Directory;
use tauri::Emitter;

use crate::acme::client::{self, map_acme_error, AcmeResult};
use crate::acme::model::{CertBundle, IssueRequest, IssueStage, JobProgress, JobState, JobStatus};
use crate::error::{AppError, ErrorCode};
use crate::secret::keyring::SecretStore;
use crate::storage::Db;

/// 任务运行上下文
pub struct FlowCtx {
    pub db: Arc<Db>,
    pub secrets: Arc<SecretStore>,
    pub app: tauri::AppHandle,
    pub req: IssueRequest,
    pub job_id: String,
    pub jobs: Arc<Mutex<HashMap<String, JobStatus>>>,
    pub cancel: Arc<AtomicBool>,
    /// DNS 手动模式：用户确认已手动添加 TXT 记录
    pub txt_confirmed: Arc<AtomicBool>,
    /// DNS-01 时的 provider（由外层构建）
    pub provider: Option<Box<dyn crate::dns::DnsProvider>>,
    /// 续期目标证书记录 id（新申请为 None；随任务状态透出，用于并发去重与前端展示）
    pub cert_id: Option<i64>,
}

impl FlowCtx {
    pub fn set_status(&self, st: JobStatus) {
        if let Ok(mut map) = self.jobs.lock() {
            map.insert(self.job_id.clone(), st);
        }
    }
}

/// 更新进度：写内存状态 + 发事件
pub fn report(ctx: &FlowCtx, stage: IssueStage, percent: u8, message: &str) -> Result<(), AppError> {
    check_cancel(ctx)?;
    // 每个阶段都写日志，便于定位卡住/超时发生在哪一步
    log::info!("job {} stage {:?}: {message}", ctx.job_id, stage);
    let p = JobProgress {
        job_id: ctx.job_id.clone(),
        stage,
        percent,
        message: message.to_string(),
        detail: None,
    };
    if let Ok(mut map) = ctx.jobs.lock() {
        map.insert(
            ctx.job_id.clone(),
            JobStatus {
                job_id: ctx.job_id.clone(),
                state: JobState::Running,
                stage: Some(stage),
                percent,
                message: Some(message.to_string()),
                error_code: None,
                error_detail: None,
                cert_id: ctx.cert_id,
                domain: Some(ctx.req.domain.clone()),
            },
        );
    }
    let _ = ctx.app.emit("acme://job-progress", &p);
    Ok(())
}

pub fn check_cancel(ctx: &FlowCtx) -> Result<(), AppError> {
    if ctx.cancel.load(Ordering::SeqCst) {
        return Err(AppError::new(ErrorCode::Canceled, "任务已取消"));
    }
    Ok(())
}

fn keyring_account_key(directory: &str, email: &str) -> String {
    format!("acme_account:{directory}:{email}")
}

/// 获取或创建 ACME 账户（账户私钥存 keyring）
fn get_or_create_account(ctx: &FlowCtx, dir: &Directory) -> AcmeResult<acme_micro::Account> {
    let email = ctx.req.contact_email.trim();
    let directory = &ctx.req.directory;
    let key_ref = keyring_account_key(directory, email);

    if let Some(pem) = ctx.secrets.load(&key_ref)? {
        match client::load_account(dir, &pem, email) {
            Ok(acc) => return Ok(acc),
            Err(_) => {
                // 账户密钥失效，重新注册
            }
        }
    }
    let pem = client::register_account(dir, email)?;
    ctx.secrets.save(&key_ref, &pem)?;
    client::load_account(dir, &pem, email)
}

/// 执行完整订单流程，返回证书 bundle
pub fn run_order_job(ctx: &FlowCtx) -> AcmeResult<CertBundle> {
    let req = &ctx.req;

    // 1. 目录就绪
    report(ctx, IssueStage::DirectoryReady, 8, "连接 Let's Encrypt 服务…")?;
    let dir = client::connect(&req.directory)?;

    // 2. 账户
    report(ctx, IssueStage::AccountRegistered, 16, "注册 / 加载账户…")?;
    let account = get_or_create_account(ctx, &dir)?;

    // 3. 创建订单（alt_names 需转 &[&str]）
    report(ctx, IssueStage::OrderCreated, 24, "创建证书订单…")?;
    let alt_refs: Vec<&str> = req.alt_names.iter().map(|s| s.as_str()).collect();
    let mut order = account
        .new_order(&req.domain, &alt_refs)
        .map_err(|e| map_acme_error(e, "创建订单失败"))?;

    // 4. 拉取授权
    report(ctx, IssueStage::AuthorizationsFetched, 32, "获取域名授权…")?;
    let auths = order
        .authorizations()
        .map_err(|e| map_acme_error(e, "获取域名授权失败"))?;
    if auths.is_empty() {
        return Err(AppError::new(ErrorCode::OrderCreate, "未获取到任何授权"));
    }

    // 5. 逐个域名完成挑战
    let http01_port = crate::storage::settings::get_i64(&ctx.db.lock(), "http01_port", 80).clamp(1, 65535) as u16;

    // HTTP-01 时先启动临时服务器（同步操作）
    let http_server = if req.challenge_type == "http01" {
        report(ctx, IssueStage::ChallengePrepared, 38, "启动 80 端口验证服务…")?;
        if http01_port != 80 {
            // B7：HTTP-01 协议固定走 80 端口，自定义端口仅配合反向代理有效
            report(
                ctx,
                IssueStage::ChallengePrepared,
                38,
                &format!("注意：Let's Encrypt 的 HTTP-01 验证始终访问 80 端口，当前配置监听 {http01_port} 端口，请确认 80 端口已转发到该端口（反向代理），否则验证会失败"),
            )?;
        }
        Some(crate::http01::server::Http01Server::start(http01_port)?)
    } else {
        None
    };

    // DNS-01 记录已添加的 TXT（便于验证完成后清理）
    let mut added_txts: Vec<(String, String, String)> = Vec::new();

    // 5. 两阶段完成挑战（与 certbot/lego 等标准客户端一致）：
    //    阶段 1 一次性准备所有授权的验证材料（DNS：连续添加全部 TXT 记录；
    //    手动模式：一次性展示全部记录）。同一记录名下多条 TXT 必须同批添加——
    //    否则本机/ISP 解析器会缓存旧答案（TTL 最长 600s），后添加的记录在
    //    等待窗口内"永远不可见"（真实案例：*.域名 + 基础域名，两条授权同名不同值）。
    //    阶段 2 再逐条等待生效并验证。
    let mut manual_txts: Vec<serde_json::Value> = Vec::new();
    for (i, auth) in auths.iter().enumerate() {
        check_cancel(ctx)?;
        let domain = auth.domain_name().to_string();
        // 多 SAN 时按序号推进进度，封顶 70（单个证书最多 100 个域名，
        // 不封顶会 u8 溢出导致 debug panic / release 进度回绕）
        let base = 42u8.saturating_add((i as u8).saturating_mul(8)).min(70);

        if req.challenge_type == "http01" {
            let server = http_server.as_ref().unwrap();
            let chall = auth.http_challenge().ok_or_else(|| {
                AppError::new(ErrorCode::ChallengeUnsupported, "该域名不支持 HTTP 验证")
            })?;
            let token = chall.http_token().to_string();
            let proof = chall.http_proof().map_err(|e| map_acme_error(e, "生成验证内容失败"))?;
            server.register(&token, &proof);
            report(ctx, IssueStage::ChallengeServed, base, &format!("为 {domain} 响应验证请求…"))?;
        } else {
            let chall = auth.dns_challenge().ok_or_else(|| {
                AppError::new(ErrorCode::ChallengeUnsupported, "该域名不支持 DNS 验证")
            })?;
            let record_name = format!("_acme-challenge.{domain}");
            let value = chall.dns_proof().map_err(|e| map_acme_error(e, "生成验证内容失败"))?;
            let bare = crate::util::domain::bare_domain(&domain);

            if ctx.req.dns_manual {
                manual_txts.push(serde_json::json!({
                    "domain": bare,
                    "recordName": record_name,
                    "value": value,
                }));
            } else {
                let provider = ctx.provider.as_ref().ok_or_else(|| {
                    AppError::new(ErrorCode::DnsProviderApi, "未选择 DNS 服务商")
                })?;
                report(ctx, IssueStage::ChallengeServed, base, &format!("向 DNS 添加验证记录 {record_name}…"))?;
                // 添加失败时清理本批次已添加的记录
                if let Err(e) = tokio_block(async {
                    provider.add_txt(&bare, &record_name, &value).await
                }) {
                    cleanup_added_txts(ctx, &added_txts);
                    return Err(e);
                }
                added_txts.push((record_name.clone(), value.clone(), bare));
            }
        }
    }

    // 手动模式：一次性展示全部 TXT 记录，等待一次确认
    if ctx.req.dns_manual && !manual_txts.is_empty() {
        report(
            ctx,
            IssueStage::ChallengeServed,
            42,
            &format!("请添加 {} 条 TXT 记录", manual_txts.len()),
        )?;
        let _ = ctx.app.emit(
            "acme://txt-needed",
            serde_json::json!({
                "jobId": ctx.job_id,
                "records": manual_txts,
            }),
        );
        wait_txt_confirmation(ctx)?;
    }

    // 阶段 2：逐个等待生效并验证（任何失败都会清理已添加的记录）
    let validate_result = (|| -> AcmeResult<()> {
        for (i, auth) in auths.iter().enumerate() {
            check_cancel(ctx)?;
            let domain = auth.domain_name().to_string();
            let base = 42u8.saturating_add((i as u8).saturating_mul(8)).min(70);

            if req.challenge_type == "http01" {
                let server = http_server.as_ref().unwrap();
                let chall = auth.http_challenge().ok_or_else(|| {
                    AppError::new(ErrorCode::ChallengeUnsupported, "该域名不支持 HTTP 验证")
                })?;
                let token = chall.http_token().to_string();
                // 本机自检（模拟 LE 的验证请求，按 LE 视角连 80 端口）。
                // 自检失败不再硬性阻断：hairpin NAT 等环境下本机回环不通但公网可达，
                // 降级为警告提示，继续交由 LE 验证（LE 失败会给出具体原因，B4/B7）。
                match server.self_check(&crate::util::domain::bare_domain(&domain), &token, 80) {
                    Ok(_) => {
                        report(
                            ctx,
                            IssueStage::ValidationInProgress,
                            base + 3,
                            &format!("等待 Let's Encrypt 验证 {domain}…"),
                        )?;
                    }
                    Err(e) => {
                        log::warn!("http01 self-check failed for {domain}: {}", e.message);
                        report(
                            ctx,
                            IssueStage::ValidationInProgress,
                            base + 3,
                            &format!("本机自检未通过（{}），继续等待 Let's Encrypt 验证 {domain}…", e.message),
                        )?;
                    }
                }
                chall
                    .validate(Duration::from_millis(5000))
                    .map_err(|e| map_acme_error(e, &format!("{domain} 验证失败")))?;
                server.unregister(&token);
                report(ctx, IssueStage::Validated, base + 5, &format!("{domain} 验证通过"))?;
            } else {
                let chall = auth.dns_challenge().ok_or_else(|| {
                    AppError::new(ErrorCode::ChallengeUnsupported, "该域名不支持 DNS 验证")
                })?;
                let record_name = format!("_acme-challenge.{domain}");
                let value = chall.dns_proof().map_err(|e| map_acme_error(e, "生成验证内容失败"))?;
                report(ctx, IssueStage::ValidationInProgress, base + 2, "等待 DNS 记录生效…")?;
                tokio_block(async {
                    crate::dns::wait_propagation(&record_name, &value, Duration::from_secs(120)).await
                })?;
                report(
                    ctx,
                    IssueStage::ValidationInProgress,
                    base + 4,
                    &format!("等待 Let's Encrypt 验证 {domain}…"),
                )?;
                chall
                    .validate(Duration::from_millis(5000))
                    .map_err(|e| map_acme_error(e, &format!("{domain} 验证失败")))?;
                report(ctx, IssueStage::Validated, base + 6, &format!("{domain} 验证通过"))?;
            }
        }
        Ok(())
    })();

    if let Err(e) = validate_result {
        cleanup_added_txts(ctx, &added_txts);
        return Err(e);
    }

    // 6. 确认校验（带总超时，避免订单长时间停留在 pending 导致任务永不结束，B8）
    report(ctx, IssueStage::Validated, 76, "确认所有域名验证结果…")?;
    let confirm_deadline = std::time::Instant::now() + Duration::from_secs(90);
    let csr_order = loop {
        check_cancel(ctx)?;
        if std::time::Instant::now() > confirm_deadline {
            return Err(AppError::new(
                ErrorCode::OrderCreate,
                "等待订单确认超时（90 秒），请稍后重试",
            )
            .detail("所有域名验证已通过，但订单长时间未就绪"));
        }
        if let Some(o) = order.confirm_validations() {
            break o;
        }
        order.refresh().map_err(|e| map_acme_error(e, "刷新订单失败"))?;
    };

    // 7. 提交 CSR
    report(ctx, IssueStage::CsrSubmitted, 82, "提交证书签发请求…")?;
    let key_type = crate::storage::settings::get_string(&ctx.db.lock(), "cert_key_type", "rsa");
    let key = client::create_key(&key_type)?;
    log::info!("job {} CSR key type: {key_type}", ctx.job_id);
    let cert_order = csr_order
        .finalize_pkey(key, Duration::from_millis(5000))
        .map_err(|e| {
            log::error!("finalize error: {e}");
            AppError::new(ErrorCode::FinalizeFailed, "证书签发失败").detail(e.to_string())
        })?;

    // 8. 下载证书
    report(ctx, IssueStage::CertReady, 90, "证书签发完成，下载中…")?;
    let cert = cert_order
        .download_cert()
        .map_err(|e| {
            log::error!("cert download error: {e}");
            AppError::new(ErrorCode::CertDownload, "证书下载失败").detail(e.to_string())
        })?;
    let fullchain_pem = cert.certificate().to_string();
    let private_key_pem = cert.private_key().to_string();
    report(ctx, IssueStage::CertDownloaded, 95, "证书下载完成")?;

    // 清理 DNS TXT 记录（失败仅记日志，不阻断）
    cleanup_added_txts(ctx, &added_txts);

    // 解析证书信息
    let parsed = crate::cert::parser::parse_bundle(&fullchain_pem)?;

    Ok(CertBundle {
        fullchain_pem,
        private_key_pem,
        issuer: parsed.issuer,
        order_url: None,
        not_after: parsed.not_after,
    })
}

/// 在 spawn_blocking 闭包内桥接 async 操作
pub fn tokio_block<F, R>(fut: F) -> Result<R, AppError>
where
    F: std::future::Future<Output = Result<R, AppError>>,
{
    let handle = tokio::runtime::Handle::current();
    handle.block_on(fut)
}

/// 清理已添加的 TXT 记录（AUTO 模式；成功与失败路径共用，仅记日志不阻断）
fn cleanup_added_txts(ctx: &FlowCtx, added_txts: &[(String, String, String)]) {
    if let Some(provider) = &ctx.provider {
        for (record_name, value, bare) in added_txts {
            if let Err(e) = tokio_block(async {
                provider.remove_txt(bare, record_name, value).await
            }) {
                log::warn!("清理 TXT 记录失败 {record_name}: {e}");
            }
        }
    }
}

/// 等待用户确认已手动添加 TXT 记录（最长 10 分钟）
fn wait_txt_confirmation(ctx: &FlowCtx) -> Result<(), AppError> {
    let deadline = std::time::Instant::now() + Duration::from_secs(600);
    loop {
        if ctx.txt_confirmed.load(Ordering::SeqCst) {
            return Ok(());
        }
        check_cancel(ctx)?;
        if std::time::Instant::now() > deadline {
            return Err(AppError::new(
                ErrorCode::DnsPropagationTimeout,
                "等待手动添加 TXT 记录超时（10 分钟）",
            )
            .detail("请确认已在 DNS 控制台添加记录后重试"));
        }
        std::thread::sleep(Duration::from_secs(2));
    }
}

/// 任务结束（成功/失败/取消）后更新状态并广播事件
pub fn finish_job(ctx: &FlowCtx, state: JobState, error: Option<&AppError>, cert_id: Option<i64>) {
    let (code, detail) = match error {
        Some(e) => (Some(e.code.as_str().to_string()), e.detail.clone()),
        None => (None, None),
    };
    let st = JobStatus {
        job_id: ctx.job_id.clone(),
        state,
        stage: if state == JobState::Completed { Some(IssueStage::Completed) } else { None },
        percent: if state == JobState::Completed { 100 } else { 0 },
        message: None,
        error_code: code.clone(),
        error_detail: detail.clone(),
        cert_id,
        domain: Some(ctx.req.domain.clone()),
    };
    ctx.set_status(st);
    // 事件携带完整结果（状态/证书 id/错误），前端无需回查即可展示失败原因
    let _ = ctx.app.emit(
        "acme://job-finished",
        serde_json::json!({
            "job_id": ctx.job_id,
            "ok": state == JobState::Completed,
            "state": state,
            "cert_id": cert_id,
            "error_code": code,
            "error_detail": detail,
        }),
    );
    let _ = ctx.app.emit("certs://changed", ());
}
