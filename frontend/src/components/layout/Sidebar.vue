<template>
  <n-menu
    :value="activeKey"
    :options="menuOptions"
    @update:value="handleMenuChange"
  />
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { NMenu } from 'naive-ui'
import { useRouter, useRoute } from 'vue-router'
import { useAuthStore } from '../../stores/auth'

const router = useRouter()
const route = useRoute()
const authStore = useAuthStore()

const activeKey = computed(() => {
  const path = route.path
  if (path.startsWith('/servers/')) return 'servers'
  return path.slice(1) || 'dashboard'
})

const menuOptions = computed(() => {
  const options: any[] = [
    {
      label: '仪表盘',
      key: 'dashboard'
    },
    {
      label: '服务器管理',
      key: 'servers'
    },
    {
      label: '版本管理',
      key: 'versions'
    },
    {
      label: '存档管理',
      key: 'saves'
    },
    {
      label: '设置',
      key: 'settings'
    }
  ]

  if (authStore.isAdmin) {
    options.push({
      label: '用户管理',
      key: 'users'
    })
  }

  return options
})

function handleMenuChange(key: string) {
  const pathMap: Record<string, string> = {
    'dashboard': '/',
    'servers': '/servers',
    'versions': '/versions',
    'saves': '/saves',
    'settings': '/settings',
    'users': '/users'
  }
  router.push(pathMap[key] || '/')
}
</script>
