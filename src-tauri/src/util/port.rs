//! 端口探测工具

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PortStatus {
    pub free: bool,
    pub code: &'static str, // 'free' | 'busy' | 'privilege' | 'error'
    pub detail: Option<String>,
}

/// 探测端口是否可绑定（尝试 TcpListener::bind）
pub fn probe_port(port: u16) -> PortStatus {
    match std::net::TcpListener::bind(("0.0.0.0", port)) {
        Ok(listener) => {
            // 立即关闭释放端口
            drop(listener);
            PortStatus { free: true, code: "free", detail: None }
        }
        Err(e) => {
            let code = match e.kind() {
                std::io::ErrorKind::PermissionDenied => "privilege",
                std::io::ErrorKind::AddrInUse => "busy",
                _ => "error",
            };
            PortStatus {
                free: false,
                code,
                detail: Some(if code == "privilege" {
                    "无权限监听该端口".to_string()
                } else {
                    e.to_string()
                }),
            }
        }
    }
}

/// 平台相关的 80 端口权限说明（Linux 需 setcap；Windows 需注意 http.sys 保留）
pub fn http01_privilege_note(platform: &str) -> Option<String> {
    if platform == "linux" {
        Some(
            "提示：Linux 下普通用户无法监听 80 端口，请执行一次授权（应用安装路径需替换为实际路径）：\n\
             sudo setcap cap_net_bind_service=+ep <应用可执行文件路径>\n\
             或在设置中将 HTTP 验证端口改为高位端口（如 8080）并配合反向代理。"
                .to_string(),
        )
    } else if platform == "windows" {
        Some(
            "提示：Windows 下 80 端口若被系统服务保留（IIS / http.sys / 远程管理服务），HTTP 验证会失败。\
             可在「服务」中停止「万维网发布服务」（World Wide Web Publishing Service）后重试，或改用 DNS 验证。"
                .to_string(),
        )
    } else {
        None
    }
}
