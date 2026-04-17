<template>
  <div class="version-manager">
    <div class="header">
      <h1>TShock 版本管理</h1>
    </div>

    <!-- Proxy Settings -->
    <div class="proxy-section">
      <div class="proxy-header" @click="showProxy = !showProxy">
        <span class="proxy-title">GitHub 代理设置</span>
        <span class="proxy-toggle">{{ showProxy ? '收起' : '展开' }}</span>
      </div>
      <div v-if="showProxy" class="proxy-body">
        <p class="proxy-hint">
          大陆用户访问 GitHub 可能较慢，可配置代理加速下载。常见代理地址如：
          <code>https://ghproxy.com</code>、<code>https://gh-proxy.com</code>、<code>https://mirror.ghproxy.com</code>
          等。留空则直连 GitHub。
        </p>
        <div class="proxy-form">
          <n-input
            v-model:value="proxyUrl"
            placeholder="输入 GitHub 代理地址，如 https://ghproxy.com"
            clearable
            :disabled="proxySaving"
          />
          <n-button
            type="primary"
            :loading="proxySaving"
            @click="handleSaveProxy"
          >
            保存
          </n-button>
        </div>
        <div v-if="currentProxy" class="proxy-current">
          当前代理: <code>{{ currentProxy }}</code>
        </div>
        <div v-else class="proxy-current">
          当前: 直连 GitHub（无代理）
        </div>
      </div>
    </div>

    <div class="version-grid">
      <!-- Downloaded Versions -->
      <div class="version-section">
        <div class="section-header">
          <h2>已下载版本</h2>
          <span class="count-badge" v-if="downloadedVersions.length > 0">
            {{ downloadedVersions.length }}
          </span>
        </div>
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
                <div class="version-name">
                  {{ version.name }}
                  <n-tag v-if="version.is_dotnet" size="tiny" type="info" class="runtime-tag">.NET</n-tag>
                  <n-tag v-else size="tiny" type="warning" class="runtime-tag">Mono</n-tag>
                </div>
                <div class="version-meta">
                  <span class="version-tag">{{ version.version }}</span>
                  <span v-if="version.size" class="version-size">{{ formatSize(version.size) }}</span>
                  <span v-if="version.installed_at" class="version-date">{{ version.installed_at }}</span>
                </div>
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

      <!-- Available Versions -->
      <div class="version-section">
        <div class="section-header">
          <h2>可用版本</h2>
          <span class="count-badge" v-if="totalAvailable > 0">
            {{ totalAvailable }}
          </span>
        </div>
        <n-spin :show="availableLoading && availablePage === 1">
          <div class="versions-list">
            <div v-if="availableVersions.length === 0 && !availableLoading" class="empty">
              无法获取版本列表，请检查网络连接或配置代理
            </div>
            <div v-if="availableVersions.length === 0 && availableLoading" class="empty">
              正在获取版本列表...
            </div>
            <div
              v-for="version in availableVersions"
              :key="version.version"
              class="version-item"
            >
              <div class="version-info">
                <div class="version-name">{{ version.name }}</div>
                <div class="version-meta">
                  <span class="version-tag">{{ version.tag_name }}</span>
                  <span v-if="version.size" class="version-size">{{ formatSize(version.size) }}</span>
                  <span class="version-date">{{ formatDate(version.published_at) }}</span>
                </div>
              </div>
              <n-button
                v-if="!version.downloaded"
                text
                type="primary"
                size="small"
                :loading="downloadingVersion === version.version"
                @click="handleDownloadVersion(version)"
              >
                {{ downloadingVersion === version.version ? '下载中...' : '下载' }}
              </n-button>
              <n-tag v-else type="success" size="small">已下载</n-tag>
            </div>

            <!-- Load More -->
            <div v-if="hasMore" class="load-more">
              <n-button
                text
                type="primary"
                :loading="loadingMore"
                @click="loadMoreVersions"
              >
                {{ loadingMore ? '加载中...' : '加载更多版本' }}
              </n-button>
              <span class="load-more-hint">
                已显示 {{ availableVersions.length }} / {{ totalAvailable }} 个版本
              </span>
            </div>
          </div>
        </n-spin>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { NSpin, NButton, NTag, NInput } from 'naive-ui'
import { versionApi } from '../api/version'
import type { VersionInfo, LocalVersion } from '../api/version'
import { useNotification } from '../composables/useNotification'

const notification = useNotification()

const downloadedLoading = ref(false)
const availableLoading = ref(false)
const loadingMore = ref(false)
const downloadedVersions = ref<LocalVersion[]>([])
const availableVersions = ref<VersionInfo[]>([])
const downloadingVersion = ref<string | null>(null)
const availablePage = ref(1)
const perPage = 10
const totalAvailable = ref(0)
const hasMore = ref(false)

// Proxy state
const showProxy = ref(false)
const proxyUrl = ref('')
const currentProxy = ref('')
const proxySaving = ref(false)

function formatSize(bytes: number): string {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return Math.round(bytes / Math.pow(k, i) * 100) / 100 + ' ' + sizes[i]
}

function formatDate(dateStr: string): string {
  if (!dateStr) return ''
  try {
    const date = new Date(dateStr)
    return date.toLocaleDateString('zh-CN', {
      year: 'numeric',
      month: '2-digit',
      day: '2-digit'
    })
  } catch {
    return dateStr
  }
}

