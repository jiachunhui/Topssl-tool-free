//! 应用共享状态

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use crate::acme::model::JobStatus;
use crate::secret::keyring::SecretStore;
use crate::storage::Db;

pub struct AppState {
    pub db: Arc<Db>,
    pub secrets: Arc<SecretStore>,
    /// 运行中任务的最后状态（内存）
    pub jobs: Arc<Mutex<HashMap<String, JobStatus>>>,
    /// 取消标记
    pub cancels: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
    /// DNS 手动模式：用户确认已添加 TXT 记录
    pub txt_confirms: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
    /// 更新下载的取消标记
    pub update_cancel: Arc<AtomicBool>,
    /// 应用数据目录
    pub app_data_dir: PathBuf,
    /// 证书根目录
    pub certs_root: PathBuf,
    /// 平台标识
    pub platform: String,
}

impl AppState {
    pub fn new(app_data_dir: PathBuf, platform: String) -> Result<Self, crate::error::AppError> {
        let db_path = app_data_dir.join("ssl_cert.db");
        let db = Db::open(&db_path)?;
        // 启动恢复：上次运行中断的续期任务（内存态已丢失）回滚，交给调度器重试
        match crate::storage::certificates::reset_interrupted_renewals(&db.lock()) {
            Ok(n) if n > 0 => log::warn!("startup: recovered {n} interrupted renewal(s)"),
            Ok(_) => {}
            Err(e) => log::error!("startup: failed to recover interrupted renewals: {e}"),
        }
        let secrets = SecretStore::new(&app_data_dir);
        let certs_root = app_data_dir.join("certs");
        Ok(Self {
            db: Arc::new(db),
            secrets: Arc::new(secrets),
            jobs: Arc::new(Mutex::new(HashMap::new())),
            cancels: Arc::new(Mutex::new(HashMap::new())),
            txt_confirms: Arc::new(Mutex::new(HashMap::new())),
            update_cancel: Arc::new(AtomicBool::new(false)),
            app_data_dir,
            certs_root,
            platform,
        })
    }
}
