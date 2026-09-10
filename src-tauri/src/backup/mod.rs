//! 数据迁移备份包：口令加密的单文件容器
//!
//! 用途：把设置、证书文件、DNS 凭据与 ACME 账户密钥打包成一个文件，换电脑后导入即可继续使用。
//! 定位是「迁移/快照」而非实时同步——导入得到的是导出那一刻的副本。
//!
//! 容器布局（字节序）：
//!   MAGIC(4) | version(1) | iterations(4, BE) | salt(16) | nonce(12) | tag(16) | ciphertext
//! MAGIC..nonce 整段作为 AES-GCM 的 AAD 参与认证，防止加密参数被篡改；tag 独立存放，
//! 不与密文拼接（openssl 的 `symm::encrypt` 会丢弃 GCM 的 tag，必须用 `encrypt_aead`）。
//!
//! 明文载荷是 JSON：`VACUUM INTO` 产生的数据库快照 + 每张证书目录下的文件 + 密钥条目。
//! 密钥的枚举**由数据库驱动**（`dns_providers.secret_ref` 与按 directory/邮箱推导的
//! `acme_account:*` 键名），而不是遍历密钥文件——macOS/Linux 的 keyring 没有枚举 API，
//! 且原始 `secrets.bin` 是 DPAPI 用户绑定的，跨机无法解密，必须解密后重新加密。

use std::path::{Path, PathBuf};

use base64::Engine;
use openssl::hash::MessageDigest;
use openssl::pkcs5::pbkdf2_hmac;
use openssl::rand::rand_bytes;
use openssl::symm::{decrypt_aead, encrypt_aead, Cipher};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::acme::model::JobState;
use crate::error::{AppError, AppResult, ErrorCode};
use crate::state::AppState;

/// 备份包文件标识
const MAGIC: &[u8; 4] = b"TSBK";
/// 载荷内的格式标识
const FORMAT_NAME: &str = "tossl-backup";
/// 当前格式版本
const FORMAT_VERSION: u8 = 1;
/// PBKDF2 迭代次数（新导出的包固定用此值；导入时以包头声明的值为准）
const ITERATIONS: usize = 600_000;
/// 包头声明迭代次数的上界，避免伪造的超大值拖死导入
const MAX_ITERATIONS: usize = 5_000_000;

const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;
const TAG_LEN: usize = 16;
const KEY_LEN: usize = 32;

const OFF_VERSION: usize = 4;
const OFF_ITERATIONS: usize = OFF_VERSION + 1;
const OFF_SALT: usize = OFF_ITERATIONS + 4;
const OFF_NONCE: usize = OFF_SALT + SALT_LEN;
const OFF_TAG: usize = OFF_NONCE + NONCE_LEN;
/// 头部总长（含 tag）：此长度之后是密文
const HEADER_LEN: usize = OFF_TAG + TAG_LEN;

/// 备份包扩展名
pub const BACKUP_EXT: &str = "tosslbak";
/// 口令最短长度
pub const MIN_PASSWORD_LEN: usize = 8;
/// 数据库版本：备份包的 user_version 高于此值则拒绝导入（迁移是单向的）
const DB_VERSION: i64 = 3;
/// 备份包大小上限，防止异常文件把内存吃满
const MAX_BLOB_LEN: usize = 64 * 1024 * 1024;

/// 备份包明文载荷
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BackupPayload {
    format: String,
    format_version: u8,
    app_version: String,
    created_at: String,
    /// 导出机器名（仅作提示，不参与逻辑）
    source_host: String,
    /// 数据库快照（`VACUUM INTO` 产物）
    db: String,
    /// 导出时的数据库 user_version（导入前校验；不依赖 VACUUM 是否保留该 pragma）
    db_version: i64,
    certs: Vec<CertBundle>,
    secrets: Vec<SecretEntry>,
}

