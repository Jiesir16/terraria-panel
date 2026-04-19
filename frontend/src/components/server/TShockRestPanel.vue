<template>
  <div class="tshock-rest-panel">
    <div class="section-header">
      <h3>TShock 实时管理 (REST API)</h3>
      <n-button text type="primary" @click="refreshAll" :loading="statusLoading">刷新</n-button>
    </div>

    <!-- REST needs restart banner -->
    <n-alert v-if="needsRestart" type="warning" :show-icon="true" style="margin-bottom: 12px;">
      <template #header>REST API Token 已自动配置</template>
      {{ setupMessage }}
      <n-button size="small" type="warning" style="margin-left: 12px;" @click="handleRestartForRest" :loading="restartingForRest">
        立即重启服务器
      </n-button>
    </n-alert>

    <!-- REST API error tip -->
    <n-alert v-if="restError && !needsRestart" type="error" :show-icon="true" style="margin-bottom: 12px;">
      {{ restError }}
    </n-alert>

    <div class="rest-cards">
      <!-- ─── Server Status ─── -->
      <n-card title="服务器状态" class="rest-card">
        <n-spin :show="statusLoading">
          <div v-if="serverStatus" class="status-grid">
            <div class="stat-item">
              <span class="stat-label">服务器名称</span>
              <span class="stat-value">{{ serverStatus.name }}</span>
            </div>
            <div class="stat-item">
              <span class="stat-label">TShock 版本</span>
              <span class="stat-value">{{ serverStatus.tshockversion }}</span>
            </div>
            <div class="stat-item">
              <span class="stat-label">游戏版本</span>
              <span class="stat-value">{{ serverStatus.serverversion }}</span>
            </div>
            <div class="stat-item">
              <span class="stat-label">端口</span>
              <span class="stat-value">{{ serverStatus.port }}</span>
            </div>
            <div class="stat-item">
              <span class="stat-label">玩家数</span>
              <span class="stat-value">{{ serverStatus.playercount }} / {{ serverStatus.maxplayers }}</span>
            </div>
            <div class="stat-item">
              <span class="stat-label">世界</span>
              <span class="stat-value">{{ serverStatus.world }}</span>
            </div>
            <div class="stat-item">
              <span class="stat-label">运行时间</span>
              <span class="stat-value">{{ serverStatus.uptime }}</span>
            </div>
            <div class="stat-item">
              <span class="stat-label">密码保护</span>
              <span class="stat-value">{{ serverStatus.serverpassword ? '是' : '否' }}</span>
            </div>
          </div>
          <div v-else class="empty-note">无法获取服务器状态，请确认服务器正在运行且 REST API 已启用。</div>
        </n-spin>

        <!-- MOTD / Rules -->
        <div style="margin-top: 16px; display: flex; gap: 12px;">
          <n-button size="small" @click="loadMotd" :loading="motdLoading">查看 MOTD</n-button>
          <n-button size="small" @click="loadRules" :loading="rulesLoading">查看规则</n-button>
        </div>
        <n-card v-if="motdText" size="small" style="margin-top: 8px;" title="MOTD">
          <pre class="pre-block">{{ motdText }}</pre>
        </n-card>
        <n-card v-if="rulesText" size="small" style="margin-top: 8px;" title="服务器规则">
          <pre class="pre-block">{{ rulesText }}</pre>
        </n-card>
      </n-card>

      <!-- ─── Online Players ─── -->
      <n-card title="在线玩家" class="rest-card">
        <div class="sub-section-header">
          <n-button size="small" @click="loadPlayers" :loading="playersLoading">刷新玩家列表</n-button>
        </div>
        <n-spin :show="playersLoading">
          <div v-if="players.length === 0" class="empty-note">当前没有在线玩家</div>
          <n-data-table
            v-else
            :columns="playerColumns"
            :data="players"
            :row-key="(row: any) => row.nickname || row.index"
            size="small"
            striped
          />
        </n-spin>
      </n-card>

      <!-- ─── Item Give ─── -->
      <n-card title="物品发放" class="rest-card">
        <div class="command-section">
          <h4>发放物品</h4>
          <p class="hint-text">物品 ID 清单按当前服务器 TShock 版本缓存；缓存不存在时会自动从 wiki.gg 下载，并合并中文名。</p>
          <div class="item-form">
            <n-select
              v-model:value="givePlayer"
              :options="playerOptions"
              filterable
              tag
              clearable
              placeholder="选择在线玩家，或输入玩家名"
            />
            <n-select
              v-model:value="selectedItemId"
              :options="itemOptions"
              filterable
              clearable
              placeholder="选择物品"
              :loading="itemsLoading"
            />
            <n-input-number v-model:value="giveStack" :min="1" :max="9999" placeholder="数量" />
            <n-button type="primary" @click="handleGiveItem" :loading="itemGiveLoading" :disabled="!canGiveItem">
              发放
            </n-button>
          </div>
        </div>

        <div class="sub-section-header" style="margin-top: 18px;">
          <n-input v-model:value="itemQuery" placeholder="搜索物品 ID / 中文名 / 英文名 / 内部名" clearable @keyup.enter="loadItems" />
          <n-button size="small" @click="loadItems" :loading="itemsLoading">搜索/刷新</n-button>
          <n-button size="small" type="warning" @click="syncItems" :loading="itemSyncLoading">重新下载物品清单</n-button>
        </div>
        <div class="hint-text" v-if="itemCatalogVersion">
          当前清单：{{ itemCatalogVersion }} · {{ itemCatalogSource }} · 已显示 {{ items.length }} 条
        </div>
        <n-spin :show="itemsLoading">
          <div v-if="items.length === 0" class="empty-note">暂无物品清单，点击“搜索/刷新”或“重新下载物品清单”。</div>
          <n-data-table
            v-else
            :columns="itemColumns"
            :data="items"
            :row-key="(row: any) => row.id"
            :pagination="{ pageSize: 20 }"
            size="small"
            striped
          />
        </n-spin>
        <n-card v-if="itemGiveResult" size="small" style="margin-top: 12px;" title="最近发放结果">
          <pre class="pre-block">{{ itemGiveResult }}</pre>
        </n-card>
      </n-card>

      <!-- ─── Bans ─── -->
      <n-card title="封禁管理" class="rest-card">
        <div class="sub-section-header">
          <n-button size="small" @click="loadBans" :loading="bansLoading">刷新封禁列表</n-button>
          <n-button size="small" type="primary" @click="showBanModal = true">+ 添加封禁</n-button>
        </div>
        <n-spin :show="bansLoading">
          <div v-if="bans.length === 0" class="empty-note">暂无封禁记录</div>
          <n-data-table
            v-else
            :columns="banColumns"
            :data="bans"
            :row-key="(row: any) => row.ticketNumber"
            :pagination="{ pageSize: 10 }"
            size="small"
            striped
          />
        </n-spin>

        <!-- Ban create modal -->
        <n-modal v-model:show="showBanModal" preset="dialog" title="添加封禁" positive-text="封禁" negative-text="取消" @positive-click="handleCreateBan">
          <n-form>
            <n-form-item label="标识符 (IP / 玩家名 / UUID)">
              <n-input v-model:value="banForm.identifier" placeholder="如 ip:1.2.3.4 或 name:Player1" />
            </n-form-item>
            <n-form-item label="原因">
              <n-input v-model:value="banForm.reason" placeholder="封禁原因" />
            </n-form-item>
            <n-form-item label="过期时间 (可选)">
              <n-input v-model:value="banForm.duration" placeholder="如 2024-12-31 或留空为永久" />
            </n-form-item>
          </n-form>
        </n-modal>
      </n-card>

      <!-- ─── World Operations ─── -->
      <n-card title="世界操作" class="rest-card">
        <n-spin :show="worldLoading">
          <div v-if="worldInfo" class="status-grid">
            <div class="stat-item">
              <span class="stat-label">世界名称</span>
              <span class="stat-value">{{ worldInfo.name }}</span>
            </div>
            <div class="stat-item">
              <span class="stat-label">世界大小</span>
              <span class="stat-value">{{ worldInfo.size }}</span>
            </div>
            <div class="stat-item">
              <span class="stat-label">时间</span>
              <span class="stat-value">{{ worldInfo.daytime ? '白天' : '夜晚' }} ({{ worldInfo.time }})</span>
            </div>
            <div class="stat-item">
              <span class="stat-label">血月</span>
              <n-tag size="small" :type="worldInfo.bloodmoon ? 'error' : 'default'">
                {{ worldInfo.bloodmoon ? '进行中' : '未触发' }}
              </n-tag>
            </div>
            <div class="stat-item">
              <span class="stat-label">入侵规模</span>
              <span class="stat-value">{{ worldInfo.invasionsize }}</span>
            </div>
          </div>
          <div v-else class="empty-note">无法获取世界信息</div>
        </n-spin>

        <div class="world-actions">
          <n-button type="primary" size="small" @click="handleWorldSave" :loading="actionLoading.worldSave">
            保存世界
          </n-button>
          <n-button type="warning" size="small" @click="handleButcher(false)" :loading="actionLoading.butcher">
            清除敌怪
          </n-button>
          <n-button type="warning" size="small" @click="handleButcher(true)" :loading="actionLoading.butcherFriendly">
            清除所有NPC
          </n-button>
          <n-button type="error" size="small" @click="handleBloodmoon(true)" :loading="actionLoading.bloodmoonOn">
            触发血月
          </n-button>
          <n-button size="small" @click="handleBloodmoon(false)" :loading="actionLoading.bloodmoonOff">
            关闭血月
          </n-button>
          <n-button type="info" size="small" @click="handleMeteor" :loading="actionLoading.meteor">
            召唤陨石
          </n-button>
          <n-button type="success" size="small" @click="handleAutosave(true)" :loading="actionLoading.autosaveOn">
            开启自动保存
          </n-button>
          <n-button size="small" @click="handleAutosave(false)" :loading="actionLoading.autosaveOff">
            关闭自动保存
          </n-button>
          <n-button size="small" @click="handleReload" :loading="actionLoading.reload">
            重载配置
          </n-button>
        </div>
      </n-card>

      <!-- ─── Broadcast & Raw Command ─── -->
      <n-card title="广播与命令" class="rest-card">
        <div class="command-section">
          <h4>全服广播</h4>
          <div class="input-row">
            <n-input v-model:value="broadcastMsg" placeholder="输入广播消息..." @keyup.enter="handleBroadcast" />
            <n-button type="primary" @click="handleBroadcast" :loading="actionLoading.broadcast" :disabled="!broadcastMsg.trim()">
              发送广播
            </n-button>
          </div>
        </div>

        <div class="command-section" style="margin-top: 20px;">
          <h4>执行原始命令</h4>
          <p class="hint-text">直接在服务器上执行 TShock 命令（不需要带 / 前缀）</p>
          <div class="input-row">
            <n-input v-model:value="rawCmd" placeholder="如: time noon 或 give 1 1 DirtBlock" @keyup.enter="handleRawcmd" />
            <n-button type="warning" @click="handleRawcmd" :loading="actionLoading.rawcmd" :disabled="!rawCmd.trim()">
              执行
            </n-button>
          </div>
          <n-card v-if="rawCmdResult" size="small" style="margin-top: 8px;" title="命令输出">
            <pre class="pre-block">{{ rawCmdResult }}</pre>
          </n-card>
        </div>
      </n-card>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, h, onMounted, reactive, computed } from 'vue'
