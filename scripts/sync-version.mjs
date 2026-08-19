#!/usr/bin/env node
// 一键同步全部版本号（单源写入，避免手工改漏）：
//   package.json / src-tauri/tauri.conf.json / src-tauri/Cargo.toml
//   src-tauri/Cargo.lock / site/src/config.ts / docs/用户手册.md / src/lib/mock.ts
//
// 用法：
//   node scripts/sync-version.mjs 0.1.4             # 全部改为 0.1.4
//   node scripts/sync-version.mjs --check v0.1.4    # 校验（发布 CI 用）：不一致则退出码 1
import { readFileSync, writeFileSync } from 'node:fs'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const SEMVER = /^\d+\.\d+\.\d+$/

/** 需要同步版本号的文件（顺序无要求，全部都会读 → 改 / 校验） */
const TARGETS = [
  {
    label: 'package.json',
    path: 'package.json',
    get: () => JSON.parse(readFileSync(join(ROOT, 'package.json'), 'utf8')).version,
    set: (v) => replaceRegex(join(ROOT, 'package.json'), /("version"\s*:\s*")[^"]*(")/, v),
  },
  {
    label: 'src-tauri/tauri.conf.json',
    path: 'src-tauri/tauri.conf.json',
    get: () => JSON.parse(readFileSync(join(ROOT, 'src-tauri', 'tauri.conf.json'), 'utf8')).version,
    set: (v) => replaceRegex(join(ROOT, 'src-tauri', 'tauri.conf.json'), /("version"\s*:\s*")[^"]*(")/, v),
  },
  {
    label: 'src-tauri/Cargo.toml',
    path: 'src-tauri/Cargo.toml',
    get: () => {
      const m = readFileSync(join(ROOT, 'src-tauri', 'Cargo.toml'), 'utf8').match(/^version = "([^"]+)"/m)
      if (!m) throw new Error('Cargo.toml 中未找到 [package] version')
      return m[1]
    },
    set: (v) => replaceRegex(join(ROOT, 'src-tauri', 'Cargo.toml'), /^version = "[^"]+"/m, v, (n) => `version = "${n}"`),
  },
  {
    label: 'src-tauri/Cargo.lock',
    path: 'src-tauri/Cargo.lock',
    get: () => {
      const m = readFileSync(join(ROOT, 'src-tauri', 'Cargo.lock'), 'utf8').match(
        /name = "topssl-free-cert-assistant"\r?\nversion = "([^"]+)"/,
      )
      if (!m) throw new Error('Cargo.lock 中未找到 topssl-free-cert-assistant 条目')
      return m[1]
    },
    set: (v) =>
      replaceRegex(
        join(ROOT, 'src-tauri', 'Cargo.lock'),
        /(name = "topssl-free-cert-assistant"\r?\nversion = ")[^"]*(")/,
        v,
        (n) => `name = "topssl-free-cert-assistant"\nversion = "${n}"`,
      ),
  },
  {
    label: 'docs/用户手册.md（文档头部版本）',
    path: 'docs/用户手册.md',
    get: () => {
      const m = readFileSync(join(ROOT, 'docs', '用户手册.md'), 'utf8').match(/^版本：([^\r\n]+)/m)
      if (!m) throw new Error('docs/用户手册.md 中未找到头部版本号')
      return m[1].trim()
    },
    set: (v) => replaceRegex(join(ROOT, 'docs', '用户手册.md'), /^(版本：).*$/m, v, (n) => `版本：${n}`),
  },
  {
    label: 'site/src/config.ts（宣传页展示版本）',
    path: 'site/src/config.ts',
    get: () => {
      const m = readFileSync(join(ROOT, 'site', 'src', 'config.ts'), 'utf8').match(/version: '([^']+)'/)
      if (!m) throw new Error('site/src/config.ts 中未找到 version')
      return m[1]
    },
    set: (v) => replaceRegex(join(ROOT, 'site', 'src', 'config.ts'), /(version: ')[^']*(')/, v, (n) => `version: '${n}'`),
  },
  {
    label: 'src/lib/mock.ts（浏览器预览当前版本）',
    path: 'src/lib/mock.ts',
    get: () => {
      const m = readFileSync(join(ROOT, 'src', 'lib', 'mock.ts'), 'utf8').match(/const current = '([^']+)'/)
      if (!m) throw new Error('src/lib/mock.ts 中未找到 const current 版本号')
      return m[1]
    },
    set: (v) => replaceRegex(join(ROOT, 'src', 'lib', 'mock.ts'), /(const current = ')[^']*(')/, v, (n) => `const current = '${n}'`),
  },
]

function replaceRegex(file, regex, version, build) {
  const raw = readFileSync(file, 'utf8')
  const replacement = build ? build(version) : `$1${version}$2`
  const next = raw.replace(regex, replacement)
  if (next === raw) throw new Error(`未能修改 ${file}（未匹配到版本号）`)
  writeFileSync(file, next)
}

const mode = process.argv[2]
const rawVer = mode === '--check' ? process.argv[3] : mode

if (!rawVer) {
  console.error('用法：node scripts/sync-version.mjs <版本号>    或    node scripts/sync-version.mjs --check <v版本号>')
  process.exit(1)
}
const version = rawVer.replace(/^v/, '')
if (!SEMVER.test(version)) {
  console.error(`版本号格式不正确：${rawVer}（应为 x.y.z，如 0.1.4）`)
  process.exit(1)
}

if (mode === '--check') {
  const mismatches = []
  for (const t of TARGETS) {
    try {
      const current = t.get()
      if (current !== version) mismatches.push(`${t.label}: 实际 ${current} ≠ 期望 ${version}`)
    } catch (e) {
      mismatches.push(`${t.label}: 读取失败（${e.message}）`)
    }
  }
  if (mismatches.length > 0) {
    console.error('❌ 版本不一致，禁止发布：')
    for (const m of mismatches) console.error('  - ' + m)
    console.error('请先执行：node scripts/sync-version.mjs ' + version)
    process.exit(1)
  }
  console.log(`✅ 版本校验通过：所有位置均为 ${version}`)
  process.exit(0)
}

for (const t of TARGETS) {
  t.set(version)
  console.log(`✓ ${t.label} → ${version}`)
}
console.log('')
console.log('发布流程：')
console.log('  1. 修改代码 / 更新日志')
console.log('  2. git commit + git tag v' + version + ' + git push --tags  →  CI 自动构建并发布 GitHub Release')
console.log('  3. 构建出安装包后，运行 npm run update:manifest 生成 updates/latest.json，与安装包一并上传到更新服务器（虚拟主机）')
