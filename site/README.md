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

1. **`src/config.ts`**：
   - `SITE.url`：改为最终部署域名（sitemap 与 canonical 基于它生成）
   - `TOPSSL.url(campaign)`：UTM 参数（utm_source/medium 可自定义）
   - `ENTERPRISE_PRICE`：企业证书起售价，价格变动时修改
2. **`astro.config.mjs`**：`site` 改为最终域名
3. **`public/robots.txt`**：Sitemap 地址改为最终域名
4. **品牌资源**：`public/favicon.ico`、`public/apple-touch-icon.png`、`public/og-image.png`
   由 `scripts/generate-assets.ps1` 生成（PowerShell System.Drawing，无外部依赖），可重新生成或替换为官方高清素材。

## SEO 已内置

- 语义化 HTML + 唯一 h1 + lang=zh-CN
- title / description / keywords / canonical / robots
- Open Graph + Twitter Card + theme-color
- JSON-LD：SoftwareApplication + FAQPage
- sitemap.xml（@astrojs/sitemap 自动生成）+ robots.txt
- 响应式 + 移动端适配 + prefers-reduced-motion

## 目录结构

```
site/
  astro.config.mjs     # 站点配置（site 地址、sitemap）
  src/
    config.ts          # 站点常量：名称/链接/UTM/价格/FAQ
    layouts/           # BaseLayout（SEO head + 导航 + 页脚）
    components/        # Hero / 特性 / 对比 / TopSSL 专区 / FAQ 等
    pages/index.astro  # 单页落地页
    styles/global.css  # 设计系统（品牌色板）
  public/              # robots.txt + 品牌图片资源
  scripts/             # 资源生成脚本（PowerShell）
```