import {
  NButton, NSpin, NDataTable, NTag, NAlert, NCard,
  NInput, NModal, NForm, NFormItem, NSelect, NInputNumber, useDialog
} from 'naive-ui'
import type { DataTableColumns } from 'naive-ui'
import { tshockRestApi } from '../../api/tshockRest'
import type { TerrariaItem, TShockRestPlayer, TShockServerStatus, TShockWorldInfo } from '../../api/tshockRest'
import { useNotification } from '../../composables/useNotification'
import { useServersStore } from '../../stores/servers'

const props = defineProps<{
  serverId: string
}>()

const notification = useNotification()
const dialog = useDialog()
const serversStore = useServersStore()

// ─── State ───

const restError = ref('')
const needsRestart = ref(false)
const setupMessage = ref('')
const restartingForRest = ref(false)

// Server status
const statusLoading = ref(false)
const serverStatus = ref<TShockServerStatus | null>(null)
const motdLoading = ref(false)
const motdText = ref('')
const rulesLoading = ref(false)
const rulesText = ref('')

// Players
const playersLoading = ref(false)
const players = ref<TShockRestPlayer[]>([])

// Items
const itemsLoading = ref(false)
const itemSyncLoading = ref(false)
const itemGiveLoading = ref(false)
const itemQuery = ref('')
const items = ref<TerrariaItem[]>([])
const itemCatalogVersion = ref('')
const itemCatalogSource = ref('')
const selectedItemId = ref<number | null>(null)
const givePlayer = ref('')
const giveStack = ref(1)
const itemGiveResult = ref('')

