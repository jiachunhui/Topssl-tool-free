import { createRouter, createWebHistory } from 'vue-router'

const router = createRouter({
  history: createWebHistory(),
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
  document.title = to.meta.title ? `${to.meta.title} - SSL 证书助手` : 'SSL 证书助手'
})

export default router
