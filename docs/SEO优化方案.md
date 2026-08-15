# TopSSL 免费证书助手 · 官网落地页 SEO 优化方案

> 编制对象：`site/` 目录（Astro SSG 静态站点）
> 方案定位：上线前技术修正（P0）→ 上线后增长（P1）→ 持续运营（P2/P3）
> 评估日期：2026-01

---

## 实施状态（2026-01 更新）

**方案中的技术项已全部落地到 `site/` 代码**，构建验证通过（`npm run build`，8 页面）：

| 项 | 状态 | 说明 |
|---|---|---|
| P0-1 域名配置点 | ✅ 已集中 | `src/config.ts` 的 `SITE.url` 为唯一配置点，astro.config / robots.txt / canonical / sitemap / JSON-LD 全部自动跟随；**上线前仅需改这一处** |
| P0-2 自定义 404 | ✅ 已实现 | `src/pages/404.astro`，noindex 防软 404 |
| P0-3 真实截图 | ⚠️ 部分 | 3 张 PNG 已由 SVG 栅格化生成（1280×800，`scripts/rasterize-screenshots.mjs`）；**建议上线前替换为真实界面截图**（同名 PNG 覆盖即可） |
| P1-1 内容站扩展 | ✅ 已实现 | 新增 download / changelog / about / 3 篇 docs 教程，共 7 个可索引 URL |
| P1-2 Title/Description | ✅ 已优化 | 核心词前移去空格，每页独立 title/description |
| P1-3 站长平台 | ⚠️ 待人工 | `SITE.baiduSiteVerification` / `SITE.analytics.baiduTongjiId` 槽位已就绪，填入即生效；GSC/Bing 需在对应平台操作 |
| P1-4 外链 referrer | ✅ 已调整 | TopSSL 跳转 rel="noopener"（保留 referrer），GitHub 保持 noopener noreferrer |
| P2-1 keywords | ✅ 已更新 | 对齐关键词矩阵 |
| P2-2 结构化数据 | ✅ 已增强 | Organization + WebSite + SoftwareApplication（downloadUrl/releaseNotes/screenshot）+ FAQPage |
| P2-3 preconnect | ✅ 已添加 | github.com + topssl.cn |
| 其他 | ✅ | canonical 每页指向自身（原默认指向首页的缺陷已修复）、og:image 尺寸标注、hero fetchpriority=high、内链闭环 |

**剩余人工事项**：① 确定最终域名并修改 `SITE.url`；② 替换真实截图；③ 站长平台注册验证；④ 持续内容运营（P2/P3）。

---

## 一、现状审计结论

### 1.1 已具备的良好基础（应保留）

| 项目 | 现状 | 评价 |
|---|---|---|
| 技术形态 | Astro SSG 静态输出，首屏即 HTML | ✅ 对爬虫友好，优于 CSR SPA |
| 语义化 | `lang=zh-CN`、唯一 H1、语义化 section/h2/h3 | ✅ 结构清晰 |
| 基础 Meta | title / description / canonical / robots / keywords | ✅ 已齐备 |
| 社交分享 | Open Graph + Twitter Card + theme-color + 1200×630 og-image | ✅ og-image 尺寸规范 |
| 结构化数据 | SoftwareApplication + FAQPage JSON-LD，FAQ 内容页内可见 | ✅ 合规且有价值 |
| 技术 SEO | sitemap.xml + robots.txt、响应式、`prefers-reduced-motion` | ✅ 基础扎实 |
| 图片 | Hero 截图有 width/height + 描述性 alt，LCP 图 eager 加载 | ✅ 减少 CLS |

**结论：该站点技术 SEO 底子属于中上水平，主要问题集中在"上线前必改项未落实"和"内容资产单一"，而非代码层面的大缺陷。**

### 1.2 核心问题清单（按优先级）

