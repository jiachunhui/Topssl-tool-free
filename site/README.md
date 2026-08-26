# Tossl 免费SSL证书管理工具 · 宣传站点

基于 **Astro（SSG）** 的静态宣传页，主色为 TopSSL 品牌色 `#0085a1`，页面结构与氛围参考 DeepSeek Harness。
独立于桌面端构建，部署于 https://www.tossl.cn。

## 开发与构建

```bash
npm install
npm run dev       # 本地开发预览 http://localhost:4321
npm run build     # 输出静态产物到 dist/
npm run preview   # 本地预览构建产物
```

## 上线前检查

1. **`src/config.ts`（唯一配置点）**：
   - `SITE.url`：已确定为 https://www.tossl.cn（sitemap、canonical、robots.txt、JSON-LD、og:url 全部自动跟随此值）
   - `DOWNLOAD_FILES`：站内直链下载文件名（与 GitHub Release 资产同名，版本号自动跟随 `SITE.version`；实际资产名不一致时只需改这一处）
   - `SITE.baiduSiteVerification`：百度搜索资源平台验证码（可选）
   - `SITE.analytics.baiduTongjiId`：百度统计 ID（可选）
   - `TOPSSL.url(campaign)`：UTM 参数（utm_source/medium 可自定义）
   - `ENTERPRISE_PRICE`：企业证书起售价，价格变动时修改
2. **站内下载文件**：把安装包放入 `public/downloads/`（文件名与 GitHub Release 资产同名，见 `src/config.ts` 的 `DOWNLOAD_FILES`）。
   构建时自动复制到 dist，下载页按钮指向 `https://www.tossl.cn/downloads/<文件名>`，GitHub 下载入口保留作为备用。
2. **品牌资源**：`public/favicon.ico`、`public/apple-touch-icon.png`、`public/og-image.png`
   由 `scripts/generate-assets.ps1` 生成（PowerShell System.Drawing，无外部依赖），可重新生成或替换为官方高清素材。
3. **产品截图**：`public/screenshots/*.png` 目前由 SVG 占位图栅格化生成（`scripts/rasterize-screenshots.mjs`，sharp 渲染 1280×800）。
   上线前请用真实界面截图覆盖（保持同名 PNG 即可，代码自动优先使用 PNG）。

## 更新源部署（应用内「检查更新」）

应用内更新机制：**国内清单 `https://www.tossl.cn/updates/latest.json` 优先，请求失败自动回退 GitHub Releases**。

每次发布版本（`git tag vX.Y.Z` 触发 GitHub Release）后，CI 会额外生成 **`site-publish`** artifact，包含：

- `updates/latest.json` —— 国内更新清单（版本号 / 更新说明 / 各平台安装包地址 + sha256）
- `downloads/` —— 本次版本的安装包（exe / dmg / AppImage / deb / rpm）

部署 www.tossl.cn 时：

1. `npm run build` 生成 `site/dist`，上传到站点根目录；
2. 从 Release 的 `site-publish` artifact 下载，把 `updates/latest.json` 上传到站点 **`/updates/`** 目录；
3. 把 `downloads/` 下的安装包上传到站点 **`/downloads/`** 目录（与下载页站内直链同目录，文件名见 `src/config.ts` 的 `DOWNLOAD_FILES`）。

最终站点目录结构：

```
/                    （site/dist 内容）
/updates/latest.json
/downloads/TopSSL-Free-Cert-Assistant_<版本>_x64-setup.exe
/downloads/...（其他平台安装包）
```

> 提示：清单与安装包不提交 git（`updates/` 已被根 `.gitignore` 忽略），始终以 CI 生成的 `site-publish` artifact 为准；本地也可用 `node scripts/gen-update-manifest.mjs` 手动生成。

## SEO 已内置

- 语义化 HTML + 每页唯一 h1 + lang=zh-CN
- title / description / canonical（每页指向自身）/ robots / keywords
- Open Graph（含 og:image 1200×630 尺寸标注）+ Twitter Card + theme-color
- JSON-LD：SoftwareApplication（downloadUrl/releaseNotes/screenshot）+ FAQPage + Organization + WebSite + Blog/BlogPosting + TechArticle + BreadcrumbList
- sitemap.xml（@astrojs/sitemap 自动生成，含全部子页面与文章）+ 动态 robots.txt（`src/pages/robots.txt.ts`）
- 百度站长验证与百度统计槽位（config 中填写即启用）
- 内容页内链闭环：首页 / 下载页 / 文档中心 / 知识库 / 更新日志 互相串联，文章正文互链
- 首页「最新动态」模块自动拉取最新版本与文章，发内容即更新首页
- 响应式 + 移动端适配 + prefers-reduced-motion
- 自定义 404 页（noindex，避免软 404）

## 内容维护（更新日志 / 博客）

内容由 **Astro Content Collections** 管理（`src/content.config.ts`），发内容 = 新增一个 markdown 文件，无需改代码：

- **更新日志**：`src/content/changelog/vX.Y.Z.md`，frontmatter 含 `version / date / title / description`，正文为版本说明。新增后列表页、单版本页、首页最新动态、sitemap 全部自动更新。
- **博客文章**：`src/content/blog/<slug>.md`，frontmatter 含 `title / description / pubDate / category（免费SSL证书｜SSL自动续期｜DNS验证｜技术动态）/ tags / related（相关文章 slug 列表）`。
- 发布节奏建议：每次发版写一条更新日志（月 1–2 次）；博客文章每月 4–8 篇，围绕「免费SSL证书 / Let's Encrypt / SSL自动续期 / DNS验证」主题。

## 目录结构

```
site/
  astro.config.mjs     # 站点配置（site 地址自动取自 config.ts）
  src/
    config.ts          # ★ 唯一配置点：域名/名称/UTM/价格/FAQ/验证与统计槽位
    content.config.ts  # ★ 内容集合 schema（blog / changelog）
    content/
      blog/            # 知识库文章（markdown）
      changelog/       # 版本更新日志（markdown）
    layouts/           # BaseLayout（SEO head + 导航 + 页脚）
    components/        # Hero / 特性 / 对比 / TopSSL 专区 / FAQ / 最新动态 等
    pages/
      index.astro      # 首页落地页（含最新动态模块）
      download.astro   # 下载页
      changelog/       # 更新日志列表 + 单版本详情页
      blog/            # 知识库索引 + 文章详情页
      docs/            # 文档中心索引 + install/ + troubleshooting/ + 使用教程
      about.astro      # 关于与开源
      robots.txt.ts    # 动态生成 robots.txt
      404.astro        # 自定义 404（noindex）
    styles/global.css  # 设计系统（品牌色板）
  public/              # 品牌图片资源 + 产品截图 + downloads/ 站内下载文件
  scripts/             # 资源生成脚本（PS 图标 + sharp 截图栅格化）
```
