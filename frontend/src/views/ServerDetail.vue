<template>
  <div class="server-detail">
    <div class="detail-header">
      <div>
        <h1>{{ currentServer?.name }}</h1>
        <server-status-badge :status="currentServer?.status || 'stopped'" />
      </div>
      <div class="header-actions">
        <n-button
          v-if="currentServer?.status === 'running'"
          type="error"
          @click="handleStop"
        >
          停止服务器
        </n-button>
        <n-button
          v-else
          type="success"
          @click="handleStart"
        >
          启动服务器
        </n-button>
        <n-button @click="handleRestart" type="warning">
          重启服务器
        </n-button>
      </div>
    </div>

    <n-spin :show="loading">
      <n-tabs type="line">
        <n-tab-pane name="console" tab="控制台">
          <server-console :server-id="serverId" />
        </n-tab-pane>

        <n-tab-pane name="config" tab="配置">
          <server-config-form :server-id="serverId" />
        </n-tab-pane>

        <n-tab-pane name="mods" tab="Mod管理">
          <div class="mods-section">
            <div class="section-header">
              <h3>已安装 Mod</h3>
              <n-button text type="primary" @click="showModUpload = true">
                + 上传 Mod
              </n-button>
            </div>
            <n-spin :show="modsLoading">
              <div class="mods-list">
                <mod-card
                  v-for="mod in mods"
                  :key="mod.name"
                  :mod="mod"
                  @toggle="() => handleToggleMod(mod.name)"
                  @delete="() => handleDeleteMod(mod.name)"
                />
              </div>
            </n-spin>
            <mod-upload-modal
              v-model:show="showModUpload"
              :server-id="serverId"
              @uploaded="loadMods"
            />
          </div>
        </n-tab-pane>

        <n-tab-pane name="saves" tab="存档">
          <div class="saves-section">
            <div class="section-header">
              <h3>世界存档</h3>
              <n-button text type="primary" @click="handleBackup">
                + 手动备份
              </n-button>
            </div>
            <n-spin :show="savesLoading">
              <div class="saves-list">
                <save-card
                  v-for="save in saves"
                  :key="save.id"
                  :save="save"
                  :server-id="serverId"
                  @import="() => handleImportSave(save.id)"
                  @delete="() => handleDeleteSave(save.id)"
                  @download="() => handleDownloadSave(save)"
                />
              </div>
            </n-spin>
          </div>
        </n-tab-pane>
      </n-tabs>
    </n-spin>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useRoute } from 'vue-router'
import { NSpin, NTabs, NTabPane, NButton } from 'naive-ui'
import { useServersStore } from '../stores/servers'
import { modsApi } from '../api/mods'
import { savesApi } from '../api/saves'
import { useNotification } from '../composables/useNotification'
import ServerConsole from '../components/server/ServerConsole.vue'
import ServerConfigForm from '../components/server/ServerConfigForm.vue'
import ServerStatusBadge from '../components/server/ServerStatusBadge.vue'
import ModCard from '../components/mod/ModCard.vue'
import ModUploadModal from '../components/mod/ModUploadModal.vue'
import SaveCard from '../components/save/SaveCard.vue'

const route = useRoute()
const serversStore = useServersStore()
const notification = useNotification()

const serverId = route.params.id as string
const loading = ref(false)
const modsLoading = ref(false)
const savesLoading = ref(false)
const showModUpload = ref(false)
const mods = ref<any[]>([])
const saves = ref<any[]>([])
let statusPollTimer: ReturnType<typeof setInterval> | null = null

const currentServer = computed(() => serversStore.currentServer)

async function loadServer() {
  loading.value = true
  try {
    await serversStore.fetchServer(serverId)
  } catch (error) {
    notification.error('加载服务器失败', '')
  } finally {
    loading.value = false
  }
}

async function loadMods() {
  modsLoading.value = true
  try {
    const response = await modsApi.getList(serverId)
    mods.value = response.data
  } catch (error) {
    notification.error('加载 Mod 失败', '')
  } finally {
    modsLoading.value = false
  }
}

