// 错误码 → 中文文案映射（与 Rust 端 error.rs 的 ErrorCode 枚举一致）
import type { ErrorInfo } from './types'

const ERROR_MAP: Record<string, Omit<ErrorInfo, 'code'>> = {
  ERR_INVALID_DOMAIN: {
    title: '域名格式不正确',
    message: '请输入完整的域名，例如 example.com 或 *.example.com',
    suggestion: '检查域名拼写，主域名不要包含 http:// 或路径',
    level: 'warn',
  },
  ERR_ACME_CONNECTION: {
    title: '无法连接 ACME 服务',
    message: '连接 Let\u2019s Encrypt 服务失败，可能是网络问题',
    suggestion: '检查网络连接后重试',
    level: 'error',
  },
  ERR_ACME_ACCOUNT: {
    title: '账户注册失败',
    message: '注册 Let\u2019s Encrypt 账户失败',
    suggestion: '检查邮箱格式，或更换邮箱后重试',
    level: 'error',
  },
  ERR_ACME_RATE_LIMIT: {
    title: '触发了 Let\u2019s Encrypt 速率限制',
    message: '同一域名每周最多签发 50 张正式证书',
    suggestion: '请先用测试环境（Staging）验证，或在官网查询配额',
    level: 'warn',
  },
  ERR_ORDER_CREATE: {
    title: '创建订单失败',
    message: '无法为域名创建证书订单',
    suggestion: '稍后重试，或检查是否触发速率限制',
    level: 'error',
  },
  ERR_CHALLENGE_UNSUPPORTED: {
    title: '该域名不支持此验证方式',
    message: '通配符域名只能使用 DNS 验证（DNS-01）',
    suggestion: '请切换到 DNS 验证并配置 DNS 服务商',
    level: 'warn',
  },
  ERR_HTTP01_PORT_BUSY: {
    title: '80 端口被占用',
    message: '需要监听 80 端口完成验证，但该端口已被其他程序占用',
    suggestion: '关闭占用 80 端口的程序，或改用 DNS 验证',
    level: 'warn',
  },
  ERR_HTTP01_UNREACHABLE: {
    title: '本机 80 端口公网不可达',
    message: 'Let\u2019s Encrypt 无法访问您的验证服务',
    suggestion: '确保域名解析到本机公网 IP 且路由器/防火墙放行 80 端口；或改用 DNS 验证',
    level: 'error',
  },
  ERR_HTTP01_PRIVILEGE: {
    title: '无权限监听 80 端口',
    message: '当前系统不允许普通用户监听 80 端口',
    suggestion: '请执行一次 setcap 授权命令（见提示），或改用 DNS 验证',
    level: 'warn',
  },
  ERR_DNS_PROVIDER_AUTH: {
    title: 'DNS 服务商认证失败',
    message: 'API 密钥无效或无权限',
    suggestion: '检查 API Key / Token 是否正确，以及域名是否在账户下',
    level: 'error',
  },
  ERR_DNS_PROVIDER_API: {
    title: 'DNS 服务商接口调用失败',
    message: '调用 DNS 服务商接口出错',
    suggestion: '查看日志详情，或稍后重试',
    level: 'error',
  },
  ERR_DNS_TXT_NOT_FOUND: {
    title: '未找到 DNS 解析区域',
    message: '在 DNS 服务商账户下找不到该域名的解析区域',
    suggestion: '确认域名已添加到该服务商并开启解析',
    level: 'warn',
  },
  ERR_DNS_PROPAGATION_TIMEOUT: {
    title: 'TXT 记录传播超时',
    message: '等待 DNS 记录全球生效超时',
    suggestion: '稍后重试；若长期如此请检查解析线路或记录值是否正确',
    level: 'warn',
  },
  ERR_VALIDATION_FAILED: {
    title: '域名所有权验证失败',
    message: 'Let\u2019s Encrypt 未能确认您对该域名的控制权',
    suggestion: '查看下方详细原因，检查验证配置后重试',
    level: 'error',
  },
  ERR_FINALIZE_FAILED: {
    title: '证书签发失败',
    message: 'Let\u2019s Encrypt 拒绝签发证书',
    suggestion: '查看返回的详细原因',
    level: 'error',
  },
  ERR_CERT_DOWNLOAD: {
    title: '证书下载失败',
    message: '无法从 Let\u2019s Encrypt 下载已签发的证书',
    suggestion: '稍后重试',
    level: 'error',
  },
  ERR_CERT_WRITE: {
    title: '证书保存失败',
    message: '无法将证书文件写入本机目录',
    suggestion: '检查应用数据目录的写入权限',
    level: 'error',
  },
  ERR_DUPLICATE_CERT: {
    title: '该域名已有有效证书',
    message: '距到期超过 30 天的有效证书不可重复申请',
    suggestion: '可以对现有证书执行续期',
    level: 'info',
  },
  ERR_CANCELED: {
    title: '已取消',
    message: '申请任务已被取消',
    suggestion: '',
    level: 'info',
  },
  ERR_COOL_DOWN: {
    title: '申请过于频繁',
    message: '同一域名刚刚申请失败，需要等待冷却时间',
    suggestion: '请 10 分钟后再试；手动 DNS 模式等待超时不受此限制',
    level: 'warn',
  },
  ERR_UPDATE_CHECK: {
    title: '检查更新失败',
    message: '无法连接更新服务器',
    suggestion: '请检查网络后重试；也可前往 GitHub Releases 页面手动下载',
    level: 'warn',
  },
  ERR_UPDATE_DOWNLOAD: {
    title: '下载更新失败',
    message: '安装包下载失败',
    suggestion: '请检查网络后重试，或前往 GitHub Releases 页面手动下载',
    level: 'error',
  },
  ERR_INVALID_SETTING: {
    title: '设置项无效',
    message: '提供的设置值不合法',
    suggestion: '请检查输入值（如端口范围 1-65535）',
    level: 'warn',
  },
  ERR_DEPLOY: {
    title: '部署失败',
    message: '证书部署过程中出现错误',
    suggestion: '按错误提示排查；如提示需要管理员权限，请以管理员身份运行 ToSSL',
    level: 'error',
  },
  ERR_DB: {
    title: '数据存储错误',
    message: '应用数据库读写异常',
    suggestion: '查看日志详情',
    level: 'error',
  },
  ERR_INTERNAL: {
    title: '内部错误',
    message: '发生了未预期的错误',
    suggestion: '请查看日志并反馈',
    level: 'error',
  },
}

export function getErrorInfo(code: string, detail?: string | null): ErrorInfo {
  const base = ERROR_MAP[code] ?? {
    title: '操作失败',
    message: '发生了错误',
    suggestion: '请查看日志详情',
    level: 'error' as const,
  }
  return { code, ...base, message: detail ? `${base.message}（${detail}）` : base.message }
}
