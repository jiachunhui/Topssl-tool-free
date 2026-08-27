import { createRouter, createWebHashHistory } from 'vue-router'

const router = createRouter({
  // Tauri 生产环境按自定义协议加载静态文件、无服务端 SPA fallback，
  // 用 hash 历史避免深链/刷新子路由白屏（轻微问题 8）
  history: createWebHashHistory(),
  routes: [
    {
      path: '/',
      name: 'home',
      component: () => import('../views/HomeView.vue'),
      meta: { title: '我的证书' },
    },
    {
      path: '/wizard',
      name: 'wizard',
      component: () => import('../views/WizardView.vue'),
      meta: { title: '申请证书' },
    },
    {
      path: '/dns',
      name: 'dns',
      component: () => import('../views/DnsView.vue'),
      meta: { title: 'DNS 配置' },
    },
    {
      path: '/settings',
      name: 'settings',
      component: () => import('../views/SettingsView.vue'),
      meta: { title: '设置' },
    },
    {
      path: '/logs',
      name: 'logs',
      component: () => import('../views/LogsView.vue'),
      meta: { title: '应用日志' },
    },
    {
      path: '/about',
      name: 'about',
      component: () => import('../views/AboutView.vue'),
      meta: { title: '关于' },
    },
  ],
})

router.afterEach((to) => {
  document.title = to.meta.title ? `${to.meta.title} - ToSSL 免费SSL证书管理工具` : 'ToSSL 免费SSL证书管理工具'
})

export default router