async function loadSaves() {
  savesLoading.value = true
  try {
    const response = await savesApi.getList()
    saves.value = response.data
  } catch (error) {
    notification.error('加载存档失败', '')
  } finally {
    savesLoading.value = false
  }
}

async function handleStart() {
  try {
    await serversStore.startServer(serverId)
    notification.success('服务器已启动', '')
    await loadServer()
  } catch (error: any) {
    notification.error('启动失败', error?.response?.data?.message || '')
  }
}

async function handleStop() {
  try {
    await serversStore.stopServer(serverId)
    notification.success('服务器已停止', '')
    await loadServer()
  } catch (error: any) {
    notification.error('停止失败', error?.response?.data?.message || '')
  }
}

async function handleRestart() {
  try {
    await serversStore.restartServer(serverId)
    notification.success('服务器已重启', '')
    await loadServer()
  } catch (error: any) {
    notification.error('重启失败', error?.response?.data?.message || '')
  }
}

async function handleToggleMod(modName: string) {
  try {
    await modsApi.toggle(serverId, modName)
    notification.success('Mod 状态已切换', '')
    loadMods()
  } catch (error: any) {
    notification.error('切换失败', error?.response?.data?.message || '')
  }
}

async function handleDeleteMod(modName: string) {
  try {
    await modsApi.delete(serverId, modName)
    notification.success('Mod 已删除', '')
    loadMods()
  } catch (error: any) {
    notification.error('删除失败', error?.response?.data?.message || '')
  }
}

async function handleImportSave(saveId: string) {
  try {
    await savesApi.importToServer(saveId, serverId)
    notification.success('存档已导入，并设为启动世界', '')
    await loadServer()
    loadSaves()
  } catch (error: any) {
    notification.error('导入失败', error?.response?.data?.message || '')
  }
}

async function handleDownloadSave(save: { id: string; name: string }) {
  try {
    const response = await savesApi.download(save.id)
    const url = window.URL.createObjectURL(response.data)
    const link = document.createElement('a')
    link.href = url
    link.download = save.name
    link.click()
    window.URL.revokeObjectURL(url)
    notification.success('下载开始', '')
  } catch (error: any) {
    notification.error('下载失败', error?.response?.data?.message || '')
  }
}

async function handleDeleteSave(saveId: string) {
  try {
    await savesApi.delete(saveId)
    notification.success('存档已删除', '')
    loadSaves()
  } catch (error: any) {
    notification.error('删除失败', error?.response?.data?.message || '')
  }
}

async function handleBackup() {
  try {
    await savesApi.backup(serverId)
    notification.success('备份已创建', '')
    loadSaves()
  } catch (error: any) {
    notification.error('备份失败', error?.response?.data?.message || '')
  }
}

onMounted(() => {
  loadServer()
  loadMods()
  loadSaves()
  // Poll server status every 5 seconds to keep UI in sync
  statusPollTimer = setInterval(() => {
    serversStore.fetchServer(serverId).catch(() => {})
  }, 5000)
})

onUnmounted(() => {
  if (statusPollTimer) {
    clearInterval(statusPollTimer)
    statusPollTimer = null
  }
})
</script>

<style scoped>
.server-detail {
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.detail-header {
  background-color: var(--bg-card);
  border: 1px solid var(--border-color);
  border-radius: 12px;
  padding: 20px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  transition: background-color 0.3s, border-color 0.3s;
}

.detail-header h1 {
  margin: 0 0 8px 0;
  color: var(--text-primary);
}

.header-actions {
  display: flex;
  gap: 12px;
}

.mods-section,
.saves-section {
  background-color: var(--bg-card);
  border: 1px solid var(--border-color);
  border-radius: 12px;
  padding: 20px;
  transition: background-color 0.3s, border-color 0.3s;
}

.section-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
}

.section-header h3 {
  margin: 0;
  color: var(--text-primary);
}

.mods-list,
.saves-list {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(250px, 1fr));
  gap: 16px;
}
</style>