// Bans
const bansLoading = ref(false)
const bans = ref<any[]>([])
const showBanModal = ref(false)
const banForm = reactive({ identifier: '', reason: '', duration: '' })

// World
const worldLoading = ref(false)
const worldInfo = ref<TShockWorldInfo | null>(null)

// Commands
const broadcastMsg = ref('')
const rawCmd = ref('')
const rawCmdResult = ref('')

// Action loading states
const actionLoading = reactive({
  worldSave: false,
  butcher: false,
  butcherFriendly: false,
  bloodmoonOn: false,
  bloodmoonOff: false,
  meteor: false,
  autosaveOn: false,
  autosaveOff: false,
  reload: false,
  broadcast: false,
  rawcmd: false,
})

// ─── Player columns ───

const playerColumns: DataTableColumns = [
  { title: '昵称', key: 'nickname', width: 150 },
  { title: '用户名', key: 'username', width: 120 },
  { title: '组', key: 'group', width: 100 },
  {
    title: '队伍',
    key: 'team',
    width: 80,
    render: (row: any) => {
      const teams: Record<number, string> = { 0: '无', 1: '红', 2: '绿', 3: '蓝', 4: '黄', 5: '粉' }
      return teams[row.team] || String(row.team)
    }
  },
  {
    title: '操作',
    key: 'actions',
    width: 200,
    render: (row: any) => {
      return h('div', { style: 'display:flex;gap:6px;' }, [
        h(NButton, { size: 'tiny', type: 'warning', onClick: () => handleKick(row.nickname) }, { default: () => '踢出' }),
        h(NButton, { size: 'tiny', type: 'error', onClick: () => handleKillPlayer(row.nickname) }, { default: () => '击杀' }),
        h(NButton, { size: 'tiny', onClick: () => handleMute(row.nickname) }, { default: () => '禁言' }),
        h(NButton, { size: 'tiny', onClick: () => handleUnmute(row.nickname) }, { default: () => '解禁' }),
      ])
    }
  },
]

