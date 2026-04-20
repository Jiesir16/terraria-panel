<template>
  <n-layout has-sider :native-scrollbar="false" style="height: 100vh;">
    <n-layout-sider
      :collapsed="appStore.sidebarCollapsed"
      :width="240"
      :collapsed-width="64"
      show-trigger="bar"
      breakpoint="lg"
      :native-scrollbar="false"
      @collapse="appStore.setSidebarCollapsed(true)"
      @expand="appStore.setSidebarCollapsed(false)"
    >
      <div class="sidebar-header">
        <span v-if="!appStore.sidebarCollapsed" class="sidebar-title">🎮 Terraria</span>
        <span v-else class="sidebar-icon">🎮</span>
      </div>
      <sidebar />
    </n-layout-sider>
    <n-layout :native-scrollbar="false">
      <top-bar />
      <n-layout-content
        :native-scrollbar="false"
        :content-style="contentPadding"
      >
        <router-view v-slot="{ Component, route }">
          <component :is="Component" :key="route.fullPath" />
        </router-view>
      </n-layout-content>
    </n-layout>
  </n-layout>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue'
import { NLayout, NLayoutSider, NLayoutContent } from 'naive-ui'
import { useAppStore } from '../../stores/app'
import Sidebar from './Sidebar.vue'
import TopBar from './TopBar.vue'

const appStore = useAppStore()
const windowWidth = ref(window.innerWidth)

function onResize() {
  windowWidth.value = window.innerWidth
}

onMounted(() => window.addEventListener('resize', onResize))
onUnmounted(() => window.removeEventListener('resize', onResize))

const contentPadding = computed(() => {
  if (windowWidth.value <= 480) return 'padding: 8px;'
  if (windowWidth.value <= 768) return 'padding: 12px;'
  return 'padding: 20px;'
})
</script>

<style scoped>
.sidebar-header {
  padding: 16px;
  border-bottom: 1px solid var(--border-color);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 18px;
  font-weight: bold;
  height: 60px;
  transition: border-color 0.3s;
}

.sidebar-title {
  color: var(--color-primary);
  white-space: nowrap;
}

.sidebar-icon {
  font-size: 24px;
}
</style>