/// 单张证书目录下的全部文件
#[derive(Debug, Serialize, Deserialize)]
struct CertBundle {
    domain: String,
    files: Vec<CertFile>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CertFile {
    name: String,
    /// 文件内容（base64）
    data: String,
}

/// 一条密钥（键名 + 明文）
#[derive(Debug, Serialize, Deserialize)]
struct SecretEntry {
    key: String,
    value: String,
}

/// 导入结果（回给前端做提示）
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSummary {
    pub cert_count: i64,
    pub provider_count: i64,
    pub secret_count: usize,
    /// 数据库里有记录、但备份包中缺少证书文件的数量
    pub missing_files: usize,
    /// 备份包的导出时间与来源机器（仅用于提示）
    pub created_at: String,
    pub source_host: String,
    /// 导入前的数据备份目录（导入错了可人工回退）
    pub safety_dir: String,
}

// ================= 容器：加解密 =================

fn derive_key(password: &str, salt: &[u8], iterations: usize) -> AppResult<Vec<u8>> {
    let mut key = vec![0u8; KEY_LEN];
    pbkdf2_hmac(
        password.as_bytes(),
        salt,
        iterations,
        MessageDigest::sha256(),
        &mut key,
    )
    .map_err(|e| AppError::new(ErrorCode::Backup, "备份密钥派生失败").detail(e.to_string()))?;
    Ok(key)
}

/// 用口令把明文封装成备份包字节
fn seal(password: &str, plaintext: &[u8]) -> AppResult<Vec<u8>> {
    if password.chars().count() < MIN_PASSWORD_LEN {
        return Err(AppError::new(
            ErrorCode::Backup,
            format!("备份口令至少需要 {MIN_PASSWORD_LEN} 位"),
        ));
    }

    let mut salt = [0u8; SALT_LEN];
    let mut nonce = [0u8; NONCE_LEN];
    rand_bytes(&mut salt)
        .map_err(|e| AppError::new(ErrorCode::Backup, "随机数生成失败").detail(e.to_string()))?;
    rand_bytes(&mut nonce)
        .map_err(|e| AppError::new(ErrorCode::Backup, "随机数生成失败").detail(e.to_string()))?;

    let key = derive_key(password, &salt, ITERATIONS)?;

    let mut header = Vec::with_capacity(HEADER_LEN);
    header.extend_from_slice(MAGIC);
    header.push(FORMAT_VERSION);
    header.extend_from_slice(&(ITERATIONS as u32).to_be_bytes());
    header.extend_from_slice(&salt);
    header.extend_from_slice(&nonce);

    let mut tag = [0u8; TAG_LEN];
    let ciphertext = encrypt_aead(
        Cipher::aes_256_gcm(),
        &key,
        Some(&nonce[..]),
        &header,
        plaintext,
        &mut tag,
    )
    .map_err(|e| AppError::new(ErrorCode::Backup, "备份加密失败").detail(e.to_string()))?;

    let mut out = header;
    out.extend_from_slice(&tag);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

/// 用口令打开备份包，返回明文
fn open(password: &str, blob: &[u8]) -> AppResult<Vec<u8>> {
    if blob.len() > MAX_BLOB_LEN {
        return Err(AppError::new(ErrorCode::Backup, "备份文件过大，无法导入"));
    }
    // 最小长度是 HEADER_LEN：GCM 对空明文不产生密文，此时包正好等于头部长度
    if blob.len() < HEADER_LEN || !blob.starts_with(MAGIC) {
        return Err(AppError::new(ErrorCode::Backup, "不是有效的 ToSSL 备份文件"));
    }
    let version = blob[OFF_VERSION];
    if version > FORMAT_VERSION {
        return Err(AppError::new(
            ErrorCode::Backup,
            "备份文件由更高版本的应用生成，请先升级本应用",
        ));
    }
    let iterations = u32::from_be_bytes([
        blob[OFF_ITERATIONS],
        blob[OFF_ITERATIONS + 1],
        blob[OFF_ITERATIONS + 2],
        blob[OFF_ITERATIONS + 3],
    ]) as usize;
    if iterations == 0 || iterations > MAX_ITERATIONS {
        return Err(AppError::new(ErrorCode::Backup, "备份文件的加密参数异常"));
    }

    let header = &blob[..OFF_TAG];
    let salt = &blob[OFF_SALT..OFF_NONCE];
    let nonce = &blob[OFF_NONCE..OFF_TAG];
    let tag = &blob[OFF_TAG..HEADER_LEN];
    let ciphertext = &blob[HEADER_LEN..];

    let key = derive_key(password, salt, iterations)?;
    decrypt_aead(
        Cipher::aes_256_gcm(),
        &key,
        Some(nonce),
        header,
        ciphertext,
        tag,
    )
    .map_err(|_| {
        // AEAD 认证失败无法区分「口令错」与「文件被改」，统一按口令错误提示（最常见原因）
        AppError::new(ErrorCode::BackupPassword, "备份口令错误，或备份文件已损坏")
    })
}

// ================= 导出 =================

/// 导出备份包到 `out_dir`，返回生成的备份文件路径
pub fn export(state: &AppState, password: &str, out_dir: &Path) -> AppResult<PathBuf> {
    if password.chars().count() < MIN_PASSWORD_LEN {
        return Err(AppError::new(
            ErrorCode::Backup,
            format!("备份口令至少需要 {MIN_PASSWORD_LEN} 位"),
        ));
    }

    let payload = build_payload(state)?;
    let json = serde_json::to_vec(&payload)?;
    let sealed = seal(password, &json)?;

    std::fs::create_dir_all(out_dir)
        .map_err(|e| AppError::new(ErrorCode::Backup, "无法创建导出目录").detail(e.to_string()))?;
    let name = format!(
        "ToSSL-backup-{}.{BACKUP_EXT}",
        chrono::Local::now().format("%Y%m%d-%H%M%S")
    );
    let path = out_dir.join(name);
    write_atomic(&path, &sealed)?;
    log::info!(
        "backup: 已导出 {} 张证书、{} 条密钥到 {}",
        payload.certs.len(),
        payload.secrets.len(),
        path.display()
    );
    Ok(path)
}

fn build_payload(state: &AppState) -> AppResult<BackupPayload> {
    // 先在锁内取出需要的元数据，随后立即释放锁再做文件/密钥 IO（与代码库既有约定一致）
    let (domains, secret_keys, db_version) = {
        let conn = state.db.lock();
        let certs = crate::storage::certificates::list(&conn)?;
        let providers = crate::storage::providers::list(&conn)?;
        let fallback_email = crate::storage::settings::get_string(&conn, "contact_email", "");
        let db_version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;

        let mut domains: Vec<String> = Vec::new();
        let mut keys: Vec<String> = Vec::new();
        for p in &providers {
            keys.push(p.secret_ref.clone());
        }
        for c in &certs {
            domains.push(c.domain.clone());
            // ACME 账户密钥的键名由 directory + 邮箱推导（与 acme/flow.rs 的 keyring_account_key 一致）；
            // 证书未记录邮箱时回退到全局 contact_email 设置
            let email = match c.contact_email.as_deref() {
                Some(e) if !e.is_empty() => e.to_string(),
                _ => fallback_email.clone(),
            };
            if !email.is_empty() {
                keys.push(format!("acme_account:{}:{}", c.directory, email));
            }
        }
        domains.sort();
        domains.dedup();
        keys.sort();
        keys.dedup();
        (domains, keys, db_version)
    };

    // 数据库快照以 base64 存入 JSON 载荷（与导入侧的 decode 对应）
    let db = base64::engine::general_purpose::STANDARD.encode(snapshot_db_bytes(state)?);

    let mut certs = Vec::new();
    for domain in &domains {
        let dir = crate::cert::store::cert_dir(&state.certs_root, domain);
        if !dir.is_dir() {
            log::warn!("backup: 证书目录不存在，跳过 {domain}");
            continue;
        }
        let mut files = Vec::new();
        let entries = std::fs::read_dir(&dir)
            .map_err(|e| AppError::new(ErrorCode::Backup, "读取证书目录失败").detail(e.to_string()))?;
        for entry in entries {
            let entry = entry
                .map_err(|e| AppError::new(ErrorCode::Backup, "读取证书目录失败").detail(e.to_string()))?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            // 跳过写临时文件
            if name.ends_with(".tmp") {
                continue;
            }
            let data = std::fs::read(&path)
                .map_err(|e| AppError::new(ErrorCode::Backup, "读取证书文件失败").detail(e.to_string()))?;
            files.push(CertFile {
                name,
                data: base64::engine::general_purpose::STANDARD.encode(&data),
            });
        }
        files.sort_by(|a, b| a.name.cmp(&b.name));
        certs.push(CertBundle {
            domain: domain.clone(),
            files,
        });
    }

    let mut secrets = Vec::new();
    for key in &secret_keys {
        match state.secrets.load(key) {
            Ok(Some(value)) => secrets.push(SecretEntry {
                key: key.clone(),
                value,
            }),
            Ok(None) => log::warn!("backup: 密钥不存在，跳过 {key}"),
            Err(e) => return Err(e),
        }
    }

    Ok(BackupPayload {
        format: FORMAT_NAME.to_string(),
        format_version: FORMAT_VERSION,
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        created_at: chrono::Local::now().to_rfc3339(),
        source_host: hostname(),
        db,
        db_version,
        certs,
        secrets,
    })
}

/// 导出机器名（仅作提示，取不到时留空）
fn hostname() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_default()
}

/// `VACUUM INTO` 生成 WAL 一致的单文件快照。
/// 不能直接拷贝 `ssl_cert.db`——WAL 模式下最新提交可能还在 -wal 文件里。
fn vacuum_into(state: &AppState, dest: &Path) -> AppResult<()> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| AppError::new(ErrorCode::Backup, "无法创建备份目录").detail(e.to_string()))?;
    }
    // VACUUM INTO 要求目标文件不存在
    let _ = std::fs::remove_file(dest);
    // 用转义后的字面量而非参数绑定，避免不同 SQLite 版本对 VACUUM INTO 表达式支持差异
    let sql = format!("VACUUM INTO '{}'", dest.to_string_lossy().replace('\'', "''"));
    let conn = state.db.lock();
    conn.execute_batch(&sql)
        .map_err(|e| AppError::new(ErrorCode::Backup, "数据库快照失败").detail(e.to_string()))?;
    Ok(())
}

