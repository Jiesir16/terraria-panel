<template>
  <div class="save-card">
    <div class="save-header">
      <h3>{{ save.name }}</h3>
    </div>

    <div class="save-info">
      <div class="info-row">
        <span class="label">大小:</span>
        <span class="value">{{ formatSize(save.file_size) }}</span>
      </div>
      <div class="info-row">
        <span class="label">创建时间:</span>
        <span class="value">{{ formatDate(save.created_at) }}</span>
      </div>
      <div class="info-row" v-if="save.source_server_id">
        <span class="label">来源:</span>
        <span class="value">{{ save.source_server_id }}</span>
      </div>
    </div>

    <div class="save-actions">
      <n-button
        v-if="serverId"
        text
        type="primary"
        size="small"
        @click="$emit('import')"
      >
        导入
      </n-button>
      <n-button
        text
        type="info"
        size="small"
        @click="$emit('download')"
      >
        下载
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
import { NButton } from 'naive-ui'

interface Props {
  save: {
    id: string
    name: string
    file_size: number
    created_at: string
    source_server_id?: string
  }
  serverId: string
}

interface Emits {
  (e: 'import'): void
  (e: 'download'): void
  (e: 'delete'): void
}

defineProps<Props>()
defineEmits<Emits>()

function formatSize(bytes: number): string {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB']
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
.save-card {
  background-color: var(--bg-card);
  border: 1px solid var(--border-color);
  border-radius: 12px;
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 12px;
  transition: background-color 0.3s, border-color 0.3s;
}

.save-header h3 {
  margin: 0;
  color: var(--text-primary);
  font-size: 14px;
  word-break: break-all;
}

.save-info {
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

.save-actions {
  display: flex;
  gap: 8px;
  justify-content: center;
  flex-wrap: wrap;
}
</style>