| 级别 | 问题 | 位置 | 影响 |
|---|---|---|---|
| **P0-1** | **占位域名未替换**：`topssl-cert-assistant.example.com` 出现在 canonical、sitemap、og:url、JSON-LD url、robots.txt 的 Sitemap 地址中 | `src/config.ts:7`、`astro.config.mjs:10`、`public/robots.txt:4`、`BaseLayout.astro`（引用 SITE.url） | 搜索引擎会为不存在的域名建立索引，全部收录信号指向错误地址，上线后必须 301 迁移，**损失最大** |
| **P0-2** | 无自定义 404 页面 | 缺 `src/pages/404.astro` | 失效链接返回软 404（200），浪费抓取配额、稀释索引质量 |
| **P0-3** | Hero 截图是 SVG 占位图（`certs-view.svg`），非真实产品截图 | `Hero.astro:7-10` | 内容可信度低（E-E-A-T），LCP 资源非真实视觉 |
| **P1-1** | 全站仅 1 个页面，内容资产单薄 | `src/pages/` 仅 index.astro | 长尾关键词无承载页面，站点"信息增益"不足，排名天花板低 |
| **P1-2** | Title 可再优化：长度约 33 字符，且核心词带空格与真实搜索习惯不一致（"免费 SSL 证书" vs "免费SSL证书"） | `index.astro:13` | 百度展示约 30 汉字即截断，核心词前移 + 去空格可提升 CTR 与相关性 |
| **P1-3** | 无站点验证与统计：未接入百度站长 / GSC / Bing Webmaster，无任何埋点 | 全站 | 无法提交、无法观测，优化无数据支撑 |
| **P1-4** | 外链统一 `rel="noopener noreferrer"`，丢失 referrer 信息 | `SiteNav.astro:25` 等 | UTM 不受影响，但 referrer 数据对分析 TopSSL 跳转转化有参考价值 |
| **P2-1** | keywords meta 已不被百度/Google 用于排名（保留无害，建议更新为真实词表或删除） | `BaseLayout.astro:57` | 低优先级，清理即可 |
| **P2-2** | 缺少 Organization / WebSite 结构化数据，品牌实体未建立 | `BaseLayout.astro:76` | 影响品牌词 SERP 富展示（Sitelinks 依赖站点结构） |
| **P2-3** | 无 preconnect 到 GitHub（下载跳转目标）与 topssl.cn | 全站 | 微优化 |

---

## 二、目标与关键词策略

### 2.1 SEO 目标（建议 SMART）

1. **3 个月内**：核心词"免费SSL证书""SSL证书免费申请"进入百度前 5 页（竞争激烈，务实目标），品牌词"TopSSL免费证书助手"占 SERP 首屏。
2. **6 个月内**：长尾词（见 2.3）合计带来 ≥5,000 次/月搜索曝光，官网自然流量 ≥2,000 UV/月。
3. **转化目标**：GitHub 下载点击（下载页埋点）自然流量转化率 ≥8%。

### 2.2 竞争格局与定位

- 直接对手：Let's Encrypt 官方、certbot、acme.sh、宝塔面板、阿里云/腾讯云免费证书入口。
- **差异化定位（写进所有文案）**：中文原生 + 桌面 GUI（非命令行）+ 自动续期开箱即用 + 全平台 + 开源。这是 certbot/acme.sh 的痛点（命令行门槛），也是云厂商的痛点（绑定平台）。
- 页面所有标题与描述应反复强化这一差异化。

### 2.3 关键词矩阵（按意图分层）

| 层级 | 关键词 | 承载页面（建议） |
|---|---|---|
| 核心词 | 免费SSL证书、SSL证书免费申请、免费申请SSL证书、Let's Encrypt证书 | 首页 |
| 品牌词 | TopSSL、TopSSL免费证书助手、topssl 证书 | 首页 + 关于页 |
| 长尾-工具 | SSL证书自动续期、免费证书自动续期工具、Windows SSL证书工具、macOS免费SSL证书、全平台证书管理工具 | 下载页 / 教程页 |
| 长尾-场景 | 通配符证书免费申请、多域名证书申请、DNS验证申请SSL证书、HTTP验证SSL证书、80端口不可用申请证书 | 教程页（每场景 1 篇） |
| 长尾-对比 | 免费SSL证书和付费区别、certbot 替代工具、acme.sh 图形化替代 | 对比/博客页 |
| 信息词 | 证书到期怎么办、SSL证书有效期多久、HTTPS证书怎么申请 | FAQ 扩展 + 教程页 |

> 选词原则：避开"SSL证书购买/价格"（商业意图已被大厂霸占且与免费定位冲突），主攻"免费申请/自动续期/工具"类高转化长尾。

---

## 三、P0 上线前必改（阻塞项，按顺序执行）

### 3.1 确定最终域名并全局替换（最重要）

三处占位域名必须一致：

| 文件 | 行 | 现值 | 改为 |
|---|---|---|---|
| `src/config.ts` | 7 | `https://topssl-cert-assistant.example.com` | 最终域名（如 `https://www.topssl.cn/ssl-cert-assistant` 或独立域名） |
| `astro.config.mjs` | 10 | `site: 'https://topssl-cert-assistant.example.com'` | 与上一致 |
| `public/robots.txt` | 4 | Sitemap 地址 | 与上一致（Astro 默认生成 `sitemap-index.xml`，路径写法正确） |

**决策建议（二选一，影响后续所有策略）：**

- **方案 A：部署为 topssl.cn 子路径**（`https://www.topssl.cn/ssl-cert-assistant`）——继承主站权重，品牌词共享，无需新站冷启动；代价是 URL 变长，且需主站 robots/canonical 配合。
- **方案 B：独立域名**（如 `https://topssl-cert.cn`）——定位独立产品线，利于后期做独立内容站；代价是 0 权重冷启动，需更长周期。

