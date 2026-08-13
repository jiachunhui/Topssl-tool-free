//! 机密信息存储
//!
//! - Windows：DPAPI 加密文件（secrets.bin）。该机器实测系统凭据管理器会
//!   自动清理应用写入的凭据（约 1 分钟内消失），故 Windows 端不使用凭据管理器，
//!   改用用户账户绑定的 DPAPI 加密文件。
//! - macOS / Linux：keyring（Keychain / Secret Service），无服务时降级明文文件 secrets.json

use std::path::PathBuf;

use crate::error::{AppError, AppResult, ErrorCode};

#[cfg(not(windows))]
const SERVICE: &str = "ssl-cert-desktop";

/// 机密存储服务封装
pub struct SecretStore {
    /// 本地文件路径（Windows: DPAPI 加密文件；unix: keyring 降级明文文件）
    file_path: Option<PathBuf>,
}

impl SecretStore {
    pub fn new(app_data_dir: &std::path::Path) -> Self {
        #[cfg(windows)]
        let file_path = Some(app_data_dir.join("secrets.bin"));
        #[cfg(not(windows))]
        let file_path = Some(app_data_dir.join("secrets.json"));
        Self { file_path }
    }

    // ================= Windows：DPAPI 加密文件 =================

    #[cfg(windows)]
    pub fn save(&self, key: &str, secret: &str) -> AppResult<()> {
        let mut map = self.load_encrypted_map();
        let blob = dpapi::protect(key, secret.as_bytes())?;
        use base64::Engine;
        let b64 = base64::engine::general_purpose::STANDARD.encode(blob);
        map.insert(key.to_string(), serde_json::Value::String(b64));
        self.save_encrypted_map(&map)
    }

    #[cfg(windows)]
    pub fn load(&self, key: &str) -> AppResult<Option<String>> {
        let map = self.load_encrypted_map();
        let Some(b64) = map.get(key).and_then(|v| v.as_str()) else {
            return Ok(None);
        };
        use base64::Engine;
        let blob = base64::engine::general_purpose::STANDARD
            .decode(b64)
            .map_err(|e| AppError::new(ErrorCode::Internal, "密钥文件损坏").detail(e.to_string()))?;
        let plain = dpapi::unprotect(key, &blob)?;
        String::from_utf8(plain)
            .map(Some)
            .map_err(|e| AppError::new(ErrorCode::Internal, "密钥文件损坏").detail(e.to_string()))
    }

    #[cfg(windows)]
    pub fn delete(&self, key: &str) -> AppResult<()> {
        let mut map = self.load_encrypted_map();
        if map.remove(key).is_some() {
            self.save_encrypted_map(&map)?;
        }
        Ok(())
    }

    #[cfg(windows)]
    fn load_encrypted_map(&self) -> serde_json::Map<String, serde_json::Value> {
        let Some(path) = &self.file_path else { return serde_json::Map::new() };
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    #[cfg(windows)]
    fn save_encrypted_map(&self, map: &serde_json::Map<String, serde_json::Value>) -> AppResult<()> {
        let Some(path) = &self.file_path else { return Ok(()) };
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).ok();
        }
        let json = serde_json::to_string_pretty(map)
            .map_err(|e| AppError::new(ErrorCode::Internal, "密钥序列化失败").detail(e.to_string()))?;
        // 临时文件 + 重命名，避免部分写入
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, json)
            .map_err(|e| AppError::new(ErrorCode::Internal, "密钥文件写入失败").detail(e.to_string()))?;
        std::fs::rename(&tmp, path)
            .map_err(|e| AppError::new(ErrorCode::Internal, "密钥文件保存失败").detail(e.to_string()))?;
        Ok(())
    }

    // ================= macOS / Linux：keyring + 降级文件 =================

    #[cfg(not(windows))]
    fn entry(key: &str) -> AppResult<keyring::Entry> {
        keyring::Entry::new(SERVICE, key)
            .map_err(|e| AppError::new(ErrorCode::Internal, "无法访问系统密钥存储").detail(e.to_string()))
    }

    #[cfg(not(windows))]
    pub fn save(&self, key: &str, secret: &str) -> AppResult<()> {
        let entry = Self::entry(key)?;
        match entry.set_password(secret) {
            Ok(()) => {
                // 读回校验：防止写入"假成功"导致密钥丢失
                match entry.get_password() {
                    Ok(p) if p == secret => Ok(()),
                    Ok(p) => {
                        log::warn!(
                            "keyring read-back mismatch (saved {} bytes, read {} bytes), falling back to file",
                            secret.len(),
                            p.len()
                        );
                        self.save_fallback(key, secret)
                    }
                    Err(e) => {
                        log::warn!("keyring read-back failed ({e}), falling back to file");
                        self.save_fallback(key, secret)
                    }
                }
            }
            Err(e) => {
                log::warn!("keyring set failed ({e}), falling back to file");
                self.save_fallback(key, secret)
            }
        }
    }

    #[cfg(not(windows))]
    pub fn load(&self, key: &str) -> AppResult<Option<String>> {
        let entry = Self::entry(key)?;
        match entry.get_password() {
            Ok(p) => Ok(Some(p)),
            Err(keyring::Error::NoEntry) => self.load_fallback(key),
            Err(e) => {
                log::warn!("keyring get failed ({e}), trying fallback");
                self.load_fallback(key)
            }
        }
    }

    #[cfg(not(windows))]
    pub fn delete(&self, key: &str) -> AppResult<()> {
        let entry = Self::entry(key)?;
        let _ = entry.delete_credential();
        self.delete_fallback(key);
        Ok(())
    }

    // ---------- unix 降级（明文文件，0600） ----------

    #[cfg(not(windows))]
    fn fallback_file(&self) -> Option<PathBuf> {
        self.file_path.clone()
    }

    #[cfg(not(windows))]
    fn load_map(&self) -> serde_json::Map<String, serde_json::Value> {
        let Some(path) = self.fallback_file() else { return serde_json::Map::new() };
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    #[cfg(not(windows))]
    fn save_map(&self, map: serde_json::Map<String, serde_json::Value>) -> std::io::Result<()> {
        let Some(path) = self.fallback_file() else { return Ok(()) };
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let json = serde_json::to_string_pretty(&map)
            .map_err(|e| std::io::Error::other(e))?;
        // 写入临时文件后重命名，避免部分写入
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, json)?;
        std::fs::rename(&tmp, &path)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
        }
        Ok(())
    }

    #[cfg(not(windows))]
    fn save_fallback(&self, key: &str, secret: &str) -> AppResult<()> {
        let mut map = self.load_map();
        map.insert(key.to_string(), serde_json::Value::String(secret.to_string()));
        self.save_map(map).map_err(|e| {
            AppError::new(ErrorCode::Internal, "密钥降级文件写入失败").detail(e.to_string())
        })
    }

    #[cfg(not(windows))]
    fn load_fallback(&self, key: &str) -> AppResult<Option<String>> {
        let map = self.load_map();
        Ok(map.get(key).and_then(|v| v.as_str()).map(|s| s.to_string()))
    }

    #[cfg(not(windows))]
    fn delete_fallback(&self, key: &str) {
        let mut map = self.load_map();
        if map.remove(key).is_some() {
            let _ = self.save_map(map);
        }
    }
}