fn snapshot_db_bytes(state: &AppState) -> AppResult<Vec<u8>> {
    let tmp = std::env::temp_dir().join(format!("tossl-backup-{}.db", uuid::Uuid::new_v4()));
    vacuum_into(state, &tmp)?;
    let bytes = std::fs::read(&tmp)
        .map_err(|e| AppError::new(ErrorCode::Backup, "读取数据库快照失败").detail(e.to_string()))?;
    let _ = std::fs::remove_file(&tmp);
    Ok(bytes)
}

// ================= 导入 =================

/// 导入备份包（覆盖当前设置、证书与密钥）
pub fn import(state: &AppState, password: &str, blob: &[u8]) -> AppResult<ImportSummary> {
    let plain = open(password, blob)?;
    let payload: BackupPayload = serde_json::from_slice(&plain)
        .map_err(|e| AppError::new(ErrorCode::Backup, "备份文件内容无法解析").detail(e.to_string()))?;
    if payload.format != FORMAT_NAME {
        return Err(AppError::new(ErrorCode::Backup, "不是有效的 ToSSL 备份文件"));
    }
    if payload.format_version > FORMAT_VERSION {
        return Err(AppError::new(
            ErrorCode::Backup,
            "备份文件由更高版本的应用生成，请先升级本应用",
        ));
    }
    if payload.db_version > DB_VERSION {
        return Err(AppError::new(
            ErrorCode::Backup,
            "备份包的数据库版本比当前应用更新，请先升级本应用再导入",
        ));
    }
    // 正在申请/续期的任务持有旧证书 id，导入会让它们写到错误的记录上
    ensure_no_running_jobs(state)?;
    log::info!(
        "backup: 导入包由应用 {} 于 {} 导出（来源 {}）",
        payload.app_version,
        payload.created_at,
        payload.source_host
    );

    let db_bytes = base64::engine::general_purpose::STANDARD
        .decode(&payload.db)
        .map_err(|e| AppError::new(ErrorCode::Backup, "备份包内的数据库快照已损坏").detail(e.to_string()))?;
    let snapshot = std::env::temp_dir().join(format!("tossl-import-{}.db", uuid::Uuid::new_v4()));
    std::fs::write(&snapshot, &db_bytes)
        .map_err(|e| AppError::new(ErrorCode::Backup, "无法写入临时文件").detail(e.to_string()))?;

    let result = restore(state, &snapshot, &payload);
    let _ = std::fs::remove_file(&snapshot);
    result
}

