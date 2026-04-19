<template>
  <div class="config-form">
    <n-spin :show="loading">
      <n-form :model="formData" ref="formRef" label-placement="left" label-width="160px">
        <!-- 基本设置 -->
        <n-divider title-placement="left" style="margin-top: 0;">
          基本设置
        </n-divider>
        <div class="form-section">

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

        <n-form-item label="进服欢迎消息" path="motd">
          <n-input v-model:value="formData.motd" type="textarea" :rows="2" placeholder="玩家进入服务器时看到的消息" />
        </n-form-item>

        <n-form-item label="满员提示" path="server_full_no_reserve_reason">
          <n-input v-model:value="formData.server_full_no_reserve_reason" placeholder="服务器已满时显示的消息" />
        </n-form-item>

        </div>
        <!-- 世界设置 -->
        <n-divider title-placement="left">
          世界设置
        </n-divider>
        <div class="form-section">

        <n-form-item label="选择存档" path="world_file">
          <div style="display: flex; gap: 8px; width: 100%;">
            <n-select
              v-model:value="selectedWorldFile"
              :options="worldFileOptions"
              placeholder="选择已有的世界存档"
              clearable
              @update:value="handleWorldFileSelect"
              style="flex: 1;"
            />
            <n-button text type="primary" size="small" @click="loadWorldFiles" :loading="worldFilesLoading">
              刷新
            </n-button>
          </div>
          <div v-if="worldFileOptions.length === 0 && !worldFilesLoading" style="font-size: 12px; color: var(--text-muted); margin-top: 4px;">
            暂无存档，请先在"存档"页上传或导入 .wld 文件
          </div>
        </n-form-item>

        <n-form-item label="世界名称" path="world_name">
          <n-input v-model:value="formData.world_name" placeholder="世界名称（选择存档后自动填充）" />
        </n-form-item>

        <n-form-item label="自动创建世界" path="auto_create">
          <n-checkbox v-model:checked="formData.auto_create" :disabled="usingExistingWorld">
            没有存档时自动创建新世界
          </n-checkbox>
          <div v-if="usingExistingWorld" class="field-hint">
            当前已选择现有存档。自动创建、世界大小、难度和种子只会在“没有存档并创建新世界”时生效。
          </div>
        </n-form-item>

        <n-form-item label="世界大小" path="world_size">
          <n-select
            v-model:value="worldSize"
            :options="worldSizeOptions"
            placeholder="选择世界大小"
            :disabled="usingExistingWorld"
            @update:value="handleWorldSizeChange"
          />
        </n-form-item>

        <n-form-item label="宽度" path="world_width">
          <n-input-number v-model:value="formData.world_width" :min="400" :max="16800" :disabled="usingExistingWorld" style="width: 100%;" />
        </n-form-item>

        <n-form-item label="高度" path="world_height">
          <n-input-number v-model:value="formData.world_height" :min="400" :max="4800" :disabled="usingExistingWorld" style="width: 100%;" />
        </n-form-item>

        <n-form-item label="游戏难度" path="difficulty">
          <n-select
            v-model:value="formData.difficulty"
            :options="difficultyOptions"
            :disabled="usingExistingWorld"
            placeholder="选择游戏难度"
          />
        </n-form-item>

        <n-form-item label="世界种子" path="seed">
          <n-input v-model:value="formData.seed" :disabled="usingExistingWorld" placeholder="留空则随机生成" />
        </n-form-item>

        </div>
        <!-- 游戏规则 -->
        <n-divider title-placement="left">
          游戏规则
        </n-divider>
        <div class="form-section">

        <n-form-item label="白名单" path="enable_whitelist">
          <div>
            <n-checkbox v-model:checked="formData.enable_whitelist">启用白名单</n-checkbox>
            <div class="field-hint">启用后，只有白名单中的玩家才能加入。使用 /whitelist add 玩家名 添加。</div>
          </div>
        </n-form-item>

        <n-form-item label="PvP 模式" path="pvp_mode">
          <n-select
            v-model:value="formData.pvp_mode"
            :options="pvpModeOptions"
            placeholder="选择 PvP 模式"
          />
        </n-form-item>

        <n-form-item label="出生点保护" path="spawn_protection">
          <div>
            <n-checkbox v-model:checked="formData.spawn_protection">启用出生点保护</n-checkbox>
          </div>
        </n-form-item>

        <n-form-item v-if="formData.spawn_protection" label="保护半径" path="spawn_protection_radius">
          <n-input-number v-model:value="formData.spawn_protection_radius" :min="0" :max="9999" style="width: 100%;" />
        </n-form-item>

        <n-form-item label="NPC 保护半径" path="npc_spawn_protection_radius">
          <n-input-number v-model:value="formData.npc_spawn_protection_radius" :min="0" :max="9999" style="width: 100%;" />
        </n-form-item>

        <n-form-item label="时间控制" path="force_time">
          <n-select
            v-model:value="formData.force_time"
            :options="forceTimeOptions"
            placeholder="选择时间模式"
          />
        </n-form-item>

        <n-form-item label="角色模式限制">
          <n-space vertical>
            <n-checkbox v-model:checked="formData.soft_core_only">仅允许软核角色 (死亡掉金币)</n-checkbox>
            <n-checkbox v-model:checked="formData.medium_core_only">仅允许中核角色 (死亡掉物品)</n-checkbox>
            <n-checkbox v-model:checked="formData.hard_core_only">仅允许硬核角色 (死亡角色消失)</n-checkbox>
          </n-space>
        </n-form-item>

        <n-form-item label="禁止墓碑" path="disable_tombstones">
          <n-checkbox v-model:checked="formData.disable_tombstones">禁止死亡时掉落墓碑</n-checkbox>
        </n-form-item>

        <n-form-item label="禁止小丑炸弹" path="disable_clown_bombs">
          <n-checkbox v-model:checked="formData.disable_clown_bombs">禁止小丑炸弹破坏地形</n-checkbox>
        </n-form-item>

        <n-form-item label="禁止地牢守卫" path="disable_dungeon_guardian">
          <n-checkbox v-model:checked="formData.disable_dungeon_guardian">禁止地牢守卫生成</n-checkbox>
        </n-form-item>

        </div>
        <!-- 权限与安全 -->
        <n-divider title-placement="left">
          权限与安全
        </n-divider>
        <div class="form-section">

        <n-form-item label="服务端存档 (SSC)" path="server_side_character">
          <div class="ssc-config-row">
            <div>
              <n-checkbox v-model:checked="formData.server_side_character">启用服务端存档</n-checkbox>
            </div>
            <n-button v-if="authStore.isOperator" size="small" secondary @click="showSscConfig = true">
              SSC 详细配置
            </n-button>
          </div>
          <div>
            <div class="field-hint">强制使用服务端角色数据，防止玩家作弊修改本地存档。开启后玩家首次进入需注册。</div>
          </div>
        </n-form-item>

        <n-form-item label="全局禁止建造" path="disable_build">
          <div>
            <n-checkbox v-model:checked="formData.disable_build">开启全服硬核禁建（仅最高管理员可无视）</n-checkbox>
            <div class="field-hint">警告：开启后全服所有普通玩家（含登录完成且有 modify 权限者）均无法建造/破坏。普通生存服请勿勾选此项。</div>
          </div>
        </n-form-item>

        <n-form-item label="禁止隐身 PvP" path="disable_invisible_pvp">
          <n-checkbox v-model:checked="formData.disable_invisible_pvp">禁止使用隐身药水进行 PvP</n-checkbox>
        </n-form-item>

        </div>
        <!-- 反作弊 -->
        <n-divider title-placement="left">
          反作弊
        </n-divider>
        <div class="form-section">

        <n-form-item label="反作弊" path="anti_cheat">
          <n-checkbox v-model:checked="formData.anti_cheat">启用反作弊检测</n-checkbox>
        </n-form-item>

        <n-form-item label="范围检查" path="range_checks">
          <div>
            <n-checkbox v-model:checked="formData.range_checks">启用操作范围检查</n-checkbox>
            <div class="field-hint">检测玩家是否在超出正常范围的距离进行操作。</div>
          </div>
        </n-form-item>

        <n-form-item label="伤害踢出阈值" path="kick_on_damage_inflicted">
          <n-input-number v-model:value="formData.kick_on_damage_inflicted" :min="0" :max="99999" style="width: 100%;" />
          <div class="field-hint" style="margin-left: 8px;">单次造成超过此伤害值将被踢出，0 = 禁用</div>
        </n-form-item>

        <n-form-item label="受伤踢出阈值" path="kick_on_damage_received">
          <n-input-number v-model:value="formData.kick_on_damage_received" :min="0" :max="99999" style="width: 100%;" />
          <div class="field-hint" style="margin-left: 8px;">单次受到超过此伤害值将被踢出，0 = 禁用</div>
        </n-form-item>

        <n-form-item label="隐藏玩家数" path="disable_player_count_reporting">
          <n-checkbox v-model:checked="formData.disable_player_count_reporting">不向外部查询报告在线人数</n-checkbox>
        </n-form-item>

        </div>
        <!-- REST API -->
        <n-divider title-placement="left">
          REST API
        </n-divider>
        <div class="form-section">

        <n-form-item label="REST API" path="rest_api_enabled">
          <div>
            <n-checkbox v-model:checked="formData.rest_api_enabled">启用 TShock REST API</n-checkbox>
            <div class="field-hint">允许通过 HTTP 接口远程管理 TShock 服务器。</div>
          </div>
        </n-form-item>

        <n-form-item v-if="formData.rest_api_enabled" label="REST API 端口" path="rest_api_port">
          <n-input-number v-model:value="formData.rest_api_port" :min="1024" :max="65535" style="width: 100%;" />
        </n-form-item>
        </div>

        <div class="form-actions">
          <n-button v-if="authStore.isOperator" type="primary" :loading="saving" @click="handleSave">
            保存配置
          </n-button>
          <n-button @click="loadConfig" :disabled="loading">
            重新加载
          </n-button>
        </div>
      </n-form>
    </n-spin>

    <ssc-config-modal
      v-model:show="showSscConfig"
      :server-id="props.serverId"
      @saved="handleSscConfigSaved"
    />
  </div>
