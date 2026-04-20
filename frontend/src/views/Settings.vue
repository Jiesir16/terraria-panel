<template>
  <div class="settings">
    <h1>系统设置</h1>

    <div class="settings-grid">
      <div class="setting-card">
        <h2>系统信息</h2>
        <n-spin :show="sysLoading">
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

      <div class="setting-card">
        <h2>用户信息</h2>
        <div class="info-list" style="margin-bottom: 24px;">
          <div class="info-item">
            <span class="label">用户名:</span>
            <span class="value">{{ authStore.user?.username || '-' }}</span>
          </div>
          <div class="info-item">
            <span class="label">角色:</span>
            <span class="value">
              <n-tag :type="authStore.user?.role === 'admin' ? 'error' : authStore.user?.role === 'operator' ? 'warning' : 'info'" size="small" :bordered="false">
                {{ authStore.user?.role === 'admin' ? '管理员' : authStore.user?.role === 'operator' ? '操作员' : '普通用户' }}
              </n-tag>
            </span>
          </div>
        </div>

        <n-divider style="margin: 24px 0;" />

        <h2 style="margin-bottom: 16px;">修改密码</h2>
        <div class="password-form">
          <n-input
            v-model:value="passwordForm.oldPassword"
            type="password"
            show-password-on="click"
            placeholder="旧密码"
            class="password-input"
          />
          <n-input
            v-model:value="passwordForm.newPassword"
            type="password"
            show-password-on="click"
            placeholder="新密码"
            class="password-input"
          />
          <n-input
            v-model:value="passwordForm.confirmPassword"
            type="password"
            show-password-on="click"
            placeholder="确认新密码"
            class="password-input"
          />
          <n-button
            type="primary"
            :loading="passwordLoading"
            @click="handleChangePassword"
            block
          >
            确认修改
          </n-button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { NSpin, NInput, NButton, NTag, NDivider } from 'naive-ui'
import { systemApi } from '../api/system'
import { useAuthStore } from '../stores/auth'
import { useNotification } from '../composables/useNotification'

const authStore = useAuthStore()
const notification = useNotification()

const sysLoading = ref(false)
const passwordLoading = ref(false)
const systemInfo = ref<any>(null)

const passwordForm = ref({
  oldPassword: '',
  newPassword: '',
  confirmPassword: ''
})

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return Math.round(bytes / Math.pow(k, i) * 100) / 100 + ' ' + sizes[i]
}

async function loadSystemInfo() {
  sysLoading.value = true
  try {
    const response = await systemApi.getSystemInfo()
    systemInfo.value = response.data
  } catch (error) {
    notification.error('加载系统信息失败', '')
  } finally {
    sysLoading.value = false
  }
}

async function handleChangePassword() {
  if (passwordForm.value.newPassword !== passwordForm.value.confirmPassword) {
    notification.error('密码不匹配', '新密码和确认密码不一致')
    return
  }

  if (passwordForm.value.newPassword.length < 6) {
    notification.error('密码过短', '密码长度至少为 6 个字符')
    return
  }

  passwordLoading.value = true
  try {
    await authStore.changePassword(
      passwordForm.value.oldPassword,
      passwordForm.value.newPassword
    )
    notification.success('密码已修改', '请使用新密码登录')
    passwordForm.value = {
      oldPassword: '',
      newPassword: '',
      confirmPassword: ''
    }
  } catch (error: any) {
    notification.error('修改失败', error?.response?.data?.message || '旧密码错误')
  } finally {
    passwordLoading.value = false
  }
}

onMounted(() => {
  loadSystemInfo()
})
</script>

<style scoped>
.settings {
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.settings h1 {
  margin: 0;
  color: var(--text-primary);
}

.settings-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
  gap: 20px;
}

.setting-card {
  background-color: var(--bg-card);
  border: 1px solid var(--border-color);
  border-radius: 12px;
  padding: 20px;
  transition: background-color 0.3s, border-color 0.3s;
}

.setting-card h2 {
  margin: 0 0 16px 0;
  color: var(--text-primary);
  font-size: 16px;
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

.password-form {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.password-input {
  width: 100%;
}
</style>
