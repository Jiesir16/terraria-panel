<template>
  <n-modal
    v-model:show="show"
    title="创建新用户"
    preset="dialog"
    :on-after-leave="handleCancel"
  >
    <n-form :model="form" :rules="rules" ref="formRef">
      <n-form-item label="用户名" path="username">
        <n-input v-model:value="form.username" placeholder="输入用户名" />
      </n-form-item>

      <n-form-item label="密码" path="password">
        <n-input v-model:value="form.password" type="password" placeholder="输入密码" />
      </n-form-item>

      <n-form-item label="确认密码" path="confirmPassword">
        <n-input v-model:value="form.confirmPassword" type="password" placeholder="确认密码" />
      </n-form-item>

      <n-form-item label="角色" path="role">
        <n-select
          v-model:value="form.role"
          :options="[
            { label: '管理员', value: 'admin' },
            { label: '操作员', value: 'operator' },
            { label: '观察者', value: 'viewer' }
          ]"
        />
      </n-form-item>
    </n-form>

    <template #action>
      <n-button @click="handleCancel">取消</n-button>
      <n-button type="primary" :loading="loading" @click="handleCreate">
        创建
      </n-button>
    </template>
  </n-modal>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { NModal, NForm, NFormItem, NInput, NSelect, NButton } from 'naive-ui'
import { systemApi } from '../../api/system'
import { useNotification } from '../../composables/useNotification'

interface Props {
  show: boolean
}

interface Emits {
  (e: 'update:show', value: boolean): void
  (e: 'created'): void
}

const props = defineProps<Props>()
const emit = defineEmits<Emits>()

const notification = useNotification()

const formRef = ref()
const loading = ref(false)

const form = ref({
  username: '',
  password: '',
  confirmPassword: '',
  role: 'operator'
})

const rules = {
  username: [
    { required: true, message: '请输入用户名', trigger: 'blur' },
    { min: 3, message: '用户名至少 3 个字符', trigger: 'blur' }
  ],
  password: [
    { required: true, message: '请输入密码', trigger: 'blur' },
    { min: 6, message: '密码至少 6 个字符', trigger: 'blur' }
  ],
  confirmPassword: [
    { required: true, message: '请确认密码', trigger: 'blur' }
  ],
  role: [
    { required: true, message: '请选择角色', trigger: 'change' }
  ]
}

const show = computed({
  get: () => props.show,
  set: (value) => emit('update:show', value)
})

async function handleCreate() {
  await formRef.value?.validate()

  if (form.value.password !== form.value.confirmPassword) {
    notification.error('密码不匹配', '密码和确认密码不一致')
    return
  }

  loading.value = true
  try {
    await systemApi.createUser({
      username: form.value.username,
      password: form.value.password,
      role: form.value.role as 'admin' | 'operator' | 'viewer'
    })
    notification.success('用户已创建', '')
    emit('created')
    handleCancel()
  } catch (error: any) {
    notification.error('创建失败', error?.response?.data?.message || '')
  } finally {
    loading.value = false
  }
}

function handleCancel() {
  show.value = false
  form.value = {
    username: '',
    password: '',
    confirmPassword: '',
    role: 'operator'
  }
}
</script>

<style scoped>
:deep(.n-form-item) {
  margin-bottom: 16px;
}

:deep(.n-input__input-el),
:deep(.n-select__input-el) {
  background-color: var(--bg-input);
  color: var(--text-primary);
  border-color: var(--border-color);
}
</style>
