<template>
  <n-menu
    :value="activeKey"
    :options="menuOptions"
    :expanded-keys="expandedKeys"
    @update:value="handleMenuChange"
    @update:expanded-keys="handleExpandedChange"
  />
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { NMenu } from 'naive-ui'
import { useRouter, useRoute } from 'vue-router'
import { useAuthStore } from '../../stores/auth'

const router = useRouter()
const route = useRoute()
const authStore = useAuthStore()

const activeKey = computed(() => {
  const path = route.path
  if (path.startsWith('/servers/')) return 'servers'
  if (path.startsWith('/settings/')) {
    const sub = path.slice('/settings/'.length).split('/')[0]
    return `settings-${sub}`
  }
  if (path === '/settings') return 'settings-system'
  return path.slice(1) || 'dashboard'
})

const expandedKeys = ref<string[]>([])

watch(
  () => route.path,
  (path) => {
    if (path.startsWith('/settings') && !expandedKeys.value.includes('settings')) {
      expandedKeys.value = [...expandedKeys.value, 'settings']
    }
  },
  { immediate: true }
)

const menuOptions = computed(() => {
  const settingsChildren: any[] = [
    { label: '系统信息', key: 'settings-system' }
  ]

  if (authStore.isOperator) {
    settingsChildren.push({ label: '备份策略', key: 'settings-backup' })
    settingsChildren.push({ label: 'FRP 设置', key: 'settings-frp' })
  }

  settingsChildren.push({ label: '用户信息', key: 'settings-user' })

  const options: any[] = [
    { label: '仪表盘', key: 'dashboard' },
    { label: '服务器管理', key: 'servers' },
    { label: '版本管理', key: 'versions' },
    { label: '存档管理', key: 'saves' },
    {
      label: '设置',
      key: 'settings',
      children: settingsChildren
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

const pathMap: Record<string, string> = {
  'dashboard': '/',
  'servers': '/servers',
  'versions': '/versions',
  'saves': '/saves',
  'settings-system': '/settings/system',
  'settings-backup': '/settings/backup',
  'settings-frp': '/settings/frp',
  'settings-user': '/settings/user',
  'users': '/users'
}

function handleMenuChange(key: string) {
  const target = pathMap[key]
  if (target) router.push(target)
}

function handleExpandedChange(keys: string[]) {
  expandedKeys.value = keys
}
</script>
