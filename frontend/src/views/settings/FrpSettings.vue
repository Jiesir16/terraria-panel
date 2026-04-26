<template>
  <div class="frp-settings">
    <h2>FRP 全局设置</h2>
    <n-spin :show="loading">
      <div class="form">
        <n-checkbox v-model:checked="settings.enabled">启用 FRP</n-checkbox>
        <n-input v-model:value="settings.frpc_bin" placeholder="frpc 可执行文件路径，例如 frpc 或 /usr/local/bin/frpc" />
        <n-input v-model:value="settings.server_addr" placeholder="frps 地址" />
        <n-input-number v-model:value="settings.server_port" :min="1" :max="65535" style="width: 100%;" placeholder="frps 端口" />
        <n-input v-model:value="settings.auth_token" type="password" show-password-on="click" placeholder="FRP token" />
        <n-checkbox v-model:checked="settings.tls_enable">启用 TLS</n-checkbox>
        <n-checkbox v-model:checked="settings.panel_tunnel.enabled">启用面板穿透</n-checkbox>
        <n-input-number v-model:value="settings.panel_tunnel.local_port" :min="1" :max="65535" style="width: 100%;" placeholder="面板本地端口" />
        <n-input-number v-model:value="settings.panel_tunnel.remote_port" :min="1" :max="65535" style="width: 100%;" placeholder="面板远端端口" />
        <n-input v-model:value="settings.panel_tunnel.proxy_name" placeholder="面板 proxy name" />
        <div class="info-list" style="margin: 4px 0 8px 0;">
          <div class="info-item">
            <span class="label">面板 FRP 状态:</span>
            <span class="value">{{ panelStatus?.running ? '运行中' : '未运行' }}</span>
          </div>
          <div class="info-item">
            <span class="label">远端端口:</span>
            <span class="value">{{ panelStatus?.remote_port ?? '-' }}</span>
          </div>
          <div class="info-item">
            <span class="label">最近错误:</span>
            <span class="value">{{ panelStatus?.last_error || '-' }}</span>
          </div>
        </div>
        <div style="display: flex; gap: 8px; flex-wrap: wrap;">
          <n-button v-if="authStore.user?.role === 'admin'" type="primary" :loading="saving" @click="handleSave">
            保存 FRP 设置
          </n-button>
          <n-button v-if="authStore.user?.role === 'admin'" :loading="restarting" @click="handleRestart">
            重启面板 FRP
          </n-button>
        </div>
      </div>
    </n-spin>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { NSpin, NCheckbox, NInput, NInputNumber, NButton } from 'naive-ui'
import { systemApi, type FrpSettings, type FrpStatus } from '../../api/system'
import { useAuthStore } from '../../stores/auth'
import { useNotification } from '../../composables/useNotification'

const authStore = useAuthStore()
const notification = useNotification()
const loading = ref(false)
const saving = ref(false)
const restarting = ref(false)

const settings = ref<FrpSettings>({
  enabled: false,
  frpc_bin: 'frpc',
  server_addr: '',
  server_port: 7000,
  auth_token: '',
  transport_protocol: 'tcp',
  tls_enable: false,
  log_level: 'info',
  panel_tunnel: {
    enabled: false,
    local_port: 3000,
    remote_port: 3000,
    proxy_name: 'terraria-panel'
  }
})
const panelStatus = ref<FrpStatus | null>(null)

async function loadSettings() {
  loading.value = true
  try {
    const [settingsResponse, statusResponse] = await Promise.all([
      systemApi.getFrpSettings(),
      systemApi.getPanelFrpStatus()
    ])
    settings.value = settingsResponse.data
    panelStatus.value = statusResponse.data
  } catch {
    notification.error('加载 FRP 设置失败', '')
  } finally {
    loading.value = false
  }
}

async function handleSave() {
  saving.value = true
  try {
    await systemApi.updateFrpSettings(settings.value)
    await loadSettings()
    notification.success('FRP 设置已保存', '全局 FRP 和面板穿透配置已更新')
  } catch (error: any) {
    notification.error('保存 FRP 设置失败', error?.response?.data?.message || '')
  } finally {
    saving.value = false
  }
}

async function handleRestart() {
  restarting.value = true
  try {
    await systemApi.restartPanelFrp()
    await loadSettings()
    notification.success('面板 FRP 已重启', '')
  } catch (error: any) {
    notification.error('重启面板 FRP 失败', error?.response?.data?.message || '')
  } finally {
    restarting.value = false
  }
}

onMounted(() => {
  loadSettings()
})
</script>

<style scoped>
.frp-settings {
  background-color: var(--bg-card);
  border: 1px solid var(--border-color);
  border-radius: 12px;
  padding: 20px;
}

.frp-settings h2 {
  margin: 0 0 16px 0;
  color: var(--text-primary);
  font-size: 18px;
}

.form {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.info-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.info-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 4px 0;
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