fn restore(state: &AppState, snapshot: &Path, payload: &BackupPayload) -> AppResult<ImportSummary> {
    // 1) 先把当前数据整体备份出来，导入错了可人工回退
    let safety_dir = create_safety_backup(state)?;
    log::info!("backup: 导入前已备份当前数据到 {}", safety_dir.display());

    // 2) 数据库：事务内整体替换，失败自动回滚
    let (cert_count, provider_count) = import_db(state, snapshot)?;

    // 3) 证书文件
    for bundle in &payload.certs {
        let dir = crate::cert::store::cert_dir(&state.certs_root, &bundle.domain);
        for file in &bundle.files {
            // 只接受纯文件名，防止包内构造出 ../ 写到目录之外
            if file.name.is_empty()
                || file.name.contains('/')
                || file.name.contains('\\')
                || file.name.contains("..")
            {
                return Err(AppError::new(ErrorCode::Backup, "备份包内含非法文件名"));
            }
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(&file.data)
                .map_err(|e| {
                    AppError::new(ErrorCode::Backup, "备份包内的证书文件已损坏").detail(e.to_string())
                })?;
            write_atomic(&dir.join(&file.name), &bytes)?;
        }
    }

    // 统计有记录但没有证书文件的条目（导出时目录就缺失的情况）
    let missing_files = {
        let conn = state.db.lock();
        crate::storage::certificates::list(&conn)?
            .into_iter()
            .filter(|c| !payload.certs.iter().any(|b| b.domain == c.domain))
            .count()
    };

    // 4) 密钥：按同名键写回，Windows 上会用本机 DPAPI 重新加密
    for entry in &payload.secrets {
        state.secrets.save(&entry.key, &entry.value)?;
    }

    log::info!(
        "backup: 导入完成，{cert_count} 张证书、{provider_count} 个 DNS 服务商、{} 条密钥",
        payload.secrets.len()
    );

    Ok(ImportSummary {
        cert_count,
        provider_count,
        secret_count: payload.secrets.len(),
        missing_files,
        created_at: payload.created_at.clone(),
        source_host: payload.source_host.clone(),
        safety_dir: safety_dir.to_string_lossy().into_owned(),
    })
}