</template>

<script setup lang="ts">
import { computed, ref, onMounted } from 'vue'
import { NSpin, NForm, NFormItem, NInput, NInputNumber, NSelect, NCheckbox, NButton, NDivider, NGrid, NGridItem, NSpace } from 'naive-ui'
import { useAuthStore } from '../../stores/auth'
import { useServersStore } from '../../stores/servers'
import { serverApi } from '../../api/server'
import { useNotification } from '../../composables/useNotification'
import SscConfigModal from './SscConfigModal.vue'

interface Props {
  serverId: string
}

const props = defineProps<Props>()
const authStore = useAuthStore()
const serversStore = useServersStore()
const notification = useNotification()

const formRef = ref()
const loading = ref(false)
const saving = ref(false)
const showSscConfig = ref(false)

const formData = ref({
  // 基本设置
  server_name: '',
  port: 7777 as number | null,
  max_players: 8 as number | null,
  server_password: '',
  motd: '',
  server_full_no_reserve_reason: '',
  // 世界设置
  world_name: '',
  auto_create: false,
  world_width: 6400 as number | null,
  world_height: 1800 as number | null,
  difficulty: 0 as number | null,
  seed: '',
  // 游戏规则
  enable_whitelist: false,
  pvp_mode: 'normal',
  spawn_protection: true,
  spawn_protection_radius: 10 as number | null,
  npc_spawn_protection_radius: 300 as number | null,
  force_time: 'normal',
  hard_core_only: false,
  medium_core_only: false,
  soft_core_only: false,
  disable_tombstones: true,
  disable_clown_bombs: false,
  disable_dungeon_guardian: false,
  // 权限与安全
  server_side_character: false,
  disable_build: false,
  disable_invisible_pvp: false,
  // 反作弊
  anti_cheat: true,
  range_checks: true,
  kick_on_damage_inflicted: 0 as number | null,
  kick_on_damage_received: 0 as number | null,
  disable_player_count_reporting: false,
  // REST API
  rest_api_enabled: false,
  rest_api_port: 7878 as number | null,
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

const pvpModeOptions = [
  { label: '普通 (玩家自行开关)', value: 'normal' },
  { label: '始终开启 PvP', value: 'always' },
  { label: '禁止 PvP', value: 'disabled' }
]

const forceTimeOptions = [
  { label: '正常 (昼夜交替)', value: 'normal' },
  { label: '强制白天', value: 'day' },
  { label: '强制黑夜', value: 'night' }
]

// World file selection
const selectedWorldFile = ref<string | null>(null)
const worldFileOptions = ref<{ label: string; value: string }[]>([])
const worldFilesLoading = ref(false)
const usingExistingWorld = computed(() => {
  const worldName = formData.value.world_name?.trim()
  if (!worldName) {
    return false
  }
  return worldFileOptions.value.some(option => option.value === worldName)
})

async function loadWorldFiles() {
  worldFilesLoading.value = true
  try {
    const response = await serverApi.listWorlds(props.serverId)
    worldFileOptions.value = response.data
      .filter((w: any) => !w.is_backup)
      .map((w: any) => ({
        label: `${w.name} (${formatWorldSize(w.size)}, ${w.modified})`,
        value: w.name
      }))
    const backups = response.data
      .filter((w: any) => w.is_backup)
      .map((w: any) => ({
        label: `[备份] ${w.name} (${formatWorldSize(w.size)}, ${w.modified})`,
        value: w.name
      }))
    worldFileOptions.value = [...worldFileOptions.value, ...backups]
  } catch {
    // silently fail
  } finally {
    worldFilesLoading.value = false
  }
}

function formatWorldSize(bytes: number): string {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return Math.round(bytes / Math.pow(k, i) * 100) / 100 + ' ' + sizes[i]
}

function handleWorldFileSelect(val: string | null) {
  if (val) {
    formData.value.world_name = val
    formData.value.auto_create = false
  }
}

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
      motd: config.motd || '',
      server_full_no_reserve_reason: config.server_full_no_reserve_reason || '',
      world_name: config.world_name || '',
      auto_create: config.auto_create || false,
      world_width: config.world_width || 6400,
      world_height: config.world_height || 1800,
      difficulty: config.difficulty ?? 0,
      seed: config.seed || '',
      enable_whitelist: config.enable_whitelist || false,
      pvp_mode: config.pvp_mode || 'normal',
      spawn_protection: config.spawn_protection ?? true,
      spawn_protection_radius: config.spawn_protection_radius ?? 10,
      npc_spawn_protection_radius: config.npc_spawn_protection_radius ?? 300,
      force_time: config.force_time || 'normal',
      hard_core_only: config.hard_core_only || false,
      medium_core_only: config.medium_core_only || false,
      soft_core_only: config.soft_core_only || false,
      disable_tombstones: config.disable_tombstones ?? true,
      disable_clown_bombs: config.disable_clown_bombs || false,
      disable_dungeon_guardian: config.disable_dungeon_guardian || false,
      server_side_character: config.server_side_character || false,
      disable_build: config.disable_build || false,
      disable_invisible_pvp: config.disable_invisible_pvp || false,
      anti_cheat: config.anti_cheat ?? true,
      range_checks: config.range_checks ?? true,
      kick_on_damage_inflicted: config.kick_on_damage_inflicted ?? 0,
      kick_on_damage_received: config.kick_on_damage_received ?? 0,
      disable_player_count_reporting: config.disable_player_count_reporting || false,
      rest_api_enabled: config.rest_api_enabled || false,
      rest_api_port: config.rest_api_port || 7878,
    }
    worldSize.value = detectWorldSize(formData.value.world_width, formData.value.world_height)
    if (formData.value.world_name) {
      selectedWorldFile.value = formData.value.world_name
    }
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
    await serversStore.updateServer(props.serverId, {
      world_name: formData.value.world_name || undefined,
      port: formData.value.port ?? undefined,
      password: formData.value.server_password ?? '',
      max_players: formData.value.max_players ?? undefined,
    })
    notification.success(
      '配置已保存',
      usingExistingWorld.value
        ? '已选择现有存档；世界大小、难度和种子不会改写现有地图，只会在新建世界时生效'
        : '下次启动服务器时生效'
    )
  } catch (error: any) {
    notification.error('保存失败', error?.response?.data?.message || '')
  } finally {
    saving.value = false
  }
}

function handleSscConfigSaved(config: { Settings: { Enabled: boolean } }) {
  formData.value.server_side_character = config.Settings.Enabled
}

onMounted(() => {
  loadConfig()
  loadWorldFiles()
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
  max-width: 1000px;
}

.form-section {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 0 32px;
}

@media (max-width: 768px) {
  .form-section {
    grid-template-columns: 1fr;
  }
}

:deep(.n-form-item) {
  margin-bottom: 16px;
}

:deep(.n-divider) {
  font-size: 14px;
  font-weight: 600;
}

.field-hint {
  font-size: 12px;
  color: var(--text-muted, #808080);
  margin-top: 4px;
  line-height: 1.4;
}

.ssc-config-row {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-wrap: wrap;
}

.form-actions {
  display: flex;
  gap: 12px;
  margin-top: 8px;
}
</style>
