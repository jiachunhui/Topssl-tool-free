import { defineCollection, z } from 'astro:content'
import { glob } from 'astro/loaders'

// ─────────────────────────────────────────────────────────────
// 内容集合：博客知识库 / 更新日志
// 发文章 = 在 src/content/blog/ 下新增一个 .md 文件，
// 列表页、详情页、首页「最新动态」、sitemap 全部自动更新。
// ─────────────────────────────────────────────────────────────

const blog = defineCollection({
  loader: glob({ pattern: '**/*.md', base: './src/content/blog' }),
  schema: z.object({
    title: z.string(),
    description: z.string(),
    pubDate: z.coerce.date(),
    updatedDate: z.coerce.date().optional(),
    category: z.enum(['免费SSL证书', 'SSL自动续期', 'DNS验证', '技术动态']),
    tags: z.array(z.string()).default([]),
    /** 相关文章 slug（不含 .md），详情页底部自动渲染 */
    related: z.array(z.string()).default([]),
    featured: z.boolean().default(false),
  }),
})

const changelog = defineCollection({
  loader: glob({
    pattern: '**/*.md',
    base: './src/content/changelog',
    // 默认 generateId 会做 githubSlug 转换（v0.1.7 → v017），
    // 这里保留原始文件名作为 id，保证 URL 为 /changelog/v0.1.7/
    generateId: ({ entry }) => entry.replace(/\.md$/, ''),
  }),
  schema: z.object({
    version: z.string(),
    date: z.coerce.date(),
    /** 完整标题，如「ToSSL v0.1.7 发布：修复自动更新错误码与下载超时问题」 */
    title: z.string(),
    /** 列表页摘要 */
    description: z.string().optional(),
  }),
})

export const collections = { blog, changelog }