/// 把快照中的数据整体替换进当前数据库。ATTACH 必须在事务之外（SQLite 不允许事务内 ATTACH），
/// 因此这里分两步：先挂载，再在事务内搬运，最后无论如何都尝试卸载。
fn import_db(state: &AppState, snapshot: &Path) -> AppResult<(i64, i64)> {
    let mut conn = state.db.lock();
    let attach = format!(
        "ATTACH DATABASE '{}' AS bak",
        snapshot.to_string_lossy().replace('\'', "''")
    );
    conn.execute_batch(&attach)
        .map_err(|e| AppError::new(ErrorCode::Backup, "无法读取备份包内的数据库").detail(e.to_string()))?;

    let result = copy_from_attachment(state, &mut conn);
    let _ = conn.execute_batch("DETACH DATABASE bak");
    result
}

fn copy_from_attachment(state: &AppState, conn: &mut Connection) -> AppResult<(i64, i64)> {
    let db_version: i64 = conn
        .query_row("PRAGMA bak.user_version", [], |r| r.get(0))
        .map_err(|e| AppError::new(ErrorCode::Backup, "无法识别备份包的数据库版本").detail(e.to_string()))?;
    if db_version > DB_VERSION {
        return Err(AppError::new(
            ErrorCode::Backup,
            "备份包由更高版本的应用生成，请先升级本应用",
        ));
    }

    let cert_count: i64 = conn.query_row("SELECT COUNT(*) FROM bak.certificates", [], |r| r.get(0))?;
    let provider_count: i64 = conn.query_row("SELECT COUNT(*) FROM bak.dns_providers", [], |r| r.get(0))?;

    let settings: Vec<(String, String)> = {
        let mut stmt = conn.prepare("SELECT key, value FROM bak.settings")?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };

    let tx = conn.transaction()?;
    // 外键约束（foreign_keys=ON）：先删子表，先插父表；保留原 id 才能维持 provider_id 引用
    tx.execute("DELETE FROM certificates", [])?;
    tx.execute("DELETE FROM dns_providers", [])?;
    // 通知去重记录按 cert_id 关联；证书 id 已被整体替换，残留记录会错误地抑制新证书的到期提醒
    tx.execute("DELETE FROM notifications", [])?;
    tx.execute(
        "INSERT INTO dns_providers (id, kind, label, config_json, secret_ref, enabled, created_at, updated_at)
         SELECT id, kind, label, config_json, secret_ref, enabled, created_at, updated_at FROM bak.dns_providers",
        [],
    )?;
    tx.execute(
        "INSERT INTO certificates (id, domain, alt_names, challenge_type, provider_id, directory, status,
             cert_chain_path, private_key_path, issuer, issued_at, expires_at, renew_after, last_renewal_at,
             last_error, order_url, created_at, updated_at, fail_streak, contact_email)
         SELECT id, domain, alt_names, challenge_type, provider_id, directory, status,
             cert_chain_path, private_key_path, issuer, issued_at, expires_at, renew_after, last_renewal_at,
             last_error, order_url, created_at, updated_at, fail_streak, contact_email FROM bak.certificates",
        [],
    )?;

    // 证书路径列存的是绝对路径，换机后必然指向旧机器，必须改写为新机器的证书目录
    let rows: Vec<(i64, String)> = {
        let mut stmt = tx.prepare("SELECT id, domain FROM certificates")?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };
    for (id, domain) in rows {
        let dir = crate::cert::store::cert_dir(&state.certs_root, &domain);
        tx.execute(
            "UPDATE certificates SET cert_chain_path=?1, private_key_path=?2 WHERE id=?3",
            rusqlite::params![
                dir.join("fullchain.pem").to_string_lossy().into_owned(),
                dir.join("privkey.pem").to_string_lossy().into_owned(),
                id
            ],
        )?;
    }

    // 设置：只覆盖与机器无关的用户设置；本机专属键保留目标机器自己的值
    for (key, value) in &settings {
        if is_local_setting(key) {
            continue;
        }
        tx.execute(
            "INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, datetime('now'))
             ON CONFLICT(key) DO UPDATE SET value=?2, updated_at=datetime('now')",
            rusqlite::params![key, value],
        )?;
    }
    // 两台机器同时开启自动续期会重复签发同一域名、并可能互相干扰 DNS TXT 校验，
    // 导入后强制关闭，由用户决定在哪一台机器上开启
    tx.execute(
        "INSERT INTO settings (key, value, updated_at) VALUES ('auto_renew', 'false', datetime('now'))
         ON CONFLICT(key) DO UPDATE SET value='false', updated_at=datetime('now')",
        [],
    )?;

    tx.commit()?;
    Ok((cert_count, provider_count))
}

