<template>
  <div class="mod-card">
    <div class="mod-header">
      <h3>{{ mod.name }}</h3>
      <n-tag :type="mod.enabled ? 'success' : 'default'">
        {{ mod.enabled ? '启用' : '禁用' }}
      </n-tag>
    </div>

    <div class="mod-info">
      <div class="info-row">
        <span class="label">大小:</span>
        <span class="value">{{ formatSize(mod.file_size) }}</span>
      </div>
      <div class="info-row">
        <span class="label">上传时间:</span>
        <span class="value">{{ formatDate(mod.uploaded_at) }}</span>
      </div>
    </div>

    <div v-if="canManage" class="mod-actions">
      <n-button
        text
        :type="mod.enabled ? 'error' : 'success'"
        size="small"
        @click="$emit('toggle')"
      >
        {{ mod.enabled ? '禁用' : '启用' }}
      </n-button>
      <n-button
        text
        type="error"
        size="small"
        @click="$emit('delete')"
      >
        删除
      </n-button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { NButton, NTag } from 'naive-ui'

interface Props {
  mod: {
    name: string
    file_size: number
    enabled: boolean
    uploaded_at: string
  }
  canManage?: boolean
}

interface Emits {
  (e: 'toggle'): void
  (e: 'delete'): void
}

withDefaults(defineProps<Props>(), {
  canManage: true
})
defineEmits<Emits>()

function formatSize(bytes: number): string {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return Math.round(bytes / Math.pow(k, i) * 100) / 100 + ' ' + sizes[i]
}

function formatDate(dateString: string): string {
  try {
    const date = new Date(dateString)
    return date.toLocaleDateString('zh-CN') + ' ' + date.toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' })
  } catch {
    return dateString
  }
}
</script>

<style scoped>
.mod-card {
  background-color: var(--bg-card);
  border: 1px solid var(--border-color);
  border-radius: 12px;
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 12px;
  transition: background-color 0.3s, border-color 0.3s;
}

.mod-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.mod-header h3 {
  margin: 0;
  color: var(--text-primary);
  font-size: 14px;
  word-break: break-all;
}

.mod-info {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.info-row {
  display: flex;
  justify-content: space-between;
  font-size: 12px;
}

.label {
  color: var(--text-secondary);
}

.value {
  color: var(--text-primary);
}

.mod-actions {
  display: flex;
  gap: 8px;
  justify-content: center;
}
</style>
