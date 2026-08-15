// 站点统一配置：上线前只需修改这里
export const SITE = {
  name: 'TopSSL 免费证书助手',
  tagline: '免费 SSL 证书一键申请，到期自动续期',
  description:
    'TopSSL 免费证书助手是一款由 TopSSL 出品的开源跨平台桌面工具：输入域名即可免费申请 Let’s Encrypt SSL 证书，自动安装到本机并到期自动续期，支持 HTTP/DNS 验证与通配符证书。',
  url: 'https://topssl-cert-assistant.example.com', // TODO: 上线前替换为最终域名
  lang: 'zh-CN',
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
    q: '和 TopSSL 是什么关系？',
    a: '本应用由 TopSSL（topssl.cn）出品并支持，免费工具部分完全开源。TopSSL 平台同时提供企业级 SSL 证书（DV/OV/EV）、自动化部署方案与专业技术支持服务。',
  },
  {
    q: '遇到问题如何获得支持？',
    a: '免费工具问题可查看应用内错误提示、用户手册，或在开源仓库提交 Issue；需要即时人工协助的企业用户，TopSSL 平台提供专业技术支持服务。',
  },
]