/// 本机专属设置：换机后应保留目标机器自己的值，不从备份包覆盖
fn is_local_setting(key: &str) -> bool {
    matches!(key, "run_at_login" | "auto_renew" | "last_check_at") || key.starts_with("updater.")
}

/// 有任务在跑时导入会让它写到刚被替换掉的证书记录上，直接拒绝
fn ensure_no_running_jobs(state: &AppState) -> AppResult<()> {
    let jobs = state.jobs.lock().unwrap_or_else(|e| e.into_inner());
    let running = jobs
        .values()
        .any(|j| matches!(j.state, JobState::Pending | JobState::Running));
    drop(jobs);
    if running {
        return Err(AppError::new(
            ErrorCode::Backup,
            "有正在进行的申请或续期任务，请等它结束后再导入",
        ));
    }
    Ok(())
}

/// 导入前把当前数据备份到 `app_data_dir/pre-import-<时间戳>/`
fn create_safety_backup(state: &AppState) -> AppResult<PathBuf> {
    let dir = state.app_data_dir.join(format!(
        "pre-import-{}",
        chrono::Local::now().format("%Y%m%d-%H%M%S")
    ));
    std::fs::create_dir_all(&dir)
        .map_err(|e| AppError::new(ErrorCode::Backup, "无法创建备份目录").detail(e.to_string()))?;

    vacuum_into(state, &dir.join("ssl_cert.db"))?;
    copy_dir(&state.certs_root, &dir.join("certs"))?;

    let secrets_name = if cfg!(windows) { "secrets.bin" } else { "secrets.json" };
    let secrets_src = state.app_data_dir.join(secrets_name);
    if secrets_src.is_file() {
        let _ = std::fs::copy(&secrets_src, dir.join(secrets_name));
    }
    Ok(dir)
}