const playerOptions = computed(() => players.value
  .filter((player: any) => player.nickname)
  .map((player: any) => ({ label: player.nickname, value: player.nickname }))
)

function itemDisplayName(item: TerrariaItem) {
  return item.zh_name ? `${item.zh_name} / ${item.name}` : item.name
}

const itemOptions = computed(() => items.value.map((item) => ({
  label: `#${item.id} ${itemDisplayName(item)} (${item.internal_name})`,
  value: item.id,
})))

const selectedItem = computed(() => items.value.find((item) => item.id === selectedItemId.value) || null)

const canGiveItem = computed(() => {
  return !!givePlayer.value.trim() && !!selectedItemId.value && giveStack.value >= 1
})

const itemColumns: DataTableColumns = [
  { title: 'ID', key: 'id', width: 80 },
  { title: '中文名', key: 'zh_name', width: 160, render: (row: any) => row.zh_name || '-' },
  { title: '名称', key: 'name', width: 180 },
  { title: '内部名', key: 'internal_name', width: 220 },
  {
    title: '操作',
    key: 'actions',
    width: 90,
    render: (row: any) => {
      return h(NButton, {
        size: 'tiny',
        type: selectedItemId.value === row.id ? 'primary' : 'default',
        onClick: () => {
          selectedItemId.value = row.id
        }
      }, { default: () => selectedItemId.value === row.id ? '已选择' : '选择' })
    }
  },
]

