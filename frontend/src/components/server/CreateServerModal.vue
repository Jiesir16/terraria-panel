<template>
  <n-modal
    v-model:show="show"
    title="创建新服务器"
    preset="dialog"
    :on-after-leave="handleCancel"
  >
    <n-form :model="form" :rules="rules" ref="formRef">
      <n-form-item label="服务器名称" path="name">
        <n-input v-model:value="form.name" placeholder="输入服务器名称" />
      </n-form-item>

      <n-form-item label="TShock 版本" path="tshock_version">
        <n-select
          v-model:value="form.tshock_version"
          placeholder="选择版本"
          :options="versionOptions"
          :loading="versionsLoading"
        />
      </n-form-item>

      <n-form-item label="端口" path="port">
        <n-input-number v-model:value="form.port" placeholder="7777" :min="1024" :max="65535" />
      </n-form-item>

      <n-form-item label="最大玩家数" path="max_players">
        <n-input-number v-model:value="form.max_players" placeholder="8" :min="1" :max="255" />
      </n-form-item>

      <n-form-item label="进入密码 (可选)" path="password">
        <n-input v-model:value="form.password" type="password" placeholder="不设置密码请留空" />
      </n-form-item>

      <n-form-item label="世界名称 (可选)" path="world_name">
        <n-input v-model:value="form.world_name" placeholder="世界名称" />
      </n-form-item>

      <n-form-item>
        <n-checkbox v-model:checked="form.auto_start">
          服务器启动时自动启动
        </n-checkbox>
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
import { ref, computed, onMounted } from 'vue'
import { NModal, NForm, NFormItem, NInput, NInputNumber, NSelect, NCheckbox, NButton } from 'naive-ui'
import { versionApi } from '../../api/version'
import { useServersStore } from '../../stores/servers'
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

const serversStore = useServersStore()
const notification = useNotification()

const formRef = ref()
const loading = ref(false)
const versionsLoading = ref(false)
const versions = ref<any[]>([])

const form = ref({
  name: '',
  tshock_version: '',
  port: 7777,
  max_players: 8,
  password: '',
  world_name: '',
  auto_start: false
})

const rules = {
  name: [
    { required: true, message: '请输入服务器名称', trigger: 'blur' }
  ],
  tshock_version: [
    { required: true, message: '请选择 TShock 版本', trigger: 'change' }
  ],
  port: [
    { required: true, type: 'number' as const, message: '请输入端口', trigger: 'change' }
  ]
}

const versionOptions = computed(() =>
  versions.value.map(v => ({
    label: v.name,
    value: v.version
  }))
)

const show = computed({
  get: () => props.show,
  set: (value) => emit('update:show', value)
})

async function loadVersions() {
  versionsLoading.value = true
  try {
    const response = await versionApi.getDownloaded()
    versions.value = response.data
    if (versions.value.length > 0) {
      form.value.tshock_version = versions.value[0].version
    }
  } catch (error) {
    notification.error('加载版本列表失败', '')
  } finally {
    versionsLoading.value = false
  }
}

async function handleCreate() {
  await formRef.value?.validate()

  loading.value = true
  try {
    await serversStore.createServer({
      name: form.value.name,
      tshock_version: form.value.tshock_version,
      port: form.value.port,
      max_players: form.value.max_players,
      password: form.value.password || undefined,
      world_name: form.value.world_name || undefined,
      auto_start: form.value.auto_start
    })
    notification.success('服务器已创建', '')
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
    name: '',
    tshock_version: '',
    port: 7777,
    max_players: 8,
    password: '',
    world_name: '',
    auto_start: false
  }
}

onMounted(() => {
  loadVersions()
})
</script>

<style scoped>
:deep(.n-modal) {
  --n-dialog-title-font-size: 18px;
}

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
