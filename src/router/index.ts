import { createRouter, createWebHashHistory } from 'vue-router'

export const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    {
      path: '/',
      component: () => import('../pages/MainPage.vue'),
      children: [
        {
          path: '',
          name: 'home',
          component: () => import('../pages/HomePage.vue'),
          meta: { titleKey: 'nav.home' }
        },
        {
          path: 'launcher',
          name: 'launcher',
          component: () => import('../pages/LauncherPage.vue'),
          meta: { titleKey: 'nav.launcher' }
        },
        {
          path: 'gacha',
          name: 'gacha',
          component: () => import('../pages/GachaPage.vue'),
          meta: { titleKey: 'nav.gacha' }
        },
        {
          path: 'settings',
          name: 'settings',
          component: () => import('../pages/SettingsPage.vue'),
          meta: { titleKey: 'nav.settings' }
        }
      ]
    },
    {
      path: '/guide',
      component: () => import('../pages/GuidePage/Main.vue'),
      redirect: '/guide/welcome',
      meta: { titleKey: 'guide.title' },
      children: [
        {
          path: 'welcome',
          name: 'guide-welcome',
          component: () => import('../pages/GuidePage/Welcome.vue')
        },
        {
          path: 'disclaimer',
          name: 'guide-disclaimer',
          component: () => import('../pages/GuidePage/Disclaimer.vue')
        },
        {
          path: 'resource',
          name: 'guide-resource',
          component: () => import('../pages/GuidePage/Resource.vue')
        },
        {
          path: 'ready',
          name: 'guide-ready',
          component: () => import('../pages/GuidePage/Ready.vue')
        },
        {
          path: 'update',
          name: 'guide-update',
          component: () => import('../pages/GuidePage/Update.vue')
        }
      ]
    }
  ]
})

export default router