// ─── Ban columns ───

const banColumns: DataTableColumns = [
  { title: '票号', key: 'ticketNumber', width: 70 },
  { title: '标识符', key: 'identifier', width: 180 },
  { title: '原因', key: 'reason', width: 180 },
  { title: '执行者', key: 'banningUser', width: 100 },
  { title: '日期', key: 'date', width: 150 },
  { title: '过期', key: 'expiration', width: 150 },
  {
    title: '操作',
    key: 'actions',
    width: 80,
    render: (row: any) => {
      return h(NButton, {
        size: 'tiny',
        type: 'error',
        onClick: () => handleUnban(row.ticketNumber)
      }, { default: () => '解封' })
    }
  },
]

// ─── Data loading ───

async function loadStatus() {
  statusLoading.value = true
  restError.value = ''
  try {
    const resp = await tshockRestApi.serverStatus(props.serverId)
    serverStatus.value = resp.data as any
    // Extract players if present
    if (serverStatus.value?.players) {
      players.value = serverStatus.value.players
    }
  } catch (e: any) {
    restError.value = e?.response?.data?.error || 'REST API 连接失败，请确认服务器正在运行且 REST API 已启用。'
    serverStatus.value = null
  } finally {
    statusLoading.value = false
  }
}

async function loadMotd() {
  motdLoading.value = true
  try {
    const resp = await tshockRestApi.serverMotd(props.serverId)
    motdText.value = (resp.data as any)?.motd || JSON.stringify(resp.data, null, 2)
  } catch (e: any) {
    notification.error('获取 MOTD 失败', e?.response?.data?.error || '')
  } finally {
    motdLoading.value = false
  }
}

async function loadRules() {
  rulesLoading.value = true
  try {
    const resp = await tshockRestApi.serverRules(props.serverId)
    rulesText.value = (resp.data as any)?.rules || JSON.stringify(resp.data, null, 2)
  } catch (e: any) {
    notification.error('获取规则失败', e?.response?.data?.error || '')
  } finally {
    rulesLoading.value = false
  }
}

async function loadPlayers() {
  playersLoading.value = true
  try {
    const resp = await tshockRestApi.playerList(props.serverId)
    const data = resp.data as any
    players.value = data?.players || []
  } catch (e: any) {
    notification.error('获取玩家列表失败', e?.response?.data?.error || '')
  } finally {
    playersLoading.value = false
  }
}

async function loadItems() {
  itemsLoading.value = true
  try {
    const resp = await tshockRestApi.itemList(props.serverId, itemQuery.value || undefined, 10000)
    items.value = resp.data.items || []
    itemCatalogVersion.value = resp.data.version || ''
    itemCatalogSource.value = resp.data.source || ''
  } catch (e: any) {
    notification.error('获取物品清单失败', e?.response?.data?.error || '')
  } finally {
    itemsLoading.value = false
  }
}

