<template>
  <div class="server-detail">
    <div class="detail-header">
      <div>
        <h1>{{ currentServer?.name }}</h1>
        <server-status-badge :status="currentServer?.status || 'stopped'" />
      </div>
      <div class="header-actions">
        <n-button @click="handleRefresh">
          刷新状态
        </n-button>
        <n-button v-if="authStore.isOperator" type="warning" :loading="killLoading" @click="handleKill">
          强制结束
        </n-button>
        <n-button
          v-if="authStore.isOperator && isCurrentServerActive"
          type="error"
          :loading="stopLoading"
          @click="handleStop"
        >
          停止服务器
        </n-button>
        <n-button
          v-else-if="authStore.isOperator"
          type="success"
          :loading="startLoading"
          @click="handleStart"
        >
          启动服务器
        </n-button>
        <n-button v-if="authStore.isOperator" @click="handleRestart" type="warning" :loading="restartLoading">
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

        <n-tab-pane v-if="authStore.isOperator" name="security" tab="TShock权限">
          <tshock-manager :server-id="serverId" ref="tshockManagerRef" />
        </n-tab-pane>

        <n-tab-pane name="mods" tab="Mod管理">
          <div class="mods-section">
            <div class="section-header">
              <h3>已安装 Mod</h3>
              <n-button v-if="authStore.isOperator" text type="primary" @click="showModUpload = true">
                + 上传 Mod
              </n-button>
            </div>
            <n-spin :show="modsLoading">
              <div class="mods-list">
                <mod-card
                  v-for="mod in mods"
                  :key="mod.name"
                  :mod="mod"
                  :can-manage="authStore.isOperator"
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
              <n-button v-if="authStore.isOperator" text type="primary" @click="handleBackup">
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
                  :can-manage="authStore.isOperator"
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
import { ref, computed, onMounted, onUnmounted, onActivated, onDeactivated, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { NSpin, NTabs, NTabPane, NButton, useDialog } from 'naive-ui'
import { useAuthStore } from '../stores/auth'
import { useServersStore } from '../stores/servers'
import { serverApi } from '../api/server'
import { modsApi } from '../api/mods'
import { savesApi } from '../api/saves'
import { useNotification } from '../composables/useNotification'
import ServerConsole from '../components/server/ServerConsole.vue'
import ServerConfigForm from '../components/server/ServerConfigForm.vue'
import ServerStatusBadge from '../components/server/ServerStatusBadge.vue'
import TShockManager from '../components/server/TShockManager.vue'
import ModCard from '../components/mod/ModCard.vue'
import ModUploadModal from '../components/mod/ModUploadModal.vue'
import SaveCard from '../components/save/SaveCard.vue'

const route = useRoute()
const router = useRouter()
const authStore = useAuthStore()
const serversStore = useServersStore()
const notification = useNotification()
const dialog = useDialog()

const serverId = computed(() => route.params.id as string)
const loading = ref(false)
const modsLoading = ref(false)
const savesLoading = ref(false)
const showModUpload = ref(false)
const mods = ref<any[]>([])
const saves = ref<any[]>([])
const tshockManagerRef = ref<InstanceType<typeof TShockManager> | null>(null)
let statusPollTimer: ReturnType<typeof setInterval> | null = null
const serverMissingHandled = ref(false)

const currentServer = computed(() => serversStore.currentServer)

const isCurrentServerActive = computed(() => currentServer.value?.status !== 'stopped')

function clearStatusPoll() {
  if (statusPollTimer) {
    clearInterval(statusPollTimer)
    statusPollTimer = null
  }
}

function setupStatusPoll() {
  clearStatusPoll()
  statusPollTimer = setInterval(() => {
    serversStore.refreshServerRuntime(serverId.value).catch((error: any) => {
      if (error?.response?.status === 404) {
        handleMissingServer()
      }
    })
  }, 5000)
}

function handleMissingServer() {
  clearStatusPoll()
  if (serverMissingHandled.value) {
    return
  }
  serverMissingHandled.value = true
  notification.error('服务器不存在', '当前服务器可能已被删除，已返回服务器列表')
  router.replace('/servers')
}

// Watch for route param changes (e.g. navigating from server 1 to server 2)
watch(serverId, () => {
  serverMissingHandled.value = false
  loadServer()
  loadMods()
  loadSaves()
  setupStatusPoll()
})

async function loadServer() {
  loading.value = true
  try {
    await serversStore.refreshServerRuntime(serverId.value)
  } catch (error: any) {
    if (error?.response?.status === 404) {
      handleMissingServer()
      return
    }
    notification.error('加载服务器失败', '')
  } finally {
    loading.value = false
  }
}

async function handleRefresh() {
  await loadServer()
  notification.success('状态已刷新', '')
}

async function loadMods() {
  modsLoading.value = true
  try {
    const response = await modsApi.getList(serverId.value)
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

const startLoading = ref(false)
const stopLoading = ref(false)
const killLoading = ref(false)
const restartLoading = ref(false)

function handleStart() {
  dialog.warning({
    title: '确认启动',
    content: `确定要启动服务器「${currentServer.value?.name || serverId.value}」吗？`,
    positiveText: '启动',
    negativeText: '取消',
    onPositiveClick: async () => {
      startLoading.value = true
      try {
        const result = await serversStore.startServer(serverId.value)
        notification.success('启动请求已发送', result?.message || '服务器正在启动中...')
        await loadServer()
      } catch (error: any) {
        notification.error('启动失败', error?.response?.data?.error || '请检查服务器配置和日志')
      } finally {
        startLoading.value = false
      }
    }
  })
}

function handleStop() {
  dialog.warning({
    title: '确认停止',
    content: `确定要停止服务器「${currentServer.value?.name || serverId.value}」吗？正在游戏中的玩家将被断开连接。`,
    positiveText: '停止',
    negativeText: '取消',
    onPositiveClick: async () => {
      stopLoading.value = true
      try {
        await serversStore.stopServer(serverId.value)
        notification.success('服务器已停止', '服务器已安全关闭')
        await loadServer()
      } catch (error: any) {
        notification.error('停止失败', error?.response?.data?.error || '')
      } finally {
        stopLoading.value = false
      }
    }
  })
}

function handleKill() {
  dialog.error({
    title: '确认强制结束',
    content: `确定要强制结束服务器「${currentServer.value?.name || serverId.value}」吗？这可能导致未保存的数据丢失！建议先尝试正常停止。`,
    positiveText: '强制结束',
    negativeText: '取消',
    onPositiveClick: async () => {
      killLoading.value = true
      try {
        const result = await serversStore.killServer(serverId.value)
        notification.success('强制结束信号已发送', result?.message || '进程已被终止')
        await loadServer()
      } catch (error: any) {
        notification.error('强制结束失败', error?.response?.data?.error || '')
      } finally {
        killLoading.value = false
      }
    }
  })
}

function handleRestart() {
  dialog.warning({
    title: '确认重启',
    content: `确定要重启服务器「${currentServer.value?.name || serverId.value}」吗？正在游戏中的玩家将被短暂断开。`,
    positiveText: '重启',
    negativeText: '取消',
    onPositiveClick: async () => {
      restartLoading.value = true
      try {
        await serversStore.restartServer(serverId.value)
        notification.success('服务器已重启', '服务器正在重新启动中...')
        await loadServer()
      } catch (error: any) {
        notification.error('重启失败', error?.response?.data?.error || '')
      } finally {
        restartLoading.value = false
      }
    }
  })
}

async function handleToggleMod(modName: string) {
  try {
    await modsApi.toggle(serverId.value, modName)
    notification.success('Mod 状态已切换', '')
    loadMods()
  } catch (error: any) {
    notification.error('切换失败', error?.response?.data?.message || '')
  }
}

async function handleDeleteMod(modName: string) {
  try {
    await modsApi.delete(serverId.value, modName)
    notification.success('Mod 已删除', '')
    loadMods()
  } catch (error: any) {
    notification.error('删除失败', error?.response?.data?.message || '')
  }
}

async function handleImportSave(saveId: string) {
  try {
    await savesApi.importToServer(saveId, serverId.value)
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
    await savesApi.backup(serverId.value)
    notification.success('备份已创建', '')
    loadSaves()
  } catch (error: any) {
    notification.error('备份失败', error?.response?.data?.message || '')
  }
}

onMounted(() => {
  serverMissingHandled.value = false
  loadServer()
  loadMods()
  loadSaves()
  setupStatusPoll()
})

onActivated(() => {
  serverMissingHandled.value = false
  setupStatusPoll()
})

onDeactivated(() => {
  clearStatusPoll()
})

onUnmounted(() => {
  clearStatusPoll()
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

.muted,
.empty-note {
  color: var(--text-muted);
  font-size: 13px;
}
</style>
