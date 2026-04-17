<template>
  <div class="topbar">
    <div class="topbar-left">
      <h2>{{ pageTitle }}</h2>
    </div>
    <div class="topbar-right">
      <n-tooltip trigger="hover">
        <template #trigger>
          <n-button text @click="appStore.toggleTheme" class="theme-btn">
            {{ appStore.isDark ? '☀️' : '🌙' }}
          </n-button>
        </template>
        {{ appStore.isDark ? '切换到亮色主题' : '切换到暗色主题' }}
      </n-tooltip>
      <div class="user-info">
        <span class="username">{{ authStore.user?.username }}</span>
        <n-popconfirm
          @positive-click="handleLogout"
          positive-text="确认登出"
          negative-text="取消"
        >
          <template #trigger>
            <n-button text type="error">
              登出
            </n-button>
          </template>
          确定要退出登录吗？
        </n-popconfirm>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { NButton, NPopconfirm, NTooltip } from 'naive-ui'
import { useRoute, useRouter } from 'vue-router'
import { useAuthStore } from '../../stores/auth'
import { useAppStore } from '../../stores/app'

const route = useRoute()
const router = useRouter()
const authStore = useAuthStore()
const appStore = useAppStore()

const pageTitle = computed(() => {
  const titles: Record<string, string> = {
    'Dashboard': '仪表盘',
    'ServerList': '服务器管理',
    'ServerDetail': '服务器详情',
    'VersionManager': '版本管理',
    'ModManager': 'Mod 管理',
    'SaveManager': '存档管理',
    'Settings': '系统设置',
    'UserManager': '用户管理'
  }
  return titles[route.name as string] || '泰拉瑞亚控制台'
})

function handleLogout() {
  authStore.logout()
  router.push('/login')
}
</script>

<style scoped>
.topbar {
  height: 60px;
  background-color: var(--bg-card);
  border-bottom: 1px solid var(--border-color);
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 20px;
  transition: background-color 0.3s, border-color 0.3s;
}

.topbar-left h2 {
  color: var(--text-primary);
  font-size: 20px;
  margin: 0;
  transition: color 0.3s;
}

.topbar-right {
  display: flex;
  align-items: center;
  gap: 20px;
}

.user-info {
  display: flex;
  align-items: center;
  gap: 12px;
}

.username {
  color: var(--text-secondary);
  font-size: 14px;
  transition: color 0.3s;
}

.theme-btn {
  font-size: 18px;
  cursor: pointer;
}
</style>