async function loadProxy() {
  try {
    const response = await versionApi.getProxy()
    currentProxy.value = response.data.mirror || ''
    proxyUrl.value = currentProxy.value
  } catch {
    // silently fail
  }
}

async function handleSaveProxy() {
  proxySaving.value = true
  try {
    await versionApi.setProxy(proxyUrl.value.trim())
    currentProxy.value = proxyUrl.value.trim()
    notification.success('代理设置已保存', '')
    // Reload available versions with new proxy
    availablePage.value = 1
    availableVersions.value = []
    loadAvailableVersions()
  } catch (error: any) {
    notification.error('保存失败', error?.response?.data?.message || '')
  } finally {
    proxySaving.value = false
  }
}

async function loadDownloadedVersions() {
  downloadedLoading.value = true
  try {
    const response = await versionApi.getDownloaded()
    downloadedVersions.value = response.data
  } catch {
    notification.error('加载已下载版本失败', '')
  } finally {
    downloadedLoading.value = false
  }
}

async function loadAvailableVersions() {
  availableLoading.value = true
  try {
    const response = await versionApi.getAvailable(availablePage.value, perPage)
    const data = response.data
    if (availablePage.value === 1) {
      availableVersions.value = data.versions
    } else {
      availableVersions.value = [...availableVersions.value, ...data.versions]
    }
    totalAvailable.value = data.total
    hasMore.value = data.has_more
  } catch {
    notification.error('加载可用版本失败', '请检查网络连接或配置 GitHub 代理')
  } finally {
    availableLoading.value = false
    loadingMore.value = false
  }
}

async function loadMoreVersions() {
  loadingMore.value = true
  availablePage.value++
  await loadAvailableVersions()
}

async function handleDownloadVersion(version: VersionInfo) {
  downloadingVersion.value = version.version
  try {
    await versionApi.download(version.tag_name, version.download_url)
    notification.success('版本已下载', `${version.name} 下载完成`)
    loadDownloadedVersions()
    // Update the downloaded status in the list
    const found = availableVersions.value.find(v => v.version === version.version)
    if (found) found.downloaded = true
  } catch (error: any) {
    notification.error('下载失败', error?.response?.data?.message || '请检查网络或代理设置')
  } finally {
    downloadingVersion.value = null
  }
}

async function handleDeleteVersion(version: string) {
  try {
    await versionApi.delete(version)
    notification.success('版本已删除', '')
    loadDownloadedVersions()
    // Update the available list too
    const found = availableVersions.value.find(v => v.version === version)
    if (found) found.downloaded = false
  } catch (error: any) {
    notification.error('删除失败', error?.response?.data?.message || '')
  }
}

onMounted(() => {
  loadProxy()
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

/* Proxy Section */
.proxy-section {
  background-color: var(--bg-card);
  border: 1px solid var(--border-color);
  border-radius: 12px;
  overflow: hidden;
  transition: background-color 0.3s, border-color 0.3s;
}

.proxy-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 14px 20px;
  cursor: pointer;
  user-select: none;
}

.proxy-header:hover {
  background-color: var(--bg-body);
}

.proxy-title {
  color: var(--text-primary);
  font-size: 14px;
  font-weight: 500;
}

.proxy-toggle {
  color: var(--color-primary);
  font-size: 12px;
}

.proxy-body {
  padding: 0 20px 20px;
}

.proxy-hint {
  color: var(--text-secondary);
  font-size: 12px;
  margin: 0 0 12px 0;
  line-height: 1.6;
}

.proxy-hint code {
  background-color: var(--bg-body);
  padding: 2px 6px;
  border-radius: 4px;
  font-size: 11px;
  color: var(--color-primary);
}

.proxy-form {
  display: flex;
  gap: 12px;
  align-items: center;
}

.proxy-form .n-input {
  flex: 1;
}

.proxy-current {
  margin-top: 10px;
  font-size: 12px;
  color: var(--text-muted);
}

.proxy-current code {
  color: var(--color-primary);
  background-color: var(--bg-body);
  padding: 2px 6px;
  border-radius: 4px;
}

/* Version Grid */
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

.section-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 16px;
}

.section-header h2 {
  margin: 0;
  color: var(--text-primary);
  font-size: 16px;
}

.count-badge {
  background-color: var(--color-primary);
  color: #fff;
  font-size: 11px;
  padding: 1px 8px;
  border-radius: 10px;
  font-weight: 500;
}

.versions-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
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

.version-item:hover {
  border-color: var(--color-primary);
}

.version-info {
  flex: 1;
  min-width: 0;
}

.version-name {
  color: var(--text-primary);
  font-weight: 600;
  margin-bottom: 4px;
  display: flex;
  align-items: center;
  gap: 6px;
  flex-wrap: wrap;
}

.runtime-tag {
  font-size: 10px !important;
}

.version-meta {
  display: flex;
  gap: 12px;
  flex-wrap: wrap;
}

.version-tag {
  color: var(--color-primary);
  font-size: 12px;
  font-family: "JetBrains Mono", monospace;
}

.version-date,
.version-size {
  color: var(--text-muted);
  font-size: 12px;
}

.empty {
  text-align: center;
  color: var(--text-muted);
  padding: 20px;
}

/* Load More */
.load-more {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  padding: 12px 0 4px;
}

.load-more-hint {
  color: var(--text-muted);
  font-size: 11px;
}

@media (max-width: 1024px) {
  .version-grid {
    grid-template-columns: 1fr;
  }
}
</style>
