<template>
  <div class="version-manager">
    <div class="header">
      <h1>TShock 版本管理</h1>
    </div>

    <div class="version-grid">
      <div class="version-section">
        <h2>已下载版本</h2>
        <n-spin :show="downloadedLoading">
          <div class="versions-list">
            <div v-if="downloadedVersions.length === 0" class="empty">
              暂无已下载版本
            </div>
            <div
              v-for="version in downloadedVersions"
              :key="version.version"
              class="version-item"
            >
              <div class="version-info">
                <div class="version-name">{{ version.name }}</div>
                <div class="version-date">{{ version.release_date }}</div>
              </div>
              <n-button
                text
                type="error"
                size="small"
                @click="handleDeleteVersion(version.version)"
              >
                删除
              </n-button>
            </div>
          </div>
        </n-spin>
      </div>

      <div class="version-section">
        <h2>可用版本</h2>
        <n-spin :show="availableLoading">
          <div class="versions-list">
            <div v-if="availableVersions.length === 0" class="empty">
              加载中...
            </div>
            <div
              v-for="version in availableVersions"
              :key="version.version"
              class="version-item"
            >
              <div class="version-info">
                <div class="version-name">{{ version.name }}</div>
                <div class="version-date">{{ version.release_date }}</div>
                <div class="version-size" v-if="version.size">
                  大小: {{ formatSize(version.size) }}
                </div>
              </div>
              <n-button
                v-if="!version.is_downloaded"
                text
                type="primary"
                size="small"
                :loading="downloadingVersion === version.version"
                @click="handleDownloadVersion(version.version)"
              >
                {{ downloadingVersion === version.version ? '下载中...' : '下载' }}
              </n-button>
              <n-tag v-else type="success">已下载</n-tag>
            </div>
          </div>
        </n-spin>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { NSpin, NButton, NTag } from 'naive-ui'
import { versionApi } from '../api/version'
import { useNotification } from '../composables/useNotification'

const notification = useNotification()

const downloadedLoading = ref(false)
const availableLoading = ref(false)
const downloadedVersions = ref<any[]>([])
const availableVersions = ref<any[]>([])
const downloadingVersion = ref<string | null>(null)

function formatSize(bytes: number): string {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return Math.round(bytes / Math.pow(k, i) * 100) / 100 + ' ' + sizes[i]
}

async function loadDownloadedVersions() {
  downloadedLoading.value = true
  try {
    const response = await versionApi.getDownloaded()
    downloadedVersions.value = response.data
  } catch (error) {
    notification.error('加载已下载版本失败', '')
  } finally {
    downloadedLoading.value = false
  }
}

async function loadAvailableVersions() {
  availableLoading.value = true
  try {
    const response = await versionApi.getAvailable()
    availableVersions.value = response.data
  } catch (error) {
    notification.error('加载可用版本失败', '')
  } finally {
    availableLoading.value = false
  }
}

async function handleDownloadVersion(version: string) {
  downloadingVersion.value = version
  try {
    await versionApi.download(version)
    notification.success('版本已下载', '')
    loadDownloadedVersions()
    loadAvailableVersions()
  } catch (error: any) {
    notification.error('下载失败', error?.response?.data?.message || '')
  } finally {
    downloadingVersion.value = null
  }
}

async function handleDeleteVersion(version: string) {
  try {
    await versionApi.delete(version)
    notification.success('版本已删除', '')
    loadDownloadedVersions()
    loadAvailableVersions()
  } catch (error: any) {
    notification.error('删除失败', error?.response?.data?.message || '')
  }
}

onMounted(() => {
  loadDownloadedVersions()
  loadAvailableVersions()
})
</script>

<style scoped>
.version-manager {
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.header h1 {
  margin: 0;
  color: var(--text-primary);
}

.version-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 20px;
}

.version-section {
  background-color: var(--bg-card);
  border: 1px solid var(--border-color);
  border-radius: 12px;
  padding: 20px;
  transition: background-color 0.3s, border-color 0.3s;
}

.version-section h2 {
  margin: 0 0 16px 0;
  color: var(--text-primary);
  font-size: 16px;
}

.versions-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.version-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px;
  background-color: var(--bg-body);
  border: 1px solid var(--border-color);
  border-radius: 8px;
  transition: background-color 0.3s, border-color 0.3s;
}

.version-info {
  flex: 1;
}

.version-name {
  color: var(--text-primary);
  font-weight: bold;
  margin-bottom: 4px;
}

.version-date,
.version-size {
  color: var(--text-secondary);
  font-size: 12px;
}

.empty {
  text-align: center;
  color: var(--text-muted);
  padding: 20px;
}

@media (max-width: 1024px) {
  .version-grid {
    grid-template-columns: 1fr;
  }
}
</style>
