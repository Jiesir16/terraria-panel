<template>
  <div class="config-form">
    <n-spin :show="loading">
      <n-form :model="formData" ref="formRef" label-placement="left" label-width="140px">
        <!-- 基本设置 -->
        <n-divider title-placement="left" style="margin-top: 0;">
          基本设置
        </n-divider>

        <n-form-item label="服务器名称" path="server_name">
          <n-input v-model:value="formData.server_name" placeholder="服务器名称" />
        </n-form-item>

        <n-form-item label="端口" path="port">
          <n-input-number v-model:value="formData.port" :min="1024" :max="65535" style="width: 100%;" />
        </n-form-item>

        <n-form-item label="最大玩家数" path="max_players">
          <n-input-number v-model:value="formData.max_players" :min="1" :max="255" style="width: 100%;" />
        </n-form-item>

        <n-form-item label="服务器密码" path="server_password">
          <n-input v-model:value="formData.server_password" type="password" placeholder="留空则无需密码" show-password-on="click" />
        </n-form-item>

        <n-form-item label="白名单" path="enable_whitelist">
          <n-checkbox v-model:checked="formData.enable_whitelist">启用白名单</n-checkbox>
        </n-form-item>

        <!-- 世界设置 -->
        <n-divider title-placement="left">
          世界设置
        </n-divider>

        <n-form-item label="世界名称" path="world_name">
          <n-input v-model:value="formData.world_name" placeholder="世界名称" />
        </n-form-item>

        <n-form-item label="自动创建世界" path="auto_create">
          <n-checkbox v-model:checked="formData.auto_create">
            没有存档时自动创建新世界
          </n-checkbox>
        </n-form-item>

        <n-form-item label="世界大小" path="world_size">
          <n-select
            v-model:value="worldSize"
            :options="worldSizeOptions"
            placeholder="选择世界大小"
            @update:value="handleWorldSizeChange"
          />
        </n-form-item>

        <n-grid :cols="2" :x-gap="12">
          <n-grid-item>
            <n-form-item label="宽度" path="world_width" label-placement="left">
              <n-input-number v-model:value="formData.world_width" :min="400" :max="16800" style="width: 100%;" />
            </n-form-item>
          </n-grid-item>
          <n-grid-item>
            <n-form-item label="高度" path="world_height" label-placement="left">
              <n-input-number v-model:value="formData.world_height" :min="400" :max="4800" style="width: 100%;" />
            </n-form-item>
          </n-grid-item>
        </n-grid>

        <n-form-item label="游戏难度" path="difficulty">
          <n-select
            v-model:value="formData.difficulty"
            :options="difficultyOptions"
            placeholder="选择游戏难度"
          />
        </n-form-item>

        <n-form-item label="世界种子" path="seed">
          <n-input v-model:value="formData.seed" placeholder="留空则随机生成" />
        </n-form-item>

        <n-form-item label="NPC 保护半径" path="npc_spawn_protection_radius">
          <n-input-number v-model:value="formData.npc_spawn_protection_radius" :min="0" :max="9999" style="width: 100%;" />
        </n-form-item>

        <div class="form-actions">
          <n-button type="primary" :loading="saving" @click="handleSave">
            保存配置
          </n-button>
          <n-button @click="loadConfig" :disabled="loading">
            重新加载
          </n-button>
        </div>
      </n-form>
    </n-spin>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { NSpin, NForm, NFormItem, NInput, NInputNumber, NSelect, NCheckbox, NButton, NDivider, NGrid, NGridItem } from 'naive-ui'
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

const formData = ref({
  server_name: '',
  port: 7777 as number | null,
  max_players: 8 as number | null,
  server_password: '',
  enable_whitelist: false,
  world_name: '',
  auto_create: false,
  world_width: 6400 as number | null,
  world_height: 1800 as number | null,
  difficulty: 0 as number | null,
  seed: '',
  npc_spawn_protection_radius: 300 as number | null,
})

const worldSizeOptions = [
  { label: '小 (4200×1200)', value: 1 },
  { label: '中 (6400×1800)', value: 2 },
  { label: '大 (8400×2400)', value: 3 },
  { label: '自定义', value: 0 }
]

const difficultyOptions = [
  { label: '经典 (Classic)', value: 0 },
  { label: '专家 (Expert)', value: 1 },
  { label: '大师 (Master)', value: 2 },
  { label: '旅途 (Journey)', value: 3 }
]

// Compute world size preset from width/height
const worldSize = ref(2)

function detectWorldSize(w: number | null, h: number | null): number {
  if (w === 4200 && h === 1200) return 1
  if (w === 6400 && h === 1800) return 2
  if (w === 8400 && h === 2400) return 3
  return 0
}

function handleWorldSizeChange(val: number) {
  worldSize.value = val
  const sizes: Record<number, [number, number]> = {
    1: [4200, 1200],
    2: [6400, 1800],
    3: [8400, 2400],
  }
  if (sizes[val]) {
    formData.value.world_width = sizes[val][0]
    formData.value.world_height = sizes[val][1]
  }
}

async function loadConfig() {
  loading.value = true
  try {
    const config = await serversStore.getConfig(props.serverId)
    formData.value = {
      server_name: config.server_name || '',
      port: config.port || 7777,
      max_players: config.max_players || 8,
      server_password: config.server_password || '',
      enable_whitelist: config.enable_whitelist || false,
      world_name: config.world_name || '',
      auto_create: config.auto_create || false,
      world_width: config.world_width || 6400,
      world_height: config.world_height || 1800,
      difficulty: config.difficulty ?? 0,
      seed: config.seed || '',
      npc_spawn_protection_radius: config.npc_spawn_protection_radius ?? 300,
    }
    worldSize.value = detectWorldSize(formData.value.world_width, formData.value.world_height)
  } catch (error) {
    notification.error('加载配置失败', '')
  } finally {
    loading.value = false
  }
}

async function handleSave() {
  saving.value = true
  try {
    await serversStore.updateConfig(props.serverId, formData.value as any)
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
  max-width: 560px;
}

:deep(.n-form-item) {
  margin-bottom: 16px;
}

:deep(.n-divider) {
  font-size: 14px;
  font-weight: 600;
}

.form-actions {
  display: flex;
  gap: 12px;
  margin-top: 8px;
}
</style>
