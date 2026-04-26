<template>
  <div class="server-detail">
    <div class="detail-header">
      <div>
        <h1>{{ currentServer?.name }}</h1>
        <div style="display: flex; gap: 8px; align-items: center; flex-wrap: wrap;">
          <server-status-badge :status="currentServer?.status || 'stopped'" />
          <n-button v-if="authStore.isOperator" size="tiny" secondary @click="handleRestartFrp" :loading="frpRestartLoading">
            {{ currentServer?.frp?.running ? '重连 FRP' : '启动 FRP' }}
          </n-button>
        </div>
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
        <n-button v-if="authStore.isAdmin" type="error" @click="handleDeleteServer">
          删除服务器
        </n-button>
      </div>
    </div>

    <n-spin :show="loading">
      <n-tabs type="line">
        <n-tab-pane name="console" tab="控制台">
          <server-console :key="serverId" :server-id="serverId" />
        </n-tab-pane>

        <n-tab-pane name="config" tab="配置">
          <server-config-form :key="serverId" :server-id="serverId" />
        </n-tab-pane>

        <n-tab-pane v-if="authStore.isOperator" name="security" tab="TShock权限">
          <TShockManager :key="serverId" :server-id="serverId" ref="tshockManagerRef" />
        </n-tab-pane>

        <n-tab-pane v-if="authStore.isOperator" name="rest" tab="实时管理">
          <TShockRestPanel :key="serverId" :server-id="serverId" ref="tshockRestRef" />
        </n-tab-pane>

        <n-tab-pane v-if="authStore.isOperator" name="commands" tab="命令库">
          <TShockCommandLibrary :key="serverId" :server-id="serverId" />
        </n-tab-pane>

        <n-tab-pane v-if="authStore.isOperator" name="frp" tab="FRP">
          <div class="mods-section">
            <div class="section-header">
              <h3>FRP 运行状态</h3>
              <n-button text type="primary" @click="handleRefresh">刷新</n-button>
            </div>
            <div class="info-list">
              <div class="info-item">
                <span class="label">运行状态</span>
                <span class="value">{{ currentServer?.frp?.running ? '运行中' : '未运行' }}</span>
              </div>
              <div class="info-item">
                <span class="label">远端端口</span>
                <span class="value">{{ currentServer?.frp?.remote_port ?? '-' }}</span>
              </div>
              <div class="info-item">
                <span class="label">最近错误</span>
                <span class="value">{{ currentServer?.frp?.last_error || '-' }}</span>
              </div>
            </div>
            <div style="margin-top: 16px;">
              <n-button v-if="authStore.isOperator" @click="handleRestartFrp" :loading="frpRestartLoading">
                重连 FRP
              </n-button>
            </div>
          </div>
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
              :key="serverId"
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
              <div style="display:flex; gap:8px; flex-wrap:wrap;">
                <n-button v-if="authStore.isOperator && !includeOtherSaves" text type="info" @click="loadOtherSaves">
                  加载其他服务器存档
                </n-button>
                <n-button v-if="authStore.isOperator" text type="primary" @click="handleBackup">
                  + 手动备份
                </n-button>
              </div>
            </div>
            <p class="hint-text">
              默认只展示本服务备份和手动导入存档；需要跨服导入时再加载其他服务器存档。
            </p>
            <n-spin :show="savesLoading">
              <div v-if="saves.length === 0" class="empty-note">暂无可用存档</div>
              <div v-else class="save-category-list">
                <div v-if="categorizedServerSaves.manual.length > 0" class="save-category">
                  <h4>手动导入</h4>
                  <div class="saves-list">
                    <save-card
                      v-for="save in categorizedServerSaves.manual"
                      :key="save.id"
                      :save="save"
                      :server-id="serverId"
                      :can-manage="authStore.isOperator"
                      @import="() => confirmImportSave(save)"
                      @delete="() => handleDeleteSave(save.id)"
                      @download="() => handleDownloadSave(save)"
                    />
                  </div>
                </div>
                <div v-for="group in categorizedServerSaves.servers" :key="group.id" class="save-category">
                  <h4>{{ group.name }}</h4>
                  <div class="saves-list">
                    <save-card
                      v-for="save in group.saves"
                      :key="save.id"
                      :save="save"
                      :server-id="serverId"
                      :can-manage="authStore.isOperator"
                      @import="() => confirmImportSave(save)"
                      @delete="() => handleDeleteSave(save.id)"
                      @download="() => handleDownloadSave(save)"
                    />
                  </div>
                </div>
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
import { modsApi } from '../api/mods'
import { savesApi } from '../api/saves'
import { serverApi } from '../api/server'
import { useNotification } from '../composables/useNotification'
import ServerConsole from '../components/server/ServerConsole.vue'
import ServerConfigForm from '../components/server/ServerConfigForm.vue'
import ServerStatusBadge from '../components/server/ServerStatusBadge.vue'
import TShockManager from '../components/server/TShockManager.vue'
import TShockCommandLibrary from '../components/server/TShockCommandLibrary.vue'
import TShockRestPanel from '../components/server/TShockRestPanel.vue'
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
const includeOtherSaves = ref(false)
const showModUpload = ref(false)
const mods = ref<any[]>([])
const saves = ref<any[]>([])
const tshockManagerRef = ref<InstanceType<typeof TShockManager> | null>(null)
const tshockRestRef = ref<InstanceType<typeof TShockRestPanel> | null>(null)
const frpRestartLoading = ref(false)
let statusPollTimer: ReturnType<typeof setInterval> | null = null
const serverMissingHandled = ref(false)