> ⚠️ 上线前必须做：域名确定后，先在本地 `npm run build` 验证生成的 `dist/sitemap-index.xml`、`dist/robots.txt`、页面 canonical 全部指向新域名，再部署。

### 3.2 自定义 404 页面

新建 `site/src/pages/404.astro`，复用 BaseLayout：

```astro
---
import BaseLayout from '../layouts/BaseLayout.astro'
---
<BaseLayout title="页面未找到 - TopSSL 免费证书助手" description="您访问的页面不存在，返回首页继续浏览。">
  <section class="section" style="text-align:center;padding-block:6rem">
    <h1>404 · 页面未找到</h1>
    <p>您访问的地址不存在或已移动。</p>
    <a class="btn btn-primary" href="/">返回首页</a>
  </section>
</BaseLayout>
```

- Astro SSG 会输出 `dist/404.html`，主流静态托管（Nginx/GitHub Pages/Cloudflare Pages）自动识别。
- 效果：失效 URL 返回真 404，避免软 404 消耗抓取配额。

### 3.3 真实产品截图替换占位 SVG

1. 将三张真实截图命名为 `certs-view.png`、`dns-view.png`、`wizard-view.png` 放入 `site/public/screenshots/`（`Hero.astro:9` 已自动优先读取 PNG，无需改代码）。
2. 截图规范：
   - 尺寸 ≥1280 宽，导出 PNG 或 WebP（建议同时提供 `.webp` 并保留 PNG 兜底）；
   - 窗口内容清晰、无隐私信息、中文界面；
   - alt 文案保持描述性（当前写法合规）。
3. 后续可加 `fetchpriority="high"` 到 Hero 主图（LCP 优化）。

---

## 四、P1 上线后首月执行（增长项）

### 4.1 站点结构扩展：从单页变"1 + N"内容站

**建议新增页面（全部复用 BaseLayout，自动继承 canonical/sitemap/结构化数据）：**

| 页面 | 路由 | 核心词 | 内容要点 |
|---|---|---|---|
| 下载页 | `/download` | Windows SSL证书工具、免费下载 | 平台安装包、版本、校验和、GitHub Release 直达、更新日志摘要 |
| 使用教程 | `/docs/quickstart` | SSL证书申请教程、免费证书怎么申请 | 图文步骤：输入域名→选择验证→安装配置（截图复用） |
| 验证方式 | `/docs/dns-validation` | DNS验证SSL证书、通配符证书 | DNS-01 详解 + 支持的服务商 API 列表 + 通配符场景 |
| 自动续期 | `/docs/auto-renewal` | SSL证书自动续期、证书到期续期 | 续期机制、开机自启/托盘说明、常见问题 |
| 更新日志 | `/changelog` | TopSSL免费证书助手更新 | 版本时间线（对"软件下载类"查询有强相关） |
| 关于/开源 | `/about` | TopSSL开源、MIT开源证书工具 | 项目背景、许可、安全说明、联系方式（E-E-A-T） |

**每新增一页，更新一次 sitemap 会自动完成；页面间用页脚/正文互链形成内链环。**

### 4.2 Title / Description 优化

**首页建议值（写入 `index.astro:13-15`）：**

```text
Title:       免费SSL证书一键申请、自动续期工具 - TopSSL
Description: TopSSL免费证书助手：免费申请Let's Encrypt SSL证书并自动续期，支持HTTP/DNS验证、通配符与多域名证书，Windows/macOS/Linux全平台，开源免费。
```

- 核心词"免费SSL证书"置于最前、**无空格**（贴合真实搜索输入习惯）；
- Title 控制在 28 汉字内（百度展示约 30 字）；
- Description 保持 ≤120 汉字，含核心词 + 差异化卖点 + 行动引导。

**其余页面按同样模板生成：`{页面核心词} - TopSSL免费证书助手`。**

### 4.3 站长平台接入（第 1 周内完成）

| 平台 | 动作 | 目的 |
|---|---|---|
| 百度搜索资源平台 | 添加站点 + 验证（文件验证或 meta）+ 提交 sitemap + 手动/API 推送首页 | 中文流量主阵地，加速收录 |
| Google Search Console | 添加资源 + 提交 sitemap + 请求索引 | 国际流量 + Core Web Vitals 监控 |
| Bing Webmaster | 可从 GSC 一键导入 | 覆盖 Bing/Copilot/ChatGPT 检索来源 |
| 51LA / Umami / 百度统计 | 页脚或 head 加统计脚本 | 埋点转化（下载点击建议用 `data-*` 事件 + 下载页独立 URL 双保险） |

> 埋点建议：下载按钮改为指向站内 `/download` 页（而非直接跳 GitHub），可统计"到站→点击下载"漏斗，同时让下载页承载"软件下载类"关键词。

