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
          <div class="security-section">
            <div class="section-header">
              <h3>TShock 用户与组</h3>
              <n-button text type="primary" @click="loadTshockSecurity" :loading="securityLoading">
                刷新
              </n-button>
            </div>

            <n-spin :show="securityLoading">
              <div class="security-summary" v-if="tshockSecurity">
                <n-alert :type="tshockSecurity.ssc_enabled ? 'success' : 'warning'" :show-icon="false">
                  <div class="summary-line">
                    <strong>SSC 状态：</strong>
                    <span>{{ tshockSecurity.ssc_enabled ? '已启用' : '未启用' }}</span>
                    <n-tag size="small" :type="tshockSecurity.ssc_enabled ? 'success' : 'warning'">
                      {{ tshockSecurity.ssc_source }}
                    </n-tag>
                  </div>
                  <div class="summary-line">
                    <strong>默认注册组：</strong>
                    <span>{{ tshockSecurity.default_registration_group || '未配置' }}</span>
                  </div>
                  <div class="summary-line">
                    <strong>默认游客组：</strong>
                    <span>{{ tshockSecurity.default_guest_group || '未配置' }}</span>
                  </div>
                  <div class="summary-line" v-if="!tshockSecurity.database_exists">
                    <strong>数据库：</strong>
                    <span>TShock 尚未生成 `tshock.sqlite`，先让服务器完整启动一次。</span>
                  </div>
                </n-alert>
              </div>

              <div v-if="tshockSecurity?.database_exists" class="security-grid">
                <div class="security-card">
                  <h4>用户账号</h4>
                  <div v-if="tshockSecurity.users.length === 0" class="empty-note">暂无已注册 TShock 用户</div>
                  <div v-else class="security-list">
                    <div v-for="user in tshockSecurity.users" :key="user.username" class="security-item">
                      <div class="item-main">
                        <strong>{{ user.username }}</strong>
                        <span class="muted">组：{{ user.group_name || '未分组' }}</span>
                      </div>
                      <div class="item-tags">
                        <n-tag v-if="user.is_superadmin" size="small" type="error">superadmin</n-tag>
                        <n-tag v-if="user.ignores_ssc" size="small" type="warning">绕过 SSC</n-tag>
                      </div>
                    </div>
                  </div>
                </div>

                <div class="security-card">
                  <h4>用户组权限</h4>
                  <div v-if="tshockSecurity.groups.length === 0" class="empty-note">暂无可读的 TShock 组信息</div>
                  <div v-else class="security-list">
                    <div v-for="group in tshockSecurity.groups" :key="group.name" class="security-item">
                      <div class="item-main">
                        <strong>{{ group.name }}</strong>
                        <span class="muted">权限数：{{ group.permission_count }}</span>
                      </div>
                      <div class="item-tags">
                        <n-tag v-if="group.is_registration_group" size="small" type="info">默认注册组</n-tag>
                        <n-tag v-if="group.is_guest_group" size="small">默认游客组</n-tag>
                        <n-tag v-if="group.ignores_ssc" size="small" type="warning">含 tshock.ignore.ssc</n-tag>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </n-spin>
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
import { NSpin, NTabs, NTabPane, NButton, NAlert, NTag, useDialog } from 'naive-ui'
import { useAuthStore } from '../stores/auth'
import { useServersStore } from '../stores/servers'
import { serverApi, TShockSecurityOverview } from '../api/server'
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
const tshockSecurity = ref<TShockSecurityOverview | null>(null)
const securityLoading = ref(false)
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
  loadTshockSecurity()
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

async function loadTshockSecurity() {
  if (!authStore.isOperator) {
    return
  }

  securityLoading.value = true
  try {
    const response = await serverApi.getTshockSecurity(serverId.value)
    tshockSecurity.value = response.data
  } catch (error: any) {
    notification.error('加载 TShock 权限信息失败', error?.response?.data?.error || '')
  } finally {
    securityLoading.value = false
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
  loadTshockSecurity()
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
.saves-section,
.security-section {
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

.security-summary {
  margin-bottom: 16px;
}

.summary-line {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
  margin-bottom: 6px;
}

.summary-line:last-child {
  margin-bottom: 0;
}

.security-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
  gap: 16px;
}

.security-card {
  border: 1px solid var(--border-color);
  border-radius: 10px;
  padding: 16px;
  background: var(--bg-body);
}

.security-card h4 {
  margin: 0 0 12px 0;
  color: var(--text-primary);
}

.security-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.security-item {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  padding: 10px 12px;
  border-radius: 8px;
  background: var(--bg-card);
  border: 1px solid var(--border-color);
}

.item-main {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.item-tags {
  display: flex;
  flex-wrap: wrap;
  justify-content: flex-end;
  align-items: flex-start;
  gap: 6px;
}

.muted,
.empty-note {
  color: var(--text-muted);
  font-size: 13px;
}
</style>
