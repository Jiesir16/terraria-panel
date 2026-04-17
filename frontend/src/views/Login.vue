<template>
  <div class="login-container">
    <canvas ref="starCanvas" class="star-background"></canvas>
    <div class="login-card">
      <div class="logo">🎮</div>
      <h1>泰拉瑞亚开服控制台</h1>
      <p class="subtitle">Terraria Server Console</p>

      <n-form :model="form" :rules="rules" ref="formRef">
        <n-form-item label="用户名" path="username">
          <n-input
            v-model:value="form.username"
            placeholder="输入用户名"
            clearable
            @keyup.enter="handleLogin"
          />
        </n-form-item>

        <n-form-item label="密码" path="password">
          <n-input
            v-model:value="form.password"
            placeholder="输入密码"
            type="password"
            clearable
            @keyup.enter="handleLogin"
          />
        </n-form-item>

        <n-button
          :loading="loading"
          type="primary"
          block
          @click="handleLogin"
        >
          登 录
        </n-button>
      </n-form>

      <div class="default-account">
        <p>默认账号: admin / admin123</p>
      </div>
    </div>

    <n-tooltip trigger="hover" class="theme-toggle-login">
      <template #trigger>
        <n-button
          class="theme-toggle-btn"
          circle
          @click="appStore.toggleTheme"
        >
          {{ appStore.isDark ? '☀️' : '🌙' }}
        </n-button>
      </template>
      {{ appStore.isDark ? '切换到亮色主题' : '切换到暗色主题' }}
    </n-tooltip>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, watch } from 'vue'
import { useRouter } from 'vue-router'
import { NForm, NFormItem, NInput, NButton, NTooltip } from 'naive-ui'
import { useAuthStore } from '../stores/auth'
import { useAppStore } from '../stores/app'
import { useNotification } from '../composables/useNotification'

const router = useRouter()
const authStore = useAuthStore()
const appStore = useAppStore()
const notification = useNotification()

const formRef = ref()
const loading = ref(false)
const starCanvas = ref<HTMLCanvasElement>()

const form = ref({
  username: '',
  password: ''
})

const rules = {
  username: [
    { required: true, message: '请输入用户名', trigger: 'blur' }
  ],
  password: [
    { required: true, message: '请输入密码', trigger: 'blur' }
  ]
}

async function handleLogin() {
  await formRef.value?.validate()

  loading.value = true
  try {
    await authStore.login(form.value.username, form.value.password)
    notification.success('登录成功', '欢迎回来！')
    router.push('/')
  } catch (error: any) {
    notification.error('登录失败', error?.response?.data?.message || '用户名或密码错误')
  } finally {
    loading.value = false
  }
}

function drawStars() {
  if (!starCanvas.value) return
  const canvas = starCanvas.value
  const ctx = canvas.getContext('2d')
  if (!ctx) return

  canvas.width = window.innerWidth
  canvas.height = window.innerHeight

  const bgColor = appStore.isDark ? '#0D1117' : '#E8ECF0'
  const starColor = appStore.isDark
    ? (opacity: number) => `rgba(232, 232, 232, ${opacity})`
    : (opacity: number) => `rgba(56, 158, 92, ${opacity})`

  ctx.fillStyle = bgColor
  ctx.fillRect(0, 0, canvas.width, canvas.height)

  for (let i = 0; i < 100; i++) {
    const x = Math.random() * canvas.width
    const y = Math.random() * canvas.height
    const radius = Math.random() * 1.5
    const opacity = Math.random() * 0.5 + 0.3

    ctx.fillStyle = starColor(opacity)
    ctx.beginPath()
    ctx.arc(x, y, radius, 0, Math.PI * 2)
    ctx.fill()
  }
}

onMounted(() => {
  drawStars()
  window.addEventListener('resize', drawStars)
})

watch(() => appStore.isDark, () => {
  drawStars()
})
</script>

<style scoped>
.login-container {
  width: 100%;
  min-height: 100vh;
  position: relative;
  overflow: hidden;
}

.star-background {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100vh;
}

.login-card {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  width: 360px;
  padding: 40px;
  background-color: var(--bg-card);
  border: 1px solid var(--border-color);
  border-radius: 12px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
  z-index: 10;
  transition: background-color 0.3s, border-color 0.3s, box-shadow 0.3s;
}

.login-card-light {
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.1);
}

.logo {
  text-align: center;
  font-size: 48px;
  margin-bottom: 16px;
}

h1 {
  text-align: center;
  color: var(--color-primary);
  font-size: 24px;
  margin: 0 0 4px 0;
}

.subtitle {
  text-align: center;
  color: var(--text-secondary);
  font-size: 12px;
  margin: 0 0 24px 0;
}

:deep(.n-form) {
  margin-bottom: 16px;
}

:deep(.n-form-item) {
  margin-bottom: 16px;
}

.default-account {
  text-align: center;
  margin-top: 20px;
  padding-top: 20px;
  border-top: 1px solid var(--border-color);
  transition: border-color 0.3s;
}

.default-account p {
  color: var(--text-muted);
  font-size: 12px;
  margin: 0;
}

.theme-toggle-btn {
  position: fixed;
  bottom: 24px;
  right: 24px;
  z-index: 20;
  font-size: 18px;
}
</style>
