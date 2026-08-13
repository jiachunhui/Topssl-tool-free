import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import tailwindcss from '@tailwindcss/vite'

// https://vite.dev/config/
export default defineConfig({
  plugins: [vue(), tailwindcss()],
  // Tauri 需要固定端口，且不自动打开浏览器
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
    watch: {
      // 忽略 Rust 目录变更避免热重载循环
      ignored: ['**/src-tauri/**'],
    },
  },
  build: {
    // 减小 Tauri 打包体积
    target: 'es2021',
    minify: 'esbuild',
    sourcemap: false,
  },
})