async function syncItems() {
  itemSyncLoading.value = true
  try {
    const resp = await tshockRestApi.itemSync(props.serverId)
    items.value = resp.data.items || []
    itemCatalogVersion.value = resp.data.version || ''
    itemCatalogSource.value = resp.data.source || ''
    notification.success('物品清单已更新', `共 ${items.value.length} 条`)
  } catch (e: any) {
    notification.error('下载物品清单失败', e?.response?.data?.error || '')
  } finally {
    itemSyncLoading.value = false
  }
}

async function loadBans() {
  bansLoading.value = true
  try {
    const resp = await tshockRestApi.banList(props.serverId)
    const data = resp.data as any
    bans.value = data?.bans || []
  } catch (e: any) {
    notification.error('获取封禁列表失败', e?.response?.data?.error || '')
  } finally {
    bansLoading.value = false
  }
}

async function loadWorld() {
  worldLoading.value = true
  try {
    const resp = await tshockRestApi.worldRead(props.serverId)
    worldInfo.value = resp.data as any
  } catch (e: any) {
    worldInfo.value = null
  } finally {
    worldLoading.value = false
  }
}

function refreshAll() {
  loadStatus()
  loadWorld()
  loadPlayers()
  loadBans()
  loadItems()
}

// ─── REST Setup ───

async function checkRestSetup() {
  try {
    const resp = await tshockRestApi.setup(props.serverId)
    const data = resp.data as any
    if (!data.ready) {
      needsRestart.value = true
      setupMessage.value = data.message || 'REST API Token 已写入配置，需要重启服务器使其生效。'
    } else {
      needsRestart.value = false
      setupMessage.value = ''
    }
  } catch (e: any) {
    // Setup check failed — could be server not started etc, non-fatal
    traceRestError(e)
  }
}

function traceRestError(e: any) {
  const msg = e?.response?.data?.error || ''
  if (msg.includes('not found') || msg.includes('config.json')) {
    restError.value = 'TShock 配置文件不存在，请先启动一次服务器。'
  }
}

async function handleRestartForRest() {
  restartingForRest.value = true
  try {
    await serversStore.restartServer(props.serverId)
    notification.success('服务器正在重启', 'REST API Token 将在重启后生效，请稍后刷新。')
    needsRestart.value = false
    // Wait a bit then try to load status
    setTimeout(() => {
      refreshAll()
    }, 8000)
  } catch (e: any) {
    notification.error('重启失败', e?.response?.data?.error || '')
  } finally {
    restartingForRest.value = false
  }
}

// ─── Player actions ───

function handleKick(player: string) {
  dialog.warning({
    title: '确认踢出',
    content: `确定要踢出玩家「${player}」吗？`,
    positiveText: '踢出',
    negativeText: '取消',
    onPositiveClick: async () => {
      try {
        await tshockRestApi.playerKick(props.serverId, player, '被管理员踢出')
        notification.success('已踢出玩家', player)
        loadPlayers()
      } catch (e: any) {
        notification.error('踢出失败', e?.response?.data?.error || '')
      }
    }
  })
}

function handleKillPlayer(player: string) {
  dialog.warning({
    title: '确认击杀',
    content: `确定要击杀玩家「${player}」吗？`,
    positiveText: '击杀',
    negativeText: '取消',
    onPositiveClick: async () => {
      try {
        await tshockRestApi.playerKill(props.serverId, player)
        notification.success('已击杀玩家', player)
      } catch (e: any) {
        notification.error('击杀失败', e?.response?.data?.error || '')
      }
    }
  })
}

async function handleMute(player: string) {
  try {
    await tshockRestApi.playerMute(props.serverId, player)
    notification.success('已禁言', player)
  } catch (e: any) {
    notification.error('禁言失败', e?.response?.data?.error || '')
  }
}