fn copy_dir(src: &Path, dst: &Path) -> AppResult<()> {
    if !src.is_dir() {
        return Ok(());
    }
    std::fs::create_dir_all(dst)
        .map_err(|e| AppError::new(ErrorCode::Backup, "无法创建备份目录").detail(e.to_string()))?;
    for entry in std::fs::read_dir(src)
        .map_err(|e| AppError::new(ErrorCode::Backup, "读取证书目录失败").detail(e.to_string()))?
    {
        let entry = entry
            .map_err(|e| AppError::new(ErrorCode::Backup, "读取证书目录失败").detail(e.to_string()))?;
        let target = dst.join(entry.file_name());
        let is_dir = entry
            .file_type()
            .map_err(|e| AppError::new(ErrorCode::Backup, "读取目录项失败").detail(e.to_string()))?
            .is_dir();
        if is_dir {
            copy_dir(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), &target)
                .map_err(|e| AppError::new(ErrorCode::Backup, "复制文件失败").detail(e.to_string()))?;
        }
    }
    Ok(())
}

/// 临时文件 + 重命名写入，避免部分写入
fn write_atomic(path: &Path, bytes: &[u8]) -> AppResult<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| AppError::new(ErrorCode::Backup, "无法创建证书目录").detail(e.to_string()))?;
    }
    let mut name = path
        .file_name()
        .map(|n| n.to_os_string())
        .unwrap_or_else(|| std::ffi::OsString::from("tmp"));
    name.push(".tmp");
    let tmp = path.with_file_name(name);

    std::fs::write(&tmp, bytes)
        .map_err(|e| AppError::new(ErrorCode::Backup, "写入文件失败").detail(e.to_string()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600));
    }
    std::fs::rename(&tmp, path)
        .map_err(|e| AppError::new(ErrorCode::Backup, "保存文件失败").detail(e.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 加解密往返：同一口令能还原明文
    #[test]
    fn seal_open_roundtrip() {
        let plain = b"hello \x00\x01 binary".to_vec();
        let sealed = seal("password123", &plain).unwrap();
        assert_eq!(&sealed[..4], MAGIC);
        assert!(sealed.len() > HEADER_LEN);
        assert_eq!(open("password123", &sealed).unwrap(), plain);
    }

    /// 口令错误必须失败（AEAD 认证），不能返回垃圾明文
    #[test]
    fn open_with_wrong_password_fails() {
        let sealed = seal("password123", b"secret").unwrap();
        let err = open("password124", &sealed).unwrap_err();
        assert_eq!(err.code, ErrorCode::BackupPassword);
    }

    /// 头部被篡改（迭代次数）必须被认证拦住
    #[test]
    fn tampered_header_fails() {
        let mut sealed = seal("password123", b"secret").unwrap();
        sealed[OFF_ITERATIONS] ^= 0xFF;
        assert!(open("password123", &sealed).is_err());
    }

    /// 密文被篡改必须被认证拦住
    #[test]
    fn tampered_ciphertext_fails() {
        let mut sealed = seal("password123", b"secret payload").unwrap();
        let last = sealed.len() - 1;
        sealed[last] ^= 0xFF;
        assert!(open("password123", &sealed).is_err());
    }

    /// 口令长度下限
    #[test]
    fn short_password_rejected() {
        assert!(seal("short", b"x").is_err());
    }

    /// 非备份文件应给出明确的「格式不对」而不是口令错误
    #[test]
    fn non_backup_file_rejected() {
        let err = open("password123", b"not a backup at all").unwrap_err();
        assert_eq!(err.code, ErrorCode::Backup);
    }

    /// 本机专属设置不参与导入
    #[test]
    fn local_settings_are_skipped() {
        assert!(is_local_setting("run_at_login"));
        assert!(is_local_setting("auto_renew"));
        assert!(is_local_setting("last_check_at"));
        assert!(is_local_setting("updater.dismissed"));
        assert!(!is_local_setting("contact_email"));
        assert!(!is_local_setting("http01_port"));
    }

    /// 每次加密的 salt/nonce 必须不同：GCM 的 nonce 复用会泄露明文且可被伪造
    #[test]
    fn seal_is_randomized() {
        let a = seal("password123", b"same").unwrap();
        let b = seal("password123", b"same").unwrap();
        assert_ne!(a, b, "两次加密的结果不应相同");
        assert_ne!(
            &a[OFF_NONCE..OFF_TAG],
            &b[OFF_NONCE..OFF_TAG],
            "nonce 必须每次随机"
        );
        assert_eq!(open("password123", &a).unwrap(), open("password123", &b).unwrap());
    }

    /// 空载荷往返（回归：此前最小长度判断成 HEADER_LEN+1，空明文解不开）
    #[test]
    fn seal_open_empty_payload() {
        let sealed = seal("password123", b"").unwrap();
        assert_eq!(sealed.len(), HEADER_LEN, "空明文的包应正好等于头部长度");
        assert_eq!(open("password123", &sealed).unwrap(), Vec::<u8>::new());
    }

    /// 口令长度按字符计：8 个汉字属于合法口令
    #[test]
    fn multibyte_password_ok() {
        let pw = "中文口令八个字节"; // 8 个字符
        assert_eq!(pw.chars().count(), 8);
        let sealed = seal(pw, b"data").unwrap();
        assert_eq!(open(pw, &sealed).unwrap(), b"data".to_vec());
        // 7 个字符应被拒绝
        assert!(seal("中文口令八个字", b"data").is_err());
    }

    /// 头部迭代次数为 0 或超上限时必须拒绝，避免伪造值拖死导入
    #[test]
    fn bogus_iterations_rejected() {
        let mut zero = seal("password123", b"x").unwrap();
        zero[OFF_ITERATIONS..OFF_SALT].copy_from_slice(&0u32.to_be_bytes());
        assert_eq!(open("password123", &zero).unwrap_err().code, ErrorCode::Backup);

        let mut huge = seal("password123", b"x").unwrap();
        huge[OFF_ITERATIONS..OFF_SALT].copy_from_slice(&(MAX_ITERATIONS as u32 + 1).to_be_bytes());
        assert_eq!(open("password123", &huge).unwrap_err().code, ErrorCode::Backup);
    }

    /// 接近真实备份包量级的载荷往返（含数据库快照与多张证书时的量级）
    #[test]
    fn large_payload_roundtrip() {
        let plain: Vec<u8> = (0..3_000_000u32).map(|i| (i % 251) as u8).collect();
        let sealed = seal("password123", &plain).unwrap();
        assert_eq!(open("password123", &sealed).unwrap(), plain);
    }

    /// 载荷是 JSON（真实编码路径），数据库快照走 base64 字段
    #[test]
    fn json_payload_roundtrip() {
        #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
        struct P {
            format: String,
            db: String,
            secrets: Vec<String>,
        }
        let db_bytes = vec![0u8, 1, 2, 250, 255];
        let payload = P {
            format: FORMAT_NAME.to_string(),
            db: base64::engine::general_purpose::STANDARD.encode(&db_bytes),
            secrets: vec!["dns_provider:abc".into()],
        };
        let json = serde_json::to_vec(&payload).unwrap();
        let sealed = seal("password123", &json).unwrap();

        let back: P = serde_json::from_slice(&open("password123", &sealed).unwrap()).unwrap();
        assert_eq!(back, payload);
        assert_eq!(
            base64::engine::general_purpose::STANDARD.decode(&back.db).unwrap(),
            db_bytes
        );
    }
}
