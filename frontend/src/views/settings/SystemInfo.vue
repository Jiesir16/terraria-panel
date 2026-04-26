<template>
  <div class="system-info">
    <h2>系统信息</h2>
    <n-spin :show="loading">
      <div class="info-list">
        <div class="info-item">
          <span class="label">主机名:</span>
          <span class="value">{{ systemInfo?.hostname }}</span>
        </div>
        <div class="info-item">
          <span class="label">操作系统:</span>
          <span class="value">{{ systemInfo?.os_name || '' }} {{ systemInfo?.os_version || '' }}</span>
        </div>
        <div class="info-item">
          <span class="label">.NET 版本:</span>
          <span class="value">{{ systemInfo?.dotnet_version || 'N/A' }}</span>
        </div>
        <div class="info-item">
          <span class="label">Mono 版本:</span>
          <span class="value">{{ systemInfo?.mono_version || 'N/A' }}</span>
        </div>
        <div class="info-item">
          <span class="label">CPU 核心数:</span>
          <span class="value">{{ systemInfo?.cpu_count }}</span>
        </div>
        <div class="info-item">
          <span class="label">总内存:</span>
          <span class="value">{{ formatBytes(systemInfo?.memory_total || 0) }}</span>
        </div>
        <div class="info-item">
          <span class="label">可用内存:</span>
          <span class="value">{{ formatBytes((systemInfo?.memory_total || 0) - (systemInfo?.memory_used || 0)) }}</span>
        </div>
        <div class="info-item">
          <span class="label">总磁盘:</span>
          <span class="value">{{ formatBytes(systemInfo?.disk_total || 0) }}</span>
        </div>
        <div class="info-item">
          <span class="label">可用磁盘:</span>
          <span class="value">{{ formatBytes((systemInfo?.disk_total || 0) - (systemInfo?.disk_used || 0)) }}</span>
        </div>
      </div>
    </n-spin>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { NSpin } from 'naive-ui'
import { systemApi } from '../../api/system'
import { useNotification } from '../../composables/useNotification'

const notification = useNotification()
const loading = ref(false)
const systemInfo = ref<any>(null)

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return Math.round(bytes / Math.pow(k, i) * 100) / 100 + ' ' + sizes[i]
}

async function loadSystemInfo() {
  loading.value = true
  try {
    const response = await systemApi.getSystemInfo()
    systemInfo.value = response.data
  } catch (error) {
    notification.error('加载系统信息失败', '')
  } finally {
    loading.value = false
  }
}

onMounted(() => {
  loadSystemInfo()
})
</script>

<style scoped>
.system-info {
  background-color: var(--bg-card);
  border: 1px solid var(--border-color);
  border-radius: 12px;
  padding: 20px;
}

.system-info h2 {
  margin: 0 0 16px 0;
  color: var(--text-primary);
  font-size: 18px;
}

.info-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.info-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px 0;
  border-bottom: 1px solid var(--border-color);
}

.info-item:last-child {
  border-bottom: none;
}

.label {
  color: var(--text-secondary);
  font-weight: 500;
}

.value {
  color: var(--text-primary);
  font-family: "JetBrains Mono", monospace;
}
</style>
