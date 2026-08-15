import type { APIRoute } from 'astro'
import { SITE } from '../config'

// 动态生成 robots.txt：Sitemap 地址自动跟随 src/config.ts 的 SITE.url
export const GET: APIRoute = () =>
  new Response(
    ['User-agent: *', 'Allow: /', '', `Sitemap: ${SITE.url}/sitemap-index.xml`, ''].join('\n'),
    { headers: { 'Content-Type': 'text/plain; charset=utf-8' } },
  )