const currentServer = computed(() => serversStore.currentServer)

const isCurrentServerActive = computed(() => currentServer.value?.status !== 'stopped')

const categorizedServerSaves = computed(() => {
  const manual: any[] = []
  const groups = new Map<string, { id: string; name: string; saves: any[] }>()
  const currentName = currentServer.value?.name || '本服务备份'

  for (const save of saves.value) {
    if (!save.source_server_id) {
      manual.push(save)
      continue
    }

    const id = save.source_server_id
    const name = id === serverId.value ? currentName : (save.source_server_name || `服务器 ${id}`)
    if (!groups.has(id)) {
      groups.set(id, { id, name, saves: [] })
    }
    groups.get(id)!.saves.push(save)
  }

  const servers = Array.from(groups.values()).sort((a, b) => {
    if (a.id === serverId.value) return -1
    if (b.id === serverId.value) return 1
    return a.name.localeCompare(b.name)
  })

  return { manual, servers }
})

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
watch(serverId, (newId) => {
  if (!newId || route.name !== 'ServerDetail') return
  
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
    const response = await savesApi.getList({
      server_id: serverId.value,
      include_other_servers: includeOtherSaves.value,
    })
    saves.value = response.data
  } catch (error) {
    notification.error('加载存档失败', '')
  } finally {
    savesLoading.value = false
  }
}

async function loadOtherSaves() {
  includeOtherSaves.value = true
  await loadSaves()
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
    onPositiveClick: () => {
      ;(async () => {
        startLoading.value = true
        loading.value = true
        try {
          const result = await serversStore.startServer(serverId.value)
          notification.success('启动请求已发送', result?.message || '服务器正在启动中...')
          await loadServer()
        } catch (error: any) {
          notification.error('启动失败', error?.response?.data?.error || '请检查服务器配置和日志')
        } finally {
          startLoading.value = false
          loading.value = false
        }
      })()
    }
  })
}

function handleStop() {
  dialog.warning({
    title: '确认停止',
    content: `确定要停止服务器「${currentServer.value?.name || serverId.value}」吗？正在游戏中的玩家将被断开连接。`,
    positiveText: '停止',
    negativeText: '取消',
    onPositiveClick: () => {
      ;(async () => {
        stopLoading.value = true
        loading.value = true
        try {
          await serversStore.stopServer(serverId.value)
          notification.success('服务器已停止', '服务器已安全关闭')
          await loadServer()
        } catch (error: any) {
          notification.error('停止失败', error?.response?.data?.error || '')
        } finally {
          stopLoading.value = false
          loading.value = false
        }
      })()
    }
  })
}

