#!/usr/bin/env node
// 生成 updates/latest.json —— 应用内「检查更新」的国内清单（与宣传页同域部署）
// 内容：版本号 / 更新说明 / 各平台安装包地址 + 大小 + sha256（应用下载后校验）
//
// 用法：
//   node scripts/gen-update-manifest.mjs \
//     --version 0.1.4 \
//     --base-url https://你的域名/download \
//     --win "src-tauri/target/release/bundle/nsis/TopSSL-Free-Cert-Assistant_0.1.4_x64-setup.exe" \
//     [--mac-aarch64 xxx.dmg] [--mac-x64 xxx.dmg] [--linux-appimage xxx.AppImage] \
//     [--notes-file notes.md]   # 更新说明文本；缺省时 notes 为 null
//
// 产物：updates/latest.json（已加入 .gitignore，直接上传到虚拟主机 /updates/ 目录即可）
import { createHash } from 'node:crypto'
import { mkdirSync, readFileSync, statSync, writeFileSync } from 'node:fs'
import { basename, dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..')

function parseArgs(argv) {
  const args = { platforms: {} }
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i]
    if (!a.startsWith('--')) throw new Error('未知参数：' + a)
    const key = a.slice(2)
    if (key === 'notes-file') {
      args[key] = argv[++i]
    } else if (['version', 'base-url'].includes(key)) {
      args[key] = argv[++i]
    } else if (['win', 'mac-aarch64', 'mac-x64', 'linux-appimage'].includes(key)) {
      args.platforms[key] = argv[++i]
    } else {
      throw new Error('未知参数：' + a)
    }
  }
  return args
}

const args = parseArgs(process.argv.slice(2))
const version = args['version'] ?? JSON.parse(readFileSync(join(ROOT, 'package.json'), 'utf8')).version
if (!/^\d+\.\d+\.\d+$/.test(version)) throw new Error('版本号格式不正确：' + version)
const baseUrl = (args['base-url'] ?? '').replace(/\/$/, '')
if (!baseUrl) throw new Error('缺少 --base-url（安装包在更新服务器上的目录，如 https://你的域名/download）')

const PLATFORM_KEYS = {
  win: 'windows-x86_64',
  'mac-aarch64': 'darwin-aarch64',
  'mac-x64': 'darwin-x86_64',
  'linux-appimage': 'linux-x86_64',
}

function describeAsset(file) {
  const abs = resolve(ROOT, file)
  const size = statSync(abs).size
  const sha256 = createHash('sha256').update(readFileSync(abs)).digest('hex')
  return { url: `${baseUrl}/${encodeURI(basename(abs))}`, size, sha256 }
}

const platforms = {}
for (const [flag, key] of Object.entries(PLATFORM_KEYS)) {
  if (!args.platforms[flag]) continue
  platforms[key] = describeAsset(args.platforms[flag])
  console.log(`✓ ${key}: ${platforms[key].url}（${(platforms[key].size / 1048576).toFixed(1)} MB, sha256=${platforms[key].sha256.slice(0, 16)}…）`)
}
if (Object.keys(platforms).length === 0) throw new Error('未指定任何平台安装包（--win / --mac-aarch64 / --mac-x64 / --linux-appimage）')

const notes = args['notes-file'] ? readFileSync(resolve(ROOT, args['notes-file']), 'utf8').trim() : null

const manifest = {
  version,
  notes,
  publishedAt: new Date().toISOString(),
  platforms,
}

const outDir = join(ROOT, 'updates')
mkdirSync(outDir, { recursive: true })
const outFile = join(outDir, 'latest.json')
writeFileSync(outFile, JSON.stringify(manifest, null, 2) + '\n')
console.log('')
console.log('已生成：' + outFile)
console.log('上传到虚拟主机 /updates/latest.json，并将上述安装包上传到 ' + baseUrl + ' 目录')
console.log('最后在 src-tauri/src/updater/mod.rs 中把 UPDATE_MANIFEST_URL 填为：<域名>/updates/latest.json（如 https://你的域名/updates/latest.json）')
