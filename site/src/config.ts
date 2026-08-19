// 站点统一配置：上线前只需修改这里
// ─────────────────────────────────────────────────────────────
// ★ 唯一需要替换的占位值：SITE.url（下方）。它同时驱动：
//   canonical / og:url / sitemap.xml / robots.txt / JSON-LD
// 其余文件（astro.config.mjs、robots.txt 等）均自动引用此值，
// 上线前无需再改任何其他文件。
// ─────────────────────────────────────────────────────────────
export const SITE = {
  name: 'TopSSL 免费证书助手',
  tagline: '免费 SSL 证书一键申请，到期自动续期',
  description:
    'TopSSL 免费证书助手是一款由 TopSSL 出品的开源跨平台桌面工具：输入域名即可免费申请 Let’s Encrypt SSL 证书，自动安装到本机并到期自动续期，支持 HTTP/DNS 验证与通配符证书。',
  // TODO: 上线前替换为最终域名（如 https://www.topssl.cn/ssl-cert-assistant 或独立域名）
  url: 'https://topssl-cert-assistant.example.com',
  lang: 'zh-CN',
  /** 站点 Logo（用于 Organization 结构化数据），上线前随域名一并替换 */
  logo: '/apple-touch-icon.png',
  /** 品牌同源链接（sameAs） */
  sameAs: ['https://github.com/jiachunhui/Topssl-tool-free'],
  /** 当前版本（与 Cargo.toml 保持一致） */
  version: '0.1.7',
  /** 软件下载入口（GitHub Releases 页） */
  downloadUrl: 'https://github.com/jiachunhui/Topssl-tool-free/releases',
  /** 更新日志页（站内） */
  releaseNotes: '/changelog/',
  /** 软件界面截图（用于 SoftwareApplication.screenshot） */
  screenshots: ['/screenshots/certs-view.png'],
  /** 百度搜索资源平台验证（可选）：填入站点验证 meta 的 content 值 */
  baiduSiteVerification: '',
  /** 站点统计（可选）：百度统计等，留空则不注入任何脚本 */
  analytics: { baiduTongjiId: '' },
}

export const TOPSSL = {
  name: 'TopSSL',
  home: 'https://www.topssl.cn',
  /** 落地页内 TopSSL 链接统一携带 UTM，便于统计转化来源 */
  url: (campaign: string): string =>
    `https://www.topssl.cn/?utm_source=topssl-cert-assistant-site&utm_medium=referral&utm_campaign=${encodeURIComponent(campaign)}`,
}

export const GITHUB = 'https://github.com/jiachunhui/Topssl-tool-free'

/** TopSSL 企业证书起售价（元/年），价格变动时只需修改此处 */
export const ENTERPRISE_PRICE = 45

export const FAQS = [
  {
    q: '证书是免费的吗？申请需要付费吗？',
    a: '完全免费。本工具调用 Let’s Encrypt 免费证书签发服务，输入域名即可申请，到期前自动续期，整个过程不收取任何费用。',
  },
  {
    q: '证书有效期多久？会自动续期吗？',
    a: 'Let’s Encrypt 证书有效期为 90 天。应用会在到期前 30 天自动续期（需保持应用后台运行，默认开启开机自启与托盘常驻）。',
  },
  {
    q: '支持通配符证书吗？',
    a: '支持。输入 *.example.com 即可申请通配符证书，一张证书覆盖主域名及所有子域名。通配符证书只能使用 DNS 验证方式。',
  },
  {
    q: 'DNS API 密钥安全吗？',
    a: '密钥仅保存在本机系统安全存储中（Windows DPAPI / macOS 钥匙串 / Linux Secret Service），仅在申请与续期时用于添加/删除 TXT 验证记录，不会上传到任何服务器。',
  },
  {
    q: '支持哪些 DNS 服务商？',
    a: '内置阿里云 DNS、DNSPod（腾讯云）、Cloudflare 三家主流服务商的 API 自动验证，填入 API 密钥后自动添加与清理 TXT 记录；其他服务商可使用手动添加解析方式完成 DNS 验证。',
  },
  {
    q: '服务器 80 端口被占用或不可用，还能申请证书吗？',
    a: '可以。改用 DNS-01 验证方式即可：应用调用 DNS 服务商 API 添加 TXT 验证记录，全程不依赖 80 端口，也适用于内网服务器、CDN 前置等场景。',
  },
  {
    q: '和 certbot、acme.sh 相比有什么优势？',
    a: '本工具是中文图形化桌面应用，无需命令行与脚本配置：输入域名、选择验证方式即可完成申请，证书自动安装到本机并支持到期前自动续期，适合个人站长与运维新人；同时保持开源免费。',
  },
  {
    q: '和 TopSSL 是什么关系？',
    a: '本应用由 TopSSL（topssl.cn）出品并支持，免费工具部分完全开源。TopSSL 平台同时提供企业级 SSL 证书（DV/OV/EV）、自动化部署方案与专业技术支持服务。',
  },
  {
    q: '遇到问题如何获得支持？',
    a: '免费工具问题可查看应用内错误提示、用户手册，或在开源仓库提交 Issue；需要即时人工协助的企业用户，TopSSL 平台提供专业技术支持服务。',
  },
]