/// 生成密钥文件路径（供 UI 提示）
pub fn fallback_hint() -> &'static str {
    "系统安全存储不可用，密钥将以加密文件形式保存在应用数据目录 secrets.bin / secrets.json 中"
}

pub fn err_internal(e: impl std::fmt::Display) -> AppError {
    AppError::internal(e)
}

// ================= Windows DPAPI 实现 =================

#[cfg(windows)]
mod dpapi {
    use crate::error::{AppError, ErrorCode};

    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Cryptography::{
        CryptProtectData, CryptUnprotectData, CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN,
    };

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    /// 用 DPAPI 加密（entropy 绑定密钥名，防同一文件内密文互换）
    pub fn protect(entropy: &str, plaintext: &[u8]) -> Result<Vec<u8>, AppError> {
        let in_blob = CRYPT_INTEGER_BLOB {
            cbData: plaintext.len() as u32,
            pbData: plaintext.as_ptr() as *mut u8,
        };
        let ent = wide(entropy);
        let ent_blob = CRYPT_INTEGER_BLOB {
            cbData: (ent.len() * 2) as u32,
            pbData: ent.as_ptr() as *mut u8,
        };
        let mut out_blob = CRYPT_INTEGER_BLOB { cbData: 0, pbData: std::ptr::null_mut() };
        let ok = unsafe {
            CryptProtectData(
                &in_blob,
                std::ptr::null_mut(),
                &ent_blob,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut out_blob,
            )
        };
        if ok == 0 {
            return Err(AppError::new(ErrorCode::Internal, "DPAPI 加密失败")
                .detail(std::io::Error::last_os_error().to_string()));
        }
        let data =
            unsafe { std::slice::from_raw_parts(out_blob.pbData, out_blob.cbData as usize) }.to_vec();
        unsafe { LocalFree(out_blob.pbData as *mut core::ffi::c_void) };
        Ok(data)
    }

    /// DPAPI 解密
    pub fn unprotect(entropy: &str, blob: &[u8]) -> Result<Vec<u8>, AppError> {
        let in_blob = CRYPT_INTEGER_BLOB {
            cbData: blob.len() as u32,
            pbData: blob.as_ptr() as *mut u8,
        };
        let ent = wide(entropy);
        let ent_blob = CRYPT_INTEGER_BLOB {
            cbData: (ent.len() * 2) as u32,
            pbData: ent.as_ptr() as *mut u8,
        };
        let mut out_blob = CRYPT_INTEGER_BLOB { cbData: 0, pbData: std::ptr::null_mut() };
        let ok = unsafe {
            CryptUnprotectData(
                &in_blob,
                std::ptr::null_mut(),
                &ent_blob,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut out_blob,
            )
        };
        if ok == 0 {
            return Err(AppError::new(ErrorCode::Internal, "DPAPI 解密失败")
                .detail(std::io::Error::last_os_error().to_string()));
        }
        let data =
            unsafe { std::slice::from_raw_parts(out_blob.pbData, out_blob.cbData as usize) }.to_vec();
        unsafe { LocalFree(out_blob.pbData as *mut core::ffi::c_void) };
        Ok(data)
    }

    #[cfg(test)]
    #[test]
    fn dpapi_roundtrip() {
        let plain = b"secret value 123 abc XYZ";
        let blob = protect("test-key", plain).unwrap();
        let back = unprotect("test-key", &blob).unwrap();
        assert_eq!(back, plain);
        // 不同 entropy 解不开
        assert!(unprotect("other-key", &blob).is_err());
    }
}
