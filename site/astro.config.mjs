import { defineConfig } from 'astro/config'
import sitemap from '@astrojs/sitemap'

// ─────────────────────────────────────────────────────────────
// 部署方式待定：上线前请把 site 改为最终部署域名/路径，例如：
//   site: 'https://www.topssl.cn/ssl-cert-assistant'
// sitemap.xml 与 canonical 会基于该值自动生成。
// ─────────────────────────────────────────────────────────────
export default defineConfig({
  site: 'https://topssl-cert-assistant.example.com',
  output: 'static',
  integrations: [sitemap()],
})
