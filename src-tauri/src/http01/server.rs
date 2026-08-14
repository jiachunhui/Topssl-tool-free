//! HTTP-01 临时验证服务器
//!
//! 使用 std TcpListener + 线程实现，手写最小 HTTP/1.1 响应，
//! 支持动态注册/注销 token→proof 映射，可探测 80 端口占用/权限。
//! 该服务器在申请期间短暂运行，验证完成后立即停止。

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use crate::error::{AppError, ErrorCode};

type Registry = Arc<Mutex<HashMap<String, String>>>;

/// 并发连接上限（验证窗口很短，正常远低于此值）
const MAX_CONNS: usize = 64;

pub struct Http01Server {
    registry: Registry,
    stop: Arc<AtomicBool>,
    port: u16,
    handle: Option<JoinHandle<()>>,
}

impl Http01Server {
    /// 尝试在指定端口启动服务器。失败时区分"被占用/无权限/其他"
    pub fn start(port: u16) -> Result<Self, AppError> {
        let listener = match TcpListener::bind(("0.0.0.0", port)) {
            Ok(l) => l,
            Err(e) => {
                let code = match e.kind() {
                    std::io::ErrorKind::PermissionDenied => ErrorCode::Http01Privilege,
                    std::io::ErrorKind::AddrInUse => ErrorCode::Http01PortBusy,
                    _ => ErrorCode::Http01PortBusy,
                };
                let msg = match code {
                    ErrorCode::Http01Privilege => {
                        "无权限监听端口（Windows 下可能是 http.sys/IIS 或安全软件占用；Linux 需 setcap 授权），可改用 DNS 验证".to_string()
                    }
                    _ => format!("端口 {port} 被占用"),
                };
                return Err(AppError::new(code, msg).detail(e.to_string()));
            }
        };

        let registry: Registry = Arc::new(Mutex::new(HashMap::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let handle = spawn_serve(listener, registry.clone(), stop.clone());

        Ok(Self { registry, stop, port, handle: Some(handle) })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    /// 注册一个挑战 token → proof
    pub fn register(&self, token: &str, proof: &str) {
        self.registry.lock().unwrap_or_else(|e| e.into_inner()).insert(token.to_string(), proof.to_string());
    }

    pub fn unregister(&self, token: &str) {
        self.registry.lock().unwrap_or_else(|e| e.into_inner()).remove(token);
    }

    /// 停止服务器（注销所有 token）
    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        self.registry.lock().unwrap_or_else(|e| e.into_inner()).clear();
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }

    /// 本机自检：模拟 Let's Encrypt 的验证请求，确认挑战文件可达。
    /// `port` 为验证方视角的端口：LE 的 HTTP-01 始终访问 80 端口，
    /// 因此即便本机监听的是自定义端口（配合反向代理），自检也应连 80，
    /// 与 LE 行为一致，避免「自定义端口自检通过、LE 验证必然失败」的假阳性。
    pub fn self_check(&self, domain: &str, token: &str, port: u16) -> Result<(), AppError> {
        let proof = self
            .registry
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(token)
            .cloned()
            .ok_or_else(|| AppError::new(ErrorCode::Internal, "挑战 token 未注册"))?;

        let host = format!("{domain}:{port}");
        let path = format!("/.well-known/acme-challenge/{token}");
        let request = format!(
            "GET {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\nUser-Agent: ssl-cert-desktop/0.1\r\n\r\n"
        );

        let mut stream = TcpStream::connect((domain, port)).map_err(|e| {
            AppError::new(
                ErrorCode::Http01Unreachable,
                format!(
                    "无法连接本机 {host}：请确认域名解析到本机公网 IP、路由器/防火墙放行 80 端口（部分 NAT 环境本机回环不通属正常现象）",
                ),
            )
            .detail(e.to_string())
        })?;
        stream.set_read_timeout(Some(std::time::Duration::from_secs(5)))?;
        stream.write_all(request.as_bytes())?;
        let mut resp = String::new();
        stream.read_to_string(&mut resp)?;

        if resp.starts_with("HTTP/1.1 200") && resp.contains(&proof) {
            Ok(())
        } else {
            Err(AppError::new(
                ErrorCode::Http01Unreachable,
                "自检失败：验证响应不正确，请检查端口转发与防火墙",
            )
            .detail(resp.chars().take(200).collect::<String>()))
        }
    }
}

impl Drop for Http01Server {
    fn drop(&mut self) {
        self.stop();
    }
}

fn spawn_serve(listener: TcpListener, registry: Registry, stop: Arc<AtomicBool>) -> JoinHandle<()> {
    std::thread::spawn(move || {
        listener.set_nonblocking(true).ok();
        let active = Arc::new(AtomicUsize::new(0));
        while !stop.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((stream, _)) => {
                    // 超出并发上限直接丢弃连接，防止线程被耗尽
                    if active.fetch_add(1, Ordering::SeqCst) >= MAX_CONNS {
                        active.fetch_sub(1, Ordering::SeqCst);
                        drop(stream);
                        continue;
                    }
                    let registry = registry.clone();
                    let active = active.clone();
                    std::thread::spawn(move || {
                        handle_conn(stream, registry);
                        active.fetch_sub(1, Ordering::SeqCst);
                    });
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                Err(_) => break,
            }
        }
    })
}

fn handle_conn(mut stream: TcpStream, registry: Registry) {
    stream.set_read_timeout(Some(std::time::Duration::from_secs(5))).ok();
    let mut buf = [0u8; 1024];
    let n = match stream.read(&mut buf) {
        Ok(n) => n,
        Err(_) => return,
    };
    let req = String::from_utf8_lossy(&buf[..n]);
    let path = req
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .unwrap_or("/")
        .to_string();

    let body = path
        .strip_prefix("/.well-known/acme-challenge/")
        .and_then(|token| registry.lock().unwrap_or_else(|e| e.into_inner()).get(token).cloned());

    match body {
        Some(proof) => {
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                proof.len(),
                proof
            );
            let _ = stream.write_all(resp.as_bytes());
        }
        None => {
            let resp = "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
            let _ = stream.write_all(resp.as_bytes());
        }
    }
}
