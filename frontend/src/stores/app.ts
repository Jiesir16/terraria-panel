import { defineStore } from 'pinia'
import { ref, computed } from 'vue'

export type ThemeMode = 'dark' | 'light'

export interface Notification {
  id: string
  type: 'success' | 'error' | 'warning' | 'info'
  title: string
  message: string
  duration?: number
}

export const useAppStore = defineStore('app', () => {
  const notifications = ref<Notification[]>([])
  const sidebarCollapsed = ref(false)
  const themeMode = ref<ThemeMode>(
    (localStorage.getItem('theme_mode') as ThemeMode) || 'dark'
  )

  const isDark = computed(() => themeMode.value === 'dark')

  function toggleTheme() {
    themeMode.value = themeMode.value === 'dark' ? 'light' : 'dark'
    localStorage.setItem('theme_mode', themeMode.value)
  }

  function addNotification(
    type: 'success' | 'error' | 'warning' | 'info',
    title: string,
    message: string,
    duration = 3000
  ) {
    const id = Date.now().toString()
    const notification: Notification = {
      id,
      type,
      title,
      message,
      duration
    }
    notifications.value.push(notification)

    if (duration > 0) {
      setTimeout(() => {
        removeNotification(id)
      }, duration)
    }

    return id
  }

  function removeNotification(id: string) {
    notifications.value = notifications.value.filter(n => n.id !== id)
  }

  function toggleSidebar() {
    sidebarCollapsed.value = !sidebarCollapsed.value
  }

  function setSidebarCollapsed(collapsed: boolean) {
    sidebarCollapsed.value = collapsed
  }

  return {
    notifications,
    sidebarCollapsed,
    themeMode,
    isDark,
    addNotification,
    removeNotification,
    toggleSidebar,
    setSidebarCollapsed,
    toggleTheme
  }
})
