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
        content-style="padding: 20px;"
      >
        <router-view v-slot="{ Component }">
          <keep-alive>
            <component :is="Component" />
          </keep-alive>
        </router-view>
      </n-layout-content>
    </n-layout>
  </n-layout>
</template>

<script setup lang="ts">
import { NLayout, NLayoutSider, NLayoutContent } from 'naive-ui'
import { useAppStore } from '../../stores/app'
import Sidebar from './Sidebar.vue'
import TopBar from './TopBar.vue'

const appStore = useAppStore()
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
