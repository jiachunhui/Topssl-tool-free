// 站点统一配置：上线前只需修改这里
// ─────────────────────────────────────────────────────────────
// ★ 站点地址统一在此配置：SITE.url（下方）。它同时驱动：
//   canonical / og:url / sitemap.xml / robots.txt / JSON-LD
// 其余文件（astro.config.mjs、robots.txt 等）均自动引用此值。
// ─────────────────────────────────────────────────────────────
export const SITE = {
  name: 'Tossl 免费SSL证书管理工具',
  tagline: '免费 SSL 证书申请、部署与到期管理',
  description:
    'Tossl是一款免费的SSL证书管理工具：输入域名即可申请Let’s Encrypt证书，支持HTTP/DNS验证与通配符证书；一键导出部署包，Windows服务器可一键部署到IIS；到期前提醒并自动续期，支持Windows、macOS、Linux。',
  // 最终域名已确定：https://www.tossl.cn（sitemap / canonical / robots.txt / JSON-LD 均自动跟随此值）
  url: 'https://www.tossl.cn',
  lang: 'zh-CN',
  /** 站点 Logo（用于 Organization 结构化数据） */
  logo: '/apple-touch-icon.png',
  /** 品牌同源链接（sameAs） */
  sameAs: ['https://github.com/jiachunhui/Topssl-tool-free'],
  /** 当前版本（与 Cargo.toml 保持一致） */
  version: '0.1.8',
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

/**
 * 站内直链下载文件清单：文件放在 site/public/downloads/ 下，
 * 文件名与 GitHub Release 资产同名，版本号自动跟随 SITE.version。
 * 实际发布资产名与此不一致时，只需修改这里的文件名。
 * 下载页按钮链接 = SITE.url + '/downloads/' + 文件名。
 */
export const DOWNLOAD_FILES = {
  windows: `TopSSL-Free-Cert-Assistant_${SITE.version}_x64-setup.exe`,
  macosArm: `TopSSL-Free-Cert-Assistant_${SITE.version}_aarch64.dmg`,
  macosX64: `TopSSL-Free-Cert-Assistant_${SITE.version}_x64.dmg`,
  linuxDeb: `TopSSL-Free-Cert-Assistant_${SITE.version}_amd64.deb`,
  linuxAppImage: `TopSSL-Free-Cert-Assistant_${SITE.version}_amd64.AppImage`,
  linuxRpm: `TopSSL-Free-Cert-Assistant_${SITE.version}-1.x86_64.rpm`,
}

/** TopSSL 企业证书起售价（元/年），价格变动时只需修改此处 */
export const ENTERPRISE_PRICE = 45

/**
 * 首页促销价目：多年期 SSL 证书优惠价（每年单价）。
 * 价格/链接变动只需修改此处，版块内容自动跟随。
 */
export const PRICING = {
  badge: '限时优惠',
  heading: '多年期 SSL 证书，一次买多年更划算',
  sub: '通配符与单域名证书多年期特惠价，买得越久单价越低，新购、续期均享',
  note: '价格为每年单价 · 含税特惠价 · 活动解释权归 TopSSL 所有 · 企业批量采购可联系 TopSSL 商务',
  plans: [
    {
      name: '通配符 SSL 证书',
      desc: '一张证书覆盖主域名及全部一级子域名，适合子域名较多的业务',
      tag: '热门',
      highlight: true,
      priceValue: 487.6,
      priceTerm: '5 年',
      terms: [
        { term: '1 年', value: 540 },
        { term: '2 年', value: 537 },
        { term: '3 年', value: 523.33 },
        { term: '4 年', value: 505.5 },
        { term: '5 年', value: 487.6, hot: true },
      ],
      cta: '立即选购通配符证书',
      url: 'https://www.topssl.cn/ssl/xinssl-dv-wildcard?utm_source=topssl-cert-assistant-site&utm_medium=referral&utm_campaign=landing-pricing-wildcard',
    },
    {
      name: '单域名 SSL 证书',
      desc: 'DV 级别企业证书，适合单站点 HTTPS 加密与基础信任展示',
      tag: '经济',
      highlight: false,
      priceValue: 40.8,
      priceTerm: '5 年',
      terms: [
        { term: '1 年', value: 45 },
        { term: '2 年', value: 45 },
        { term: '3 年', value: 43.67 },
        { term: '4 年', value: 42.25 },
        { term: '5 年', value: 40.8, hot: true },
      ],
      cta: '立即选购单域名证书',
      url: 'https://www.topssl.cn/ssl/xinssl-dv-ssl?utm_source=topssl-cert-assistant-site&utm_medium=referral&utm_campaign=landing-pricing-single',
    },
  ],
}

export const FAQS = [
  {
    q:'Tossl申请的SSL证书安全吗？',
    a:'安全。Tossl申请的是Let’s Encrypt签发的标准SSL证书，符合现代浏览器HTTPS安全要求。',
  },
  {
    q:'免费SSL证书可以用于企业网站吗？',
    a:'可以用于基础HTTPS加密。但如果企业网站需要展示企业身份、提升客户信任，可以选择OV或EV SSL证书。',
  },
  {
    q:'SSL证书为什么需要自动续期？',
    a:'免费SSL证书有效期较短，自动续期可以避免因忘记更新导致网站出现HTTPS错误。',
  },
  {
    q:'Tossl支持哪些系统？',
    a:'支持Windows、macOS和Linux系统。',
  },
  {
    q: '证书是免费的吗？申请需要付费吗？',
    a: '完全免费。本工具调用 Let’s Encrypt 免费证书签发服务，输入域名即可申请，到期前自动续期，整个过程不收取任何费用。',
  },
  {
    q: '证书有效期多久？会自动续期吗？',
    a: 'Let’s Encrypt 证书有效期为 90 天。开启自动续期并保持应用运行（建议同时开启开机自启）时，应用会在到期前 30 天自动续期；即使错过续期窗口，应用每次启动也会检测到期风险并提醒您一键续期。',
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
    q: '个人电脑上申请的证书，如何部署到服务器？',
    a: 'Tossl 支持一键导出「部署包」：包含证书文件与 nginx / Apache / IIS 配置示例，把部署包拷贝到服务器按说明配置即可完成部署；Windows 服务器上还可直接使用 IIS 一键部署功能。',
  },
  {
    q: '自动续期需要一直打开应用吗？',
    a: '是。自动续期依赖应用在后台运行，建议在设置中开启「开机自启」；个人电脑如果无法保证常开，请保持到期提醒开启——应用每次启动都会检查并提醒即将到期或已过期的证书，可一键续期，避免证书静默过期。',
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
