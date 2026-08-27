# TopSSL 免费证书助手（TopSSL Free Cert Assistant）

面向普通用户的跨平台桌面程序：输入域名 → 一键申请 **Let's Encrypt 免费 SSL 证书** → 自动安装到本机 → 到期自动续期。

由 [TopSSL（topssl.cn）](https://www.topssl.cn/?utm_source=github&utm_medium=readme&utm_campaign=top) 出品并支持，免费工具部分完全开源。

技术栈：**Tauri 2**（Rust 后端）+ **Vue 3 + Vite + TypeScript + Tailwind CSS v4**（前端）+ **SQLite**（rusqlite）。

> 详细使用说明见 [docs/用户手册.md](docs/用户手册.md)。
>
> **相关链接**：[TopSSL 官网](https://www.topssl.cn/?utm_source=github&utm_medium=readme&utm_campaign=top) ｜ [GitHub 仓库](https://github.com/jiachunhui/Topssl-tool-free) ｜ [ToSSL 官网（下载/教程/知识库）](https://www.tossl.cn) ｜ [Let's Encrypt](https://letsencrypt.org/)

## 功能特性

- ✅ 一键申请：输入域名，几分钟内获得 Let's Encrypt 证书
- ✅ 两种验证方式：
  - **DNS-01**：自动调用 DNS 服务商 API 添加 TXT 记录，无需 80 端口（推荐，国内通用），支持通配符证书
  - **HTTP-01**：自动在 80 端口临时开启验证服务，适用于云服务器等 80 端口公网可访问的环境
- ✅ **通配符证书**（`*.example.com`）、多域名 SAN 证书
- ✅ 证书安装到本机，提供 nginx / Apache 等使用指引
- ✅ **自动续期**：证书 90 天有效期，到期前 30 天自动续期（开机自启 + 托盘常驻）
- ✅ 测试环境（Staging）与正式环境切换，规避速率限制
- ✅ 跨平台：Windows / macOS / Linux
- ✅ 密钥安全存储：Windows DPAPI / macOS Keychain / Linux Secret Service
- ✅ **应用内检查更新**：启动静默检查新版本（国内源优先 + GitHub Releases 兜底），一键下载安装包并自动运行安装（Windows）

## 下载与安装

各平台安装包发布在 [GitHub Releases](https://github.com/jiachunhui/Topssl-tool-free/releases)（现已开源）。应用内也会自动检查更新：启动时静默检查（国内更新源优先，不可用时自动回退 GitHub），发现新版本弹窗提示，点击「立即更新」即自动下载安装包并启动安装程序；也可在「关于」页手动检查。

| 平台 | 安装包 | 说明 |
|---|---|---|
| Windows 10/11 x64 | `TopSSL-Free-Cert-Assistant_<版本>_x64-setup.exe` | NSIS 安装程序，双击安装（当前用户，无需管理员） |
| macOS（Apple Silicon） | `TopSSL-Free-Cert-Assistant_<版本>_aarch64.dmg` | 未签名：首次打开请在 Finder 中**右键 → 打开**，或在终端执行 `xattr -cr /Applications/TopSSL-Free-Cert-Assistant.app` |
| macOS（Intel） | `TopSSL-Free-Cert-Assistant_<版本>_x64.dmg` | 同上 |
| Linux x64 | `.deb` / `.AppImage` / `.rpm` | Debian 系：`sudo dpkg -i xxx.deb`；AppImage：`chmod +x` 后直接运行 |

## 支持的 DNS 服务商

| 服务商 | 凭证 | 说明 |
|---|---|---|
| 阿里云 DNS | AccessKey ID + Secret | RAM 子账号需 `AliyunDNSFullAccess` 权限 |
| DNSPod / 腾讯云 | API Token（ID,Token） | 旧版 dnsapi.cn Token |
| Cloudflare | API Token | 需 `Zone:DNS:Edit` 权限 |

## 开发环境

### 前置依赖

| 依赖 | 版本 | 说明 |
|---|---|---|
| Node.js | ≥ 20.19（推荐 22+） | 前端构建 |
| Rust | ≥ 1.77（stable） | 后端，`rustup` 安装 |
| MSVC Build Tools | VS 2022/2026 | Windows 编译（C++ 桌面开发工作负载） |
| WebView2 | Win10/11 自带 | Tauri 渲染 |
| **Perl + NASM** | Strawberry Perl | **Windows 构建必需**（openssl vendored 源码构建用） |
| Linux 系统库 | webkit2gtk-4.1 等 | 见下方 Linux 说明 |

### 启动开发

```bash
npm install
npm run tauri dev        # 前端热更新 + Rust 调试构建
```

### 构建安装包

```bash
npm run tauri build      # 产出 NSIS（Windows）、dmg（macOS）、deb/rpm/AppImage（Linux）
```

### 常见构建问题

- **openssl vendored 需要 perl/nasm**（Windows）：安装 [Strawberry Perl](https://strawberryperl.com/) 与 [NASM](https://www.nasm.us/)：`choco install strawberryperl nasm -y`。
- **Rust 找不到 MSVC linker**：安装 Visual Studio Build Tools 并勾选「使用 C++ 的桌面开发」工作负载。
- **Linux 缺系统库**（Ubuntu/Debian）：
  ```bash
  sudo apt-get install -y libwebkit2gtk-4.1-dev libayatana-appindicator3-dev \
    librsvg2-dev patchelf build-essential curl wget file libxdo-dev libssl-dev
  ```
- 首次编译耗时较长（openssl、sqlite 源码编译），请耐心等待。

## 自动构建与发布（GitHub Actions）

仓库内置 `.github/workflows/release.yml` 三平台构建流水线：

- 推送 `v*` 标签：自动构建 Windows（NSIS）、macOS（ARM + Intel dmg）、Linux（deb/AppImage/rpm）并创建 GitHub Release 挂载全部安装包；发布前先校验 tag 版本与代码内版本号一致（不一致直接失败）
- 手动触发（Actions → Release → Run workflow）：仅构建并产出构建产物（Artifacts），不创建 Release

### 发版流程（版本号与 GitHub Releases 同步）

版本号统一由脚本同步（`package.json` / `tauri.conf.json` / `Cargo.toml` / `Cargo.lock` / 宣传页配置 / 用户手册六处），**不要手工修改**：

```bash
# 1. 一键同步版本号（如 0.1.4）
npm run version:set -- 0.1.4

# 2. 提交代码后打 tag 推送，触发三平台构建 + 自动创建 GitHub Release
git tag v0.1.4 && git push origin v0.1.4

# 3. 构建产物下载后，生成国内更新清单（应用内「检查更新」的数据源）
npm run update:manifest -- --base-url https://你的域名/download \
  --win src-tauri/target/release/bundle/nsis/TopSSL-Free-Cert-Assistant_0.1.4_x64-setup.exe \
  --notes-file CHANGELOG.md

# 4. 把 updates/latest.json 上传到虚拟主机 /updates/，安装包上传到 /download/；
#    域名确定后在 src-tauri/src/updater/mod.rs 填入 UPDATE_MANIFEST_URL 即可启用国内源
```

> 国内源未配置时应用自动使用 GitHub API 作为更新源；配置后国内清单优先、失败自动回退 GitHub。

## Linux 的 80 端口权限

Linux 普通用户无法监听 1024 以下端口。使用 HTTP-01 验证前需执行一次性授权：

```bash
sudo setcap cap_net_bind_service=+ep /path/to/topssl-free-cert-assistant
```

或在设置中改用 DNS 验证。

## 证书文件位置

| 平台 | 路径 |
|---|---|
| Windows | `%APPDATA%\com.topsl.ssl-cert-desktop\certs\{域名}\` |
| macOS | `~/Library/Application Support/com.topsl.ssl-cert-desktop/certs/{域名}/` |
| Linux | `~/.local/share/com.topsl.ssl-cert-desktop/certs/{域名}/` |

每个证书目录包含 `fullchain.pem`（证书链）与 `privkey.pem`（私钥，权限 0600）。

## 项目结构

```
src/                    # 前端（Vue3 + TS + Tailwind）
  lib/                  # IPC 封装、事件、错误码中文映射、类型定义
  stores/               # Pinia 状态（app/certs/job/providers/settings）
  components/           # UI 组件、布局、证书卡片、申请向导
  views/                # 6 个页面：证书列表 / 申请向导 / DNS 配置 / 设置 / 日志 / 关于
src-tauri/              # Rust 后端
  src/acme/             # ACME 状态机（client/flow/model/limits）
  src/dns/              # DNS Provider（aliyun/dnspod/cloudflare）
  src/http01/           # HTTP-01 临时验证服务器
  src/storage/          # SQLite（迁移 + 表模块）
  src/cert/             # 证书落盘 / 解析 / 使用指引
  src/secret/           # keyring 机密存储 + Windows DPAPI
  src/scheduler/        # 自动续期调度
  src/updater/           # 应用内更新（检查/下载/安装）
  src/commands/         # Tauri IPC commands
  vendor/               # 本地补丁依赖（acme-micro / keyring）
.github/workflows/      # 三平台自动构建 + Release（含版本一致性校验）
scripts/                # 版本同步 / 更新清单生成脚本
docs/                   # 用户手册
```

## 本地补丁说明

仓库通过 `[patch.crates-io]` 引用了 `src-tauri/vendor/` 下的两个上游 crate 补丁：

- **acme-micro**：Let's Encrypt 新增 `dns-persist-01` 挑战类型（无 token 字段），上游 0.14.0 的 `ApiChallenge.token` 必填导致解析失败
- **keyring**：Windows 凭据持久化模式由 `CRED_PERSIST_ENTERPRISE`（会被系统清理）改为 `CRED_PERSIST_LOCAL_MACHINE`；注：Windows 端实际主存储为 DPAPI 加密文件，keyring 仅供 macOS/Linux 使用

## 关于 TopSSL

[TopSSL（topssl.cn）](https://www.topssl.cn/?utm_source=github&utm_medium=readme&utm_campaign=top) 是专业的 SSL 证书服务平台，提供：

- **企业级 SSL 证书**（DV / OV / EV）：地址栏显示企业身份，兼容旧设备，支持多年期购买
- **证书自动化部署方案**：CDN、云服务器、负载均衡等场景的部署与更新
- **专业技术支持**：申请、部署、续期全程人工协助

本应用由 TopSSL 出品并支持，免费工具部分完全开源；企业用户可按需了解 TopSSL 付费服务。

## 安全说明

- ACME 账户私钥、DNS API 密钥存入操作系统安全存储（DPAPI / Keychain / Secret Service），数据库仅存引用标识
- 证书私钥文件权限 0600
- 默认使用 Let's Encrypt **测试环境（Staging）**，正式证书需在向导中显式确认

## 许可证

[MIT](LICENSE)

## 免责声明

本项目仅用于申请 Let's Encrypt 免费证书。请遵守 [Let's Encrypt 服务条款](https://letsencrypt.org/repository/) 与速率限制（每域每周 50 张正式证书）。
