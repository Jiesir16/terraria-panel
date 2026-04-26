<template>
  <div class="notification-container">
    <div
      v-for="notification in appStore.notifications"
      :key="notification.id"
      class="notification-item"
      :class="`notification-${notification.type}`"
    >
      <div class="notification-content">
        <div class="notification-title">{{ notification.title }}</div>
        <div v-if="notification.message" class="notification-message">
          {{ notification.message }}
        </div>
      </div>
      <button class="notification-close" @click="appStore.removeNotification(notification.id)">
        ×
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useAppStore } from '../../stores/app'

const appStore = useAppStore()
</script>

<style scoped>
.notification-container {
  position: fixed;
  top: 20px;
  right: 20px;
  z-index: 3000;
  display: flex;
  flex-direction: column;
  gap: 12px;
  pointer-events: none;
}

.notification-item {
  min-width: 320px;
  max-width: 420px;
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  padding: 14px 16px;
  border-radius: 12px;
  border: 1px solid var(--border-color);
  background: var(--bg-card);
  box-shadow: 0 12px 28px rgba(0, 0, 0, 0.22);
  pointer-events: auto;
}

.notification-success {
  border-color: rgba(80, 200, 120, 0.45);
}

.notification-error {
  border-color: rgba(255, 107, 107, 0.5);
}

.notification-warning {
  border-color: rgba(255, 179, 71, 0.5);
}

.notification-info {
  border-color: rgba(100, 181, 246, 0.5);
}

.notification-content {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.notification-title {
  color: var(--text-primary);
  font-size: 14px;
  font-weight: 600;
}

.notification-message {
  color: var(--text-secondary);
  font-size: 13px;
  line-height: 1.45;
  word-break: break-word;
}

.notification-close {
  border: none;
  background: transparent;
  color: var(--text-secondary);
  font-size: 18px;
  line-height: 1;
  cursor: pointer;
  padding: 0;
}
</style>