async function handleUnmute(player: string) {
  try {
    await tshockRestApi.playerUnmute(props.serverId, player)
    notification.success('已解除禁言', player)
  } catch (e: any) {
    notification.error('解禁失败', e?.response?.data?.error || '')
  }
}

function handleGiveItem() {
  if (!canGiveItem.value || !selectedItem.value) return
  const player = givePlayer.value.trim()
  const item = selectedItem.value
  const stack = giveStack.value || 1

  dialog.warning({
    title: '确认发放物品',
    content: `确定给「${player}」发放 ${stack} 个 #${item.id} ${item.name} 吗？`,
    positiveText: '发放',
    negativeText: '取消',
    onPositiveClick: async () => {
      itemGiveLoading.value = true
      try {
        const resp = await tshockRestApi.itemGive(props.serverId, {
          player,
          item_id: item.id,
          stack,
        })
        itemGiveResult.value = JSON.stringify(resp.data, null, 2)
        notification.success('物品已发放', `${player} <- ${stack} x ${itemDisplayName(item)}`)
      } catch (e: any) {
        itemGiveResult.value = e?.response?.data ? JSON.stringify(e.response.data, null, 2) : ''
        notification.error('发放失败', e?.response?.data?.error || '')
      } finally {
        itemGiveLoading.value = false
      }
    }
  })
}

// ─── Ban actions ───

async function handleCreateBan() {
  if (!banForm.identifier.trim()) {
    notification.error('请输入封禁标识符', '')
    return false
  }
  try {
    await tshockRestApi.banCreate(
      props.serverId,
      banForm.identifier,
      banForm.reason || undefined,
      banForm.duration || undefined
    )
    notification.success('封禁已创建', '')
    banForm.identifier = ''
    banForm.reason = ''
    banForm.duration = ''
    showBanModal.value = false
    loadBans()
  } catch (e: any) {
    notification.error('封禁失败', e?.response?.data?.error || '')
  }
  return false
}

function handleUnban(ticket: string) {
  dialog.warning({
    title: '确认解封',
    content: `确定要解除封禁 #${ticket} 吗？`,
    positiveText: '解封',
    negativeText: '取消',
    onPositiveClick: async () => {
      try {
        await tshockRestApi.banDestroy(props.serverId, ticket)
        notification.success('已解封', `票号 #${ticket}`)
        loadBans()
      } catch (e: any) {
        notification.error('解封失败', e?.response?.data?.error || '')
      }
    }
  })
}

// ─── World actions ───

async function handleWorldSave() {
  actionLoading.worldSave = true
  try {
    await tshockRestApi.worldSave(props.serverId)
    notification.success('世界已保存', '')
  } catch (e: any) {
    notification.error('保存失败', e?.response?.data?.error || '')
  } finally {
    actionLoading.worldSave = false
  }
}

async function handleButcher(killFriendly: boolean) {
  const key = killFriendly ? 'butcherFriendly' : 'butcher'
  actionLoading[key] = true
  try {
    await tshockRestApi.worldButcher(props.serverId, killFriendly)
    notification.success(killFriendly ? '所有 NPC 已清除' : '敌怪已清除', '')
  } catch (e: any) {
    notification.error('清除失败', e?.response?.data?.error || '')
  } finally {
    actionLoading[key] = false
  }
}

async function handleBloodmoon(state: boolean) {
  const key = state ? 'bloodmoonOn' : 'bloodmoonOff'
  actionLoading[key] = true
  try {
    await tshockRestApi.worldBloodmoon(props.serverId, state)
    notification.success(state ? '血月已触发' : '血月已关闭', '')
    loadWorld()
  } catch (e: any) {
    notification.error('操作失败', e?.response?.data?.error || '')
  } finally {
    actionLoading[key] = false
  }
}

