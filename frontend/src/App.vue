<template>
  <n-config-provider :theme="currentTheme" :theme-overrides="currentThemeOverrides">
    <n-message-provider>
      <n-dialog-provider>
        <router-view />
        <notification-container />
      </n-dialog-provider>
    </n-message-provider>
  </n-config-provider>
</template>

<script setup lang="ts">
import { computed, watchEffect } from 'vue'
import { NConfigProvider, NMessageProvider, NDialogProvider, darkTheme, type GlobalThemeOverrides } from 'naive-ui'
import NotificationContainer from './components/common/NotificationContainer.vue'
import { useAppStore } from './stores/app'
import './styles/global.css'

const appStore = useAppStore()

const darkThemeOverrides: GlobalThemeOverrides = {
  common: {
    primaryColor: '#50C878',
    primaryColorHover: '#6FE096',
    primaryColorPressed: '#3DAF65',
    successColor: '#50C878',
    warningColor: '#FFB347',
    errorColor: '#FF6B6B',
    infoColor: '#64B5F6',
    textColor1: '#E8E8E8',
    textColor2: '#B0B0B0',
    textColor3: '#808080',
    bodyColor: '#0D1117',
    cardColor: '#161B22',
    modalColor: '#161B22',
    popoverColor: '#1C2128',
    tableColor: '#161B22',
    inputColor: '#0D1117',
    borderColor: '#30363D',
    dividerColor: '#21262D',
    hoverColor: '#1C2128',
    borderRadius: '8px'
  },
  Card: {
    borderRadius: '12px',
    borderColor: '#30363D'
  },
  Button: {
    borderRadiusMedium: '8px'
  },
  Menu: {
    itemColorActive: 'rgba(80, 200, 120, 0.1)',
    itemTextColorActive: '#50C878',
    itemIconColorActive: '#50C878'
  },
  Tag: {
    borderRadius: '6px'
  }
}

const lightThemeOverrides: GlobalThemeOverrides = {
  common: {
    primaryColor: '#389E5C',
    primaryColorHover: '#50C878',
    primaryColorPressed: '#2D8A4E',
    successColor: '#389E5C',
    warningColor: '#E5983E',
    errorColor: '#E05252',
    infoColor: '#4A9AE0',
    bodyColor: '#F5F7FA',
    cardColor: '#FFFFFF',
    modalColor: '#FFFFFF',
    popoverColor: '#FFFFFF',
    tableColor: '#FFFFFF',
    inputColor: '#F0F2F5',
    borderColor: '#D9DEE3',
    dividerColor: '#E8ECF0',
    hoverColor: '#F0F2F5',
    borderRadius: '8px'
  },
  Card: {
    borderRadius: '12px',
    borderColor: '#D9DEE3'
  },
  Button: {
    borderRadiusMedium: '8px'
  },
  Menu: {
    itemColorActive: 'rgba(56, 158, 92, 0.08)',
    itemTextColorActive: '#389E5C',
    itemIconColorActive: '#389E5C'
  },
  Tag: {
    borderRadius: '6px'
  }
}

const currentTheme = computed(() => appStore.isDark ? darkTheme : null)
const currentThemeOverrides = computed(() => appStore.isDark ? darkThemeOverrides : lightThemeOverrides)

// Sync data-theme attribute — CSS variables in global.css handle the rest
watchEffect(() => {
  document.documentElement.setAttribute('data-theme', appStore.isDark ? 'dark' : 'light')
})
</script>
