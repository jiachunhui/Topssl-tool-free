// 统一宣传常量：TopSSL 链接 / UTM / GitHub / 企业证书价格
// 集中管理，后期价格或链接变动只需改这里
import { api } from './api'
import { isTauri } from './mock'

export const TOPSSL_HOME = 'https://www.topssl.cn'
export const GITHUB_REPO = 'https://github.com/jiachunhui/Topssl-tool-free'

/** UTM 来源：与产品 identifier 一致（com.topsl.ssl-cert-desktop），长期稳定 */
const UTM_SOURCE = 'topssl-cert-desktop'

/** 生成带 UTM 统计参数的 TopSSL 链接；path 传入子路径（如 '/ssl/one'）时跳转到对应产品页 */
export function topsslUrl(medium: string, campaign: string, path = ''): string {
  const base = path ? `${TOPSSL_HOME}${path.startsWith('/') ? path : '/' + path}` : TOPSSL_HOME
  return `${base}/?utm_source=${UTM_SOURCE}&utm_medium=${encodeURIComponent(medium)}&utm_campaign=${encodeURIComponent(campaign)}`
}

/** TopSSL 企业证书起售价（元/年）——价格变动时只需修改此处 */
export const ENTERPRISE_PRICE = 45

/** 打开外部链接：桌面端用系统默认浏览器，浏览器（mock）环境回退到新窗口 */
export async function openExternal(url: string): Promise<void> {
  if (isTauri()) {
    await api.openUrl(url)
  } else {
    window.open(url, '_blank', 'noopener,noreferrer')
  }
}
