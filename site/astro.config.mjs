import { defineConfig } from 'astro/config'
import sitemap from '@astrojs/sitemap'
import { SITE } from './src/config'

// ─────────────────────────────────────────────────────────────
// 站点地址统一从 src/config.ts 的 SITE.url 读取（单一配置点）。
// 上线前只需修改 src/config.ts 一处，sitemap.xml / canonical
// / robots.txt 会全部自动跟随。
// ─────────────────────────────────────────────────────────────
export default defineConfig({
  site: SITE.url,
  output: 'static',
  integrations: [sitemap()],
  markdown: {
    // 代码高亮采用 GitHub 风格（浅色主题）
    shikiConfig: {
      theme: 'github-light',
    },
  },
})
