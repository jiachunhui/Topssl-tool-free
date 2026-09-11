#!/usr/bin/env node
// 从站点更新日志抽取「更新说明」文本，供应用内更新弹窗与 updates/latest.json 使用
//
// 为什么需要这个脚本：
//   应用内「发现新版本」弹窗是**纯文本**渲染（whitespace-pre-wrap），
//   直接把 Markdown 塞进去，用户会看到 ** 加粗标记、- 列表符号、[文字](链接) 语法，
//   所以这里把站点更新日志转换成干净的纯文本。
//
// 内容源复用站点的更新日志（site/src/content/changelog/vX.Y.Z.md），
// 避免再单独维护一份 CHANGELOG。
//
// 用法：
//   node scripts/extract-changelog-notes.mjs                     # 版本取 package.json，输出 updates/release-notes.md
//   node scripts/extract-changelog-notes.mjs 0.1.11              # 指定版本（v 前缀可省）
//   node scripts/extract-changelog-notes.mjs 0.1.11 /tmp/n.md    # 指定输出路径
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..')

/** 去掉 Markdown 标记，得到适合弹窗展示的纯文本 */
function toPlainText(md) {
  return md
    // 图片 → alt 文本
    .replace(/!\[([^\]]*)\]\([^)]*\)/g, '$1')
    // 链接 → 链接文字（弹窗里带 URL 太吵）
    .replace(/\[([^\]]+)\]\([^)]*\)/g, '$1')
    // 加粗 / 斜体 / 行内代码
    .replace(/\*\*([^*]+)\*\*/g, '$1')
    .replace(/(^|[^*])\*([^*\n]+)\*/g, '$1$2')
    .replace(/`([^`]+)`/g, '$1')
    // 标题符号
    .replace(/^#{1,6}\s+/gm, '')
    // 引用
    .replace(/^>\s?/gm, '')
    // 无序列表项 → 圆点
    .replace(/^[-*+]\s+/gm, '• ')
    // 有序列表保持 "1. " 原样，去掉多余缩进
    .replace(/^[ \t]+(?=\S)/gm, '')
    // 压缩连续空行
    .replace(/\n{3,}/g, '\n\n')
    .trim()
}

/** 取站点域名作为兜底说明里的链接（读不到时用默认值） */
function siteUrl() {
  try {
    const cfg = readFileSync(join(ROOT, 'site', 'src', 'config.ts'), 'utf8')
    const m = cfg.match(/url:\s*'([^']+)'/)
    if (m) return m[1].replace(/\/$/, '')
  } catch {
    /* 忽略，用默认值 */
  }
  return 'https://www.tossl.cn'
}

const version = (process.argv[2] ?? JSON.parse(readFileSync(join(ROOT, 'package.json'), 'utf8')).version).replace(/^v/, '')
if (!/^\d+\.\d+\.\d+$/.test(version)) throw new Error('版本号格式不正确：' + version)

const outFile = resolve(ROOT, process.argv[3] ?? 'updates/release-notes.md')
const changelogFile = join(ROOT, 'site', 'src', 'content', 'changelog', `v${version}.md`)

let notes
let source
if (existsSync(changelogFile)) {
  const raw = readFileSync(changelogFile, 'utf8')
  // 去掉 YAML frontmatter（--- 开头到下一个 --- 之间的内容）
  const body = raw.replace(/^---\r?\n[\s\S]*?\r?\n---\r?\n?/, '')
  notes = toPlainText(body)
  source = `site/src/content/changelog/v${version}.md`
} else {
  // 没写更新日志时不让发版流程失败，给一条指向更新日志页的兜底说明
  notes = `v${version} 版本更新，详情见 ${siteUrl()}/changelog/`
  source = '（未找到站点更新日志，使用兜底说明）'
}

mkdirSync(dirname(outFile), { recursive: true })
writeFileSync(outFile, notes + '\n')

console.log(`✓ 更新说明来源：${source}`)
console.log(`✓ 已生成：${outFile.replace(ROOT + '\\', '').replace(ROOT + '/', '')}`)
console.log('--- 内容预览 ---')
console.log(notes.split('\n').slice(0, 6).join('\n'))
if (notes.split('\n').length > 6) console.log('…')
