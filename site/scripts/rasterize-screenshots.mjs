// 将 public/screenshots/*.svg 占位图栅格化为 1280x800 PNG，
// 使 Hero 等组件自动优先使用真实图片路径（代码逻辑：存在 PNG 即用 PNG）。
// 用法：node scripts/rasterize-screenshots.mjs
// 后期替换真实截图：直接把真机截图命名为 certs-view.png 等覆盖即可。
import { readFile, writeFile } from 'node:fs/promises'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import sharp from 'sharp'

const dir = path.join(path.dirname(fileURLToPath(import.meta.url)), '..', 'public', 'screenshots')
const names = ['certs-view', 'dns-view', 'wizard-view']
const WIDTH = 1280
const HEIGHT = 800

for (const name of names) {
  const svgPath = path.join(dir, `${name}.svg`)
  const pngPath = path.join(dir, `${name}.png`)
  try {
    const svg = await readFile(svgPath)
    const png = await sharp(svg, { density: 144 })
      .resize(WIDTH, HEIGHT, { fit: 'fill' })
      .png({ compressionLevel: 9 })
      .toBuffer()
    await writeFile(pngPath, png)
    console.log(`OK  ${name}.png  (${(png.length / 1024).toFixed(1)} KB)`)
  } catch (err) {
    console.error(`FAIL ${name}: ${err.message}`)
    process.exitCode = 1
  }
}
console.log('done')
