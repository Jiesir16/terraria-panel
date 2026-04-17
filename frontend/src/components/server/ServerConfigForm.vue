<template>
  <div class="config-form">
    <n-spin :show="loading">
      <n-form :model="formData" :rules="rules" ref="formRef">
        <n-form-item label="服务器名称" path="ServerName">
          <n-input v-model:value="formData.ServerName" placeholder="服务器名称" />
        </n-form-item>

        <n-form-item label="最大玩家数" path="MaxPlayers">
          <n-input-number v-model:value="formData.MaxPlayers" :min="1" :max="255" />
        </n-form-item>

        <n-form-item label="难度" path="Difficulty">
          <n-select
            v-model:value="formData.Difficulty"
            :options="[
              { label: '普通', value: 'normal' },
              { label: '困难', value: 'hard' },
              { label: '专家', value: 'expert' }
            ]"
          />
        </n-form-item>

        <n-form-item label="PVP 模式" path="PvpMode">
          <n-checkbox v-model:checked="formData.PvpMode">启用</n-checkbox>
        </n-form-item>

        <n-form-item label="反作弊" path="AntiCheat">
          <n-checkbox v-model:checked="formData.AntiCheat">启用</n-checkbox>
        </n-form-item>

        <n-form-item label="允许 NPC 之家占用" path="HardcoreOnly">
          <n-checkbox v-model:checked="formData.HardcoreOnly">启用</n-checkbox>
        </n-form-item>

        <n-button type="primary" :loading="saving" @click="handleSave">
          保存配置
        </n-button>
      </n-form>
    </n-spin>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { NSpin, NForm, NFormItem, NInput, NInputNumber, NSelect, NCheckbox, NButton } from 'naive-ui'
import { useServersStore } from '../../stores/servers'
import { useNotification } from '../../composables/useNotification'

interface Props {
  serverId: string
}

const props = defineProps<Props>()
const serversStore = useServersStore()
const notification = useNotification()

const formRef = ref()
const loading = ref(false)
const saving = ref(false)
const formData = ref<any>({
  ServerName: '',
  MaxPlayers: 8,
  Difficulty: 'normal',
  PvpMode: false,
  AntiCheat: true,
  HardcoreOnly: false
})

const rules = {
  ServerName: [
    { required: true, message: '请输入服务器名称', trigger: 'blur' }
  ],
  MaxPlayers: [
    { required: true, type: 'number' as const, message: '请输入最大玩家数', trigger: 'change' }
  ]
}

async function loadConfig() {
  loading.value = true
  try {
    const config = await serversStore.getConfig(props.serverId)
    formData.value = {
      ServerName: config.ServerName || '',
      MaxPlayers: config.MaxPlayers || 8,
      Difficulty: config.Difficulty || 'normal',
      PvpMode: config.PvpMode || false,
      AntiCheat: config.AntiCheat !== false,
      HardcoreOnly: config.HardcoreOnly || false
    }
  } catch (error) {
    notification.error('加载配置失败', '')
  } finally {
    loading.value = false
  }
}

async function handleSave() {
  await formRef.value?.validate()

  saving.value = true
  try {
    await serversStore.updateConfig(props.serverId, formData.value)
    notification.success('配置已保存', '')
  } catch (error: any) {
    notification.error('保存失败', error?.response?.data?.message || '')
  } finally {
    saving.value = false
  }
}

onMounted(() => {
  loadConfig()
})
</script>

<style scoped>
.config-form {
  background-color: var(--bg-card);
  border: 1px solid var(--border-color);
  border-radius: 12px;
  padding: 20px;
  transition: background-color 0.3s, border-color 0.3s;
}

:deep(.n-form) {
  max-width: 500px;
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
