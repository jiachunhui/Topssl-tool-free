# TopSSL 免费证书助手 · 宣传落地页

基于 **Astro（SSG）** 的静态宣传页，主色为 TopSSL 品牌色 `#0085a1`，页面结构与氛围参考 DeepSeek Harness。
独立于桌面端构建，部署方式待定（可部署到 topssl.cn 子路径 / GitHub Pages / 任意静态托管）。

## 开发与构建

```bash
npm install
npm run dev       # 本地开发预览 http://localhost:4321
npm run build     # 输出静态产物到 dist/
npm run preview   # 本地预览构建产物
```

## 上线前必改

1. **`src/config.ts`（唯一配置点）**：
   - `SITE.url`：改为最终部署域名（sitemap、canonical、robots.txt、JSON-LD、og:url 全部自动跟随此值，无需改其他文件）
   - `SITE.baiduSiteVerification`：百度搜索资源平台验证码（可选）
   - `SITE.analytics.baiduTongjiId`：百度统计 ID（可选）
   - `TOPSSL.url(campaign)`：UTM 参数（utm_source/medium 可自定义）
   - `ENTERPRISE_PRICE`：企业证书起售价，价格变动时修改
2. **品牌资源**：`public/favicon.ico`、`public/apple-touch-icon.png`、`public/og-image.png`
   由 `scripts/generate-assets.ps1` 生成（PowerShell System.Drawing，无外部依赖），可重新生成或替换为官方高清素材。
3. **产品截图**：`public/screenshots/*.png` 目前由 SVG 占位图栅格化生成（`scripts/rasterize-screenshots.mjs`，sharp 渲染 1280×800）。
   上线前请用真实界面截图覆盖（保持同名 PNG 即可，代码自动优先使用 PNG）。

## SEO 已内置

- 语义化 HTML + 每页唯一 h1 + lang=zh-CN
- title / description / canonical（每页指向自身）/ robots / keywords
- Open Graph（含 og:image 1200×630 尺寸标注）+ Twitter Card + theme-color
- JSON-LD：SoftwareApplication（downloadUrl/releaseNotes/screenshot）+ FAQPage + Organization + WebSite
- sitemap.xml（@astrojs/sitemap 自动生成，含全部子页面）+ 动态 robots.txt（`src/pages/robots.txt.ts`）
- 百度站长验证与百度统计槽位（config 中填写即启用）
- 内容页内链闭环：下载页 / 教程页 / 更新日志 / 关于 互相串联
- 响应式 + 移动端适配 + prefers-reduced-motion
- 自定义 404 页（noindex，避免软 404）

## 目录结构

```
site/
  astro.config.mjs     # 站点配置（site 地址自动取自 config.ts）
  src/
    config.ts          # ★ 唯一配置点：域名/名称/UTM/价格/FAQ/验证与统计槽位
    layouts/           # BaseLayout（SEO head + 导航 + 页脚）
    components/        # Hero / 特性 / 对比 / TopSSL 专区 / FAQ 等
    pages/
      index.astro      # 首页落地页
      download.astro   # 下载页
      changelog.astro  # 更新日志
      about.astro      # 关于与开源
      docs/            # 使用教程（quickstart / dns-validation / auto-renewal）
      robots.txt.ts    # 动态生成 robots.txt
      404.astro        # 自定义 404（noindex）
    styles/global.css  # 设计系统（品牌色板）
  public/              # 品牌图片资源 + 产品截图
  scripts/             # 资源生成脚本（PS 图标 + sharp 截图栅格化）
```