async function handleMeteor() {
  actionLoading.meteor = true
  try {
    await tshockRestApi.worldMeteor(props.serverId)
    notification.success('陨石已召唤', '')
  } catch (e: any) {
    notification.error('召唤失败', e?.response?.data?.error || '')
  } finally {
    actionLoading.meteor = false
  }
}

async function handleAutosave(state: boolean) {
  const key = state ? 'autosaveOn' : 'autosaveOff'
  actionLoading[key] = true
  try {
    await tshockRestApi.worldAutosave(props.serverId, state)
    notification.success(state ? '自动保存已开启' : '自动保存已关闭', '')
  } catch (e: any) {
    notification.error('操作失败', e?.response?.data?.error || '')
  } finally {
    actionLoading[key] = false
  }
}

async function handleReload() {
  actionLoading.reload = true
  try {
    await tshockRestApi.serverReload(props.serverId)
    notification.success('配置已重载', '')
  } catch (e: any) {
    notification.error('重载失败', e?.response?.data?.error || '')
  } finally {
    actionLoading.reload = false
  }
}

// ─── Broadcast & Raw Command ───

async function handleBroadcast() {
  if (!broadcastMsg.value.trim()) return
  actionLoading.broadcast = true
  try {
    await tshockRestApi.serverBroadcast(props.serverId, broadcastMsg.value)
    notification.success('广播已发送', '')
    broadcastMsg.value = ''
  } catch (e: any) {
    notification.error('广播失败', e?.response?.data?.error || '')
  } finally {
    actionLoading.broadcast = false
  }
}

async function handleRawcmd() {
  if (!rawCmd.value.trim()) return
  actionLoading.rawcmd = true
  rawCmdResult.value = ''
  try {
    const resp = await tshockRestApi.serverRawcmd(props.serverId, rawCmd.value)
    const data = resp.data as any
    rawCmdResult.value = data?.response || JSON.stringify(data, null, 2)
    rawCmd.value = ''
  } catch (e: any) {
    notification.error('命令执行失败', e?.response?.data?.error || '')
  } finally {
    actionLoading.rawcmd = false
  }
}

// ─── Init ───

onMounted(async () => {
  await checkRestSetup()
  refreshAll()
})

defineExpose({ refreshAll })
</script>

<style scoped>
.tshock-rest-panel {
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

.status-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
  gap: 12px;
  margin-bottom: 12px;
}

.rest-cards {
  display: grid;
  gap: 16px;
}

.rest-card {
  border: 1px solid var(--border-color);
}

.stat-item {
  background: var(--bg-overlay, rgba(255,255,255,0.03));
  border: 1px solid var(--border-color);
  border-radius: 8px;
  padding: 12px;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.stat-label {
  font-size: 12px;
  color: var(--text-muted, #808080);
}

.stat-value {
  font-size: 14px;
  font-weight: 500;
  color: var(--text-primary);
}

.sub-section-header {
  display: flex;
  gap: 8px;
  margin-bottom: 12px;
}

.world-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-top: 16px;
  padding-top: 16px;
  border-top: 1px solid var(--border-color);
}

.command-section h4 {
  margin: 0 0 8px 0;
  color: var(--text-primary);
}

.hint-text {
  font-size: 12px;
  color: var(--text-muted, #808080);
  margin: 0 0 8px 0;
}

.input-row {
  display: flex;
  gap: 8px;
}

.item-form {
  display: grid;
  grid-template-columns: minmax(160px, 1fr) minmax(260px, 2fr) 120px auto;
  gap: 8px;
  align-items: center;
}

.pre-block {
  margin: 0;
  white-space: pre-wrap;
  word-break: break-all;
  font-family: 'JetBrains Mono', 'Fira Code', monospace;
  font-size: 13px;
  color: var(--text-primary);
}

.empty-note {
  color: var(--text-muted, #808080);
  font-size: 13px;
  padding: 20px;
  text-align: center;
}

@media (max-width: 900px) {
  .item-form {
    grid-template-columns: 1fr;
  }
}
</style>