function handleKill() {
  dialog.error({
    title: '确认强制结束',
    content: `确定要强制结束服务器「${currentServer.value?.name || serverId.value}」吗？这可能导致未保存的数据丢失！建议先尝试正常停止。`,
    positiveText: '强制结束',
    negativeText: '取消',
    onPositiveClick: () => {
      ;(async () => {
        killLoading.value = true
        loading.value = true
        try {
          const result = await serversStore.killServer(serverId.value)
          notification.success('强制结束信号已发送', result?.message || '进程已被终止')
          await loadServer()
        } catch (error: any) {
          notification.error('强制结束失败', error?.response?.data?.error || '')
        } finally {
          killLoading.value = false
          loading.value = false
        }
      })()
    }
  })
}

function handleRestart() {
  dialog.warning({
    title: '确认重启',
    content: `确定要重启服务器「${currentServer.value?.name || serverId.value}」吗？正在游戏中的玩家将被短暂断开。`,
    positiveText: '重启',
    negativeText: '取消',
    onPositiveClick: () => {
      ;(async () => {
        restartLoading.value = true
        loading.value = true
        try {
          await serversStore.restartServer(serverId.value)
          notification.success('服务器已重启', '服务器正在重新启动中...')
          await loadServer()
        } catch (error: any) {
          notification.error('重启失败', error?.response?.data?.error || '')
        } finally {
          restartLoading.value = false
          loading.value = false
        }
      })()
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

function confirmImportSave(save: any) {
  if (save.source_type === 'server_archive' || save.name?.endsWith('.zip')) {
    notification.warning('归档包不能直接导入', '请先下载并解压，再导入其中的 .wld 文件')
    return
  }

  const source = save.source_server_id
    ? (save.source_server_id === serverId.value ? '本服务备份' : `其他服务器备份：${save.source_server_name || save.source_server_id}`)
    : '手动导入存档'

  dialog.warning({
    title: '确认导入存档',
    content: `导入会覆盖当前服务器启动世界并把「${save.name}」设为启动存档。来源：${source}。建议先确认当前服务器已停止或已完成备份。`,
    positiveText: '导入',
    negativeText: '取消',
    onPositiveClick: () => handleImportSave(save.id),
  })
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

async function handleRestartFrp() {
  frpRestartLoading.value = true
  try {
    await serverApi.restartFrp(serverId.value)
    notification.success('FRP 已重连', '')
    await loadServer()
  } catch (error: any) {
    notification.error('FRP 重连失败', error?.response?.data?.message || '')
  } finally {
    frpRestartLoading.value = false
  }
}

function handleDeleteServer() {
  const serverName = currentServer.value?.name || serverId.value

  const runDelete = async (backupMode: 'keep' | 'delete') => {
    try {
      const result = await serversStore.deleteServer(serverId.value, backupMode)
      notification.success(
        backupMode === 'delete' ? '服务器和相关备份已删除' : '服务器已删除，备份已保留',
        result?.deleted_backup_count ? `已删除 ${result.deleted_backup_count} 个备份文件` : ''
      )
      router.replace('/servers')
    } catch (error: any) {
      notification.error('删除失败', error?.response?.data?.message || error?.response?.data?.error || '')
    }
  }

  dialog.warning({
    title: '删除服务器并保留备份',
    content: `确定要删除服务器「${serverName}」吗？服务器配置和运行数据会被移除，但现有备份会保留，后续仍可在存档管理中单独删除。`,
    positiveText: '删除服务器，保留备份',
    negativeText: '取消',
    onPositiveClick: async () => {
      await runDelete('keep')
    },
    onNegativeClick: () => {
      dialog.error({
        title: '删除服务器和相关备份',
        content: `要把服务器「${serverName}」的相关备份也一起删除吗？这会同时删除该服务器生成的备份记录和磁盘文件，且不可恢复。`,
        positiveText: '删除服务器和备份',
        negativeText: '返回',
        onPositiveClick: async () => {
          await runDelete('delete')
        }
      })
    }
  })
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
  gap: 8px;
  flex-wrap: wrap;
  justify-content: flex-end;
}

@media (max-width: 768px) {
  .detail-header {
    flex-direction: column;
    align-items: flex-start;
    gap: 12px;
    padding: 16px;
  }

  .header-actions {
    width: 100%;
    justify-content: flex-start;
  }
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

.hint-text {
  margin: -8px 0 14px;
  color: var(--text-muted);
  font-size: 13px;
}

.save-category-list {
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.save-category h4 {
  margin: 0 0 10px;
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
