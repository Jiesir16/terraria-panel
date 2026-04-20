import { createRouter, createWebHistory, RouteRecordRaw } from 'vue-router'
import { createDiscreteApi } from 'naive-ui'
import { useAuthStore } from '../stores/auth'

const { loadingBar } = createDiscreteApi(['loadingBar'])

const routes: RouteRecordRaw[] = [
  {
    path: '/login',
    name: 'Login',
    component: () => import('../views/Login.vue'),
    meta: { requiresAuth: false }
  },
  {
    path: '/',
    component: () => import('../components/layout/AppLayout.vue'),
    meta: { requiresAuth: true },
    children: [
      {
        path: '',
        name: 'Dashboard',
        component: () => import('../views/Dashboard.vue')
      },
      {
        path: 'servers',
        name: 'ServerList',
        component: () => import('../views/ServerList.vue')
      },
      {
        path: 'servers/:id',
        name: 'ServerDetail',
        component: () => import('../views/ServerDetail.vue')
      },
      {
        path: 'versions',
        name: 'VersionManager',
        component: () => import('../views/VersionManager.vue')
      },
      {
        path: 'mods',
        name: 'ModManager',
        component: () => import('../views/ModManager.vue')
      },
      {
        path: 'saves',
        name: 'SaveManager',
        component: () => import('../views/SaveManager.vue')
      },
      {
        path: 'settings',
        name: 'Settings',
        component: () => import('../views/Settings.vue')
      },
      {
        path: 'users',
        name: 'UserManager',
        component: () => import('../views/UserManager.vue'),
        meta: { requiresAdmin: true }
      }
    ]
  },
  {
    path: '/:pathMatch(.*)*',
    redirect: '/'
  }
]

const router = createRouter({
  history: createWebHistory(),
  routes
})

router.beforeEach(async (to, _from, next) => {
  loadingBar.start()
  
  const authStore = useAuthStore()

  authStore.initFromStorage()

  if (!authStore.isAuthenticated && authStore.token) {
    try {
      await authStore.fetchCurrentUser()
    } catch {
      authStore.logout()
    }
  }

  const requiresAuth = to.meta.requiresAuth !== false
  const requiresAdmin = to.meta.requiresAdmin === true

  if (requiresAuth && !authStore.isAuthenticated) {
    next('/login')
    return
  }

  if (requiresAdmin && !authStore.isAdmin) {
    next('/')
    return
  }

  if (to.path === '/login' && authStore.isAuthenticated) {
    next('/')
    return
  }

  next()
})

router.afterEach(() => {
  loadingBar.finish()
})

router.onError(() => {
  loadingBar.error()
})

export default router
