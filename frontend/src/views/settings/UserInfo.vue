<template>
  <div class="user-info">
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
      />
      <n-input
        v-model:value="passwordForm.newPassword"
        type="password"
        show-password-on="click"
        placeholder="新密码"
      />
      <n-input
        v-model:value="passwordForm.confirmPassword"
        type="password"
        show-password-on="click"
        placeholder="确认新密码"
      />
      <n-button
        type="primary"
        :loading="loading"
        @click="handleChangePassword"
        block
      >
        确认修改
      </n-button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { NTag, NDivider, NInput, NButton } from 'naive-ui'
import { useAuthStore } from '../../stores/auth'
import { useNotification } from '../../composables/useNotification'

const authStore = useAuthStore()
const notification = useNotification()
const loading = ref(false)

const passwordForm = ref({
  oldPassword: '',
  newPassword: '',
  confirmPassword: ''
})

async function handleChangePassword() {
  if (passwordForm.value.newPassword !== passwordForm.value.confirmPassword) {
    notification.error('密码不匹配', '新密码和确认密码不一致')
    return
  }

  if (passwordForm.value.newPassword.length < 6) {
    notification.error('密码过短', '密码长度至少为 6 个字符')
    return
  }

  loading.value = true
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
    loading.value = false
  }
}
</script>

<style scoped>
.user-info {
  background-color: var(--bg-card);
  border: 1px solid var(--border-color);
  border-radius: 12px;
  padding: 20px;
}

.user-info h2 {
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

.password-form {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
</style>