### 4.4 结构化数据增强

在 `BaseLayout.astro` 的 head 中追加（需在 `src/config.ts` 增加 `SITE.logo`、`SITE.sameAs` 等常量）：

```json
{
  "@context": "https://schema.org",
  "@type": "Organization",
  "name": "TopSSL",
  "url": "https://www.topssl.cn",
  "logo": "https://<最终域名>/apple-touch-icon.png",
  "sameAs": ["https://github.com/jiachunhui/Topssl-tool-free"]
}
```

- SoftwareApplication 补充字段：`downloadUrl`（指向 GitHub Release）、`screenshot`（指向真实截图）、`releaseNotes`（更新日志页）。
- FAQPage 保持现状（页内可见即合规；百度仍展示 FAQ 富摘要，Google 自 2023-08 起仅限权威站点展示，不影响索引价值）。

### 4.5 外链与 referrer 微调

- TopSSL 跳转链接建议 `rel="noopener"`（保留 referrer，便于主站识别来源路径），GitHub 链接保持 `noopener noreferrer`。
- 页脚"相关资源"增加指向 topssl.cn 的 1-2 个正文级链接（锚文本用"企业SSL证书""OV/EV证书"），形成品牌内链闭环。

---

## 五、P2 持续优化（第 2-3 个月）

1. **内容营销（核心增长引擎）**：在 `/docs` 下按"场景关键词"持续产出教程（每周 1-2 篇）：如《80端口被占用如何申请SSL证书》《Nginx部署免费证书教程》《多域名证书与通配符证书怎么选》。每篇含 1 个核心词 + 2-3 个长尾词，页内互链 + 返回首页 CTA。
2. **性能预算（Core Web Vitals）**：目标 LCP < 2.5s、CLS < 0.1、INP < 200ms。Astro SSG 已占优，重点：Hero 图换 WebP、CSS 体积监控（`global.css` 为单文件，建议按需拆包）、JS 保持零第三方库。
3. **内链与锚文本**：所有新增页面遵守"首页→栏目页→内容页"三级结构，锚文本含关键词。
4. **品牌词监控**：百度/Google 定期搜索"TopSSL 免费证书助手"，确保首屏全部为自有资产（官网、GitHub、README）。
5. **外链建设**：GitHub 仓库 README 顶部加官网链接；在知乎"如何免费申请SSL证书"类问题下以产品身份作答；V2EX/掘金发布教程（每篇外链官网）；申请收录到免费工具导航站。
6. **清理**：删除或更新 keywords meta 词表（对齐 2.3 关键词矩阵）。

---

## 六、P3 长期机制

| 机制 | 做法 |
|---|---|
| 月度 SEO 巡检 | 检查：sitemap 覆盖、404 率、canonical 一致性、页面速度（PageSpeed Insights）、索引量 |
| 关键词复盘 | 每季度按搜索量/排名/转化重排关键词矩阵，淘汰无转化词 |
| 内容日历 | 跟随 Let's Encrypt 政策变化（如证书有效期调整）产出解读文章，抢占时效流量 |
| E-E-A-T 强化 | 站点底部长期展示：开源协议、版本号、维护者信息、Issue 入口、版权与免责声明（已有基础，保持） |
| 数据看板 | 百度统计/GSC/Bing 周报合并为一张看板：曝光→点击→下载 转化漏斗 |

---

## 七、执行路线图（汇总）

| 阶段 | 周期 | 动作 | 负责人建议 |
|---|---|---|---|
| P0 上线前 | 1 周 | 定域名、替换 3 处占位、404 页、真实截图 | 开发 |
| P1 首月 | 第 1 周 | 站长平台接入、统计埋点、Title/Description 更新 | 开发 + 运营 |
| P1 首月 | 第 2-4 周 | 6 个新页面上线、结构化数据增强、内链闭环 | 开发 + 内容 |
| P2 | 第 2-3 月 | 教程内容日历、性能优化、外链建设 | 内容 + 运营 |
| P3 | 持续 | 月度巡检 + 季度复盘 | 运营 |

---

## 八、衡量指标（KPI）

| 指标 | 工具 | 基线→目标（6 个月） |
|---|---|---|
| 收录页面数 | 百度资源平台 / GSC | 1 → 10+ |
| 品牌词 SERP 首屏占比 | 手动 / 工具 | 0 → 100% |
| 自然搜索 UV/月 | 百度统计 / GA | 0 → 2,000+ |
| 下载转化率（自然流量） | 下载页埋点 | — → ≥8% |
| LCP / CLS / INP | PageSpeed Insights | 达标（绿区） |
| 核心词排名 | 百度指数 + 工具 | 进前 5 页 → 前 3 页 |

---

*本方案所有改动均可基于现有 Astro 工程直接落地，无技术债；上线前唯一硬性前置条件是**确定最终域名**。*
