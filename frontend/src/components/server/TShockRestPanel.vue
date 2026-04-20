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
      <n-card title="REST API 状态" class="rest-card">
        <div class="world-actions compact">
          <n-button size="small" @click="handleTokenTest" :loading="tokenTestLoading">测试 Token</n-button>
          <n-button size="small" type="warning" @click="handleRestRestart" :loading="actionLoading.restRestart">
            REST 重启服务器
          </n-button>
        </div>
        <p class="hint-text" style="margin-top: 10px;">
          已接入：server status/broadcast/off/rawcmd/reload/restart、players list/read/kick/ban/kill/mute/unmute、users、groups、bans、world。
        </p>
        <n-card v-show="tokenTestResult" size="small" title="Token 测试结果">
          <pre class="pre-block">{{ tokenTestResult }}</pre>
        </n-card>
      </n-card>

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
        <n-card v-show="motdText" size="small" style="margin-top: 8px;" title="MOTD">
          <pre class="pre-block">{{ motdText }}</pre>
        </n-card>
        <n-card v-show="rulesText" size="small" style="margin-top: 8px;" title="服务器规则">
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
            :pagination="{ pageSize: 8 }"
            size="small"
            striped
          />
        </n-spin>
        <n-card v-show="itemGiveResult" size="small" style="margin-top: 12px;" title="最近发放结果">
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

      <!-- ─── Quick Commands ─── -->
      <n-card title="快捷命令" class="rest-card compact-card">
        <p class="hint-text">只保留 REST/rawcmd 在控制台上下文可可靠执行的命令。传送、Boss 召唤这类游戏内上下文命令不会再直接发送。</p>

        <div style="margin-bottom: 12px;">
          <n-select
            v-model:value="quickCmdPlayer"
            :options="playerOptions"
            filterable
            tag
            clearable
            placeholder="目标玩家 (部分命令需要)"
            style="max-width: 300px;"
          />
        </div>

        <div class="quick-cmd-group">
          <h4>增益 Buff</h4>
          <div class="input-row">
            <n-select
              v-model:value="selectedBuffId"
              :options="buffOptions"
              filterable
              tag
              clearable
              placeholder="选择 Buff / Debuff，或直接输入 ID"
              style="min-width: 260px;"
            />
            <n-input-number v-model:value="customBuffDuration" :min="1" :max="86400" placeholder="秒" style="width: 100px;" />
            <n-button size="small" type="primary" @click="applySelectedBuff" :disabled="!quickCmdPlayer || !selectedBuffId || quickCmdLoading">
              施加 Buff
            </n-button>
          </div>
          <p class="hint-text">Buff ID 来源：Terraria 官方 Wiki Buff IDs。下拉内置常用项，也可以直接输入 Wiki 上的任意 Buff ID。</p>
        </div>

        <div class="quick-cmd-group">
          <h4>时间与天气</h4>
          <div class="world-actions compact">
            <n-button size="small" @click="quickCmd('/time 12:00')" :disabled="quickCmdLoading">正午</n-button>
            <n-button size="small" @click="quickCmd('/time 00:00')" :disabled="quickCmdLoading">午夜</n-button>
            <n-button size="small" @click="quickCmd('/time 04:30')" :disabled="quickCmdLoading">黎明</n-button>
            <n-button size="small" @click="quickCmd('/time 19:30')" :disabled="quickCmdLoading">黄昏</n-button>
            <n-button size="small" @click="quickCmd('/wind 0')" :disabled="quickCmdLoading">无风</n-button>
          </div>
          <p class="hint-text">TShock 当前版本提示 `/time` 必须使用 24 小时 `hh:mm` 格式，所以这里不再发送 dawn/dusk 字符串。</p>
        </div>

        <div class="quick-cmd-group">
          <h4>世界事件（REST 支持）</h4>
          <div class="world-actions compact">
            <n-button size="small" type="error" @click="handleBloodmoon(true)" :loading="actionLoading.bloodmoonOn">触发血月</n-button>
            <n-button size="small" @click="handleBloodmoon(false)" :loading="actionLoading.bloodmoonOff">关闭血月</n-button>
            <n-button size="small" type="info" @click="handleMeteor" :loading="actionLoading.meteor">召唤陨石</n-button>
          </div>
          <p class="hint-text">官方 REST 只稳定暴露血月、陨石、保存、屠夫、自动保存等世界端点。入侵/日食/雨通常需要游戏内命令或插件，面板不再发送会失败的 rawcmd。</p>
        </div>

        <div class="quick-cmd-group">
          <h4>传送</h4>
          <n-alert type="warning" :show-icon="false">
            `/tp`、`/home`、`/spawn`、`/tpnpc` 在当前 TShock 返回 “You must use this command in-game.”，不能通过 REST 控制台可靠执行。需要游戏内管理员执行，或安装提供远程传送的 TShock 插件。
          </n-alert>
        </div>

        <div class="quick-cmd-group">
          <h4>Boss 召唤</h4>
          <n-alert type="warning" :show-icon="false">
            `/spawnmob`、`/spawnboss` 在当前 TShock 返回 “You must use this command in-game.”，说明命令需要玩家上下文。面板不再提供无效按钮。
          </n-alert>
        </div>

        <n-card v-show="quickCmdResult" size="small" style="margin-top: 12px;" title="命令输出">
          <pre class="pre-block">{{ quickCmdResult }}</pre>
        </n-card>
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
          <n-card v-show="rawCmdResult" size="small" style="margin-top: 8px;" title="命令输出">
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
const tokenTestLoading = ref(false)
const tokenTestResult = ref('')
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

// Quick commands
const quickCmdPlayer = ref('')
const quickCmdLoading = ref(false)
const quickCmdResult = ref('')
const selectedBuffId = ref<number | string | null>(5)
const customBuffDuration = ref(3600)

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
  restRestart: false,
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
    width: 250,
    render: (row: any) => {
      return h('div', { style: 'display:flex;gap:6px;' }, [
        h(NButton, { size: 'tiny', type: 'warning', onClick: () => handleKick(row.nickname) }, { default: () => '踢出' }),
        h(NButton, { size: 'tiny', type: 'error', onClick: () => handleBanPlayer(row.nickname) }, { default: () => '封禁' }),
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

const TERRARIA_BUFFS = [
  { id: 1, name: 'Obsidian Skin', zh: '黑曜石皮', type: 'Buff' },
  { id: 2, name: 'Regeneration', zh: '再生', type: 'Buff' },
  { id: 3, name: 'Swiftness', zh: '迅捷', type: 'Buff' },
  { id: 4, name: 'Gills', zh: '鱼鳃', type: 'Buff' },
  { id: 5, name: 'Ironskin', zh: '铁皮', type: 'Buff' },
  { id: 6, name: 'Mana Regeneration', zh: '魔力再生', type: 'Buff' },
  { id: 7, name: 'Magic Power', zh: '魔能', type: 'Buff' },
  { id: 8, name: 'Featherfall', zh: '羽落', type: 'Buff' },
  { id: 9, name: 'Spelunker', zh: '洞穴探险', type: 'Buff' },
  { id: 10, name: 'Invisibility', zh: '隐身', type: 'Buff' },
  { id: 11, name: 'Shine', zh: '光芒', type: 'Buff' },
  { id: 12, name: 'Night Owl', zh: '夜猫子', type: 'Buff' },
  { id: 13, name: 'Battle', zh: '战斗', type: 'Buff' },
  { id: 14, name: 'Thorns', zh: '荆棘', type: 'Buff' },
  { id: 15, name: 'Water Walking', zh: '水上漂', type: 'Buff' },
  { id: 16, name: 'Archery', zh: '箭术', type: 'Buff' },
  { id: 17, name: 'Hunter', zh: '狩猎', type: 'Buff' },
  { id: 18, name: 'Gravitation', zh: '重力', type: 'Buff' },
  { id: 26, name: 'Well Fed', zh: '酒足饭饱', type: 'Buff' },
  { id: 29, name: 'Clairvoyance', zh: '灵视', type: 'Buff' },
  { id: 48, name: 'Honey', zh: '蜂蜜', type: 'Buff' },
  { id: 87, name: 'Cozy Fire', zh: '舒适篝火', type: 'Buff' },
  { id: 89, name: 'Heart Lamp', zh: '心灯', type: 'Buff' },
  { id: 93, name: 'Ammo Box', zh: '弹药箱', type: 'Buff' },
  { id: 104, name: 'Mining', zh: '挖矿', type: 'Buff' },
  { id: 105, name: 'Heartreach', zh: '拾心', type: 'Buff' },
  { id: 106, name: 'Calm', zh: '镇静', type: 'Buff' },
  { id: 107, name: 'Builder', zh: '建筑工', type: 'Buff' },
  { id: 108, name: 'Titan', zh: '泰坦', type: 'Buff' },
  { id: 109, name: 'Flipper', zh: '脚蹼', type: 'Buff' },
  { id: 110, name: 'Summoning', zh: '召唤', type: 'Buff' },
  { id: 111, name: 'Dangersense', zh: '危险感知', type: 'Buff' },
  { id: 112, name: 'Ammo Reservation', zh: '弹药储备', type: 'Buff' },
  { id: 113, name: 'Lifeforce', zh: '生命力', type: 'Buff' },
  { id: 114, name: 'Endurance', zh: '耐力', type: 'Buff' },
  { id: 115, name: 'Rage', zh: '暴怒', type: 'Buff' },
  { id: 116, name: 'Inferno', zh: '狱火', type: 'Buff' },
  { id: 117, name: 'Wrath', zh: '怒气', type: 'Buff' },
  { id: 121, name: 'Fishing', zh: '钓鱼', type: 'Buff' },
  { id: 122, name: 'Sonar', zh: '声呐', type: 'Buff' },
  { id: 123, name: 'Crate', zh: '宝匣', type: 'Buff' },
  { id: 124, name: 'Warmth', zh: '温暖', type: 'Buff' },
  { id: 150, name: 'Bewitched', zh: '着魔', type: 'Buff' },
  { id: 257, name: 'Lucky', zh: '幸运', type: 'Buff' },
  { id: 336, name: 'Hearty Meal', zh: '丰盛大餐', type: 'Buff' },
  { id: 20, name: 'Poisoned', zh: '中毒', type: 'Debuff' },
  { id: 24, name: 'On Fire!', zh: '着火', type: 'Debuff' },
  { id: 31, name: 'Confused', zh: '困惑', type: 'Debuff' },
  { id: 39, name: 'Cursed Inferno', zh: '咒火', type: 'Debuff' },
  { id: 44, name: 'Frostburn', zh: '霜冻', type: 'Debuff' },
  { id: 69, name: 'Ichor', zh: '灵液', type: 'Debuff' },
  { id: 70, name: 'Acid Venom', zh: '酸性毒液', type: 'Debuff' },
  { id: 88, name: 'Chaos State', zh: '混沌状态', type: 'Debuff' },
  { id: 323, name: 'Hellfire', zh: '地狱火', type: 'Debuff' },
  { id: 324, name: 'Frostbite', zh: '冻伤', type: 'Debuff' },
]

const buffOptions = TERRARIA_BUFFS.map((buff) => ({
  label: `#${buff.id} ${buff.zh} / ${buff.name} (${buff.type})`,
  value: buff.id,
}))

function itemDisplayName(item: TerrariaItem) {
  return item.zh_name ? `${item.zh_name} / ${item.name}` : item.name
}

function restResponseText(data: any): string {
  const response = data?.response?.response ?? data?.response ?? data?.message ?? data?.error
  if (Array.isArray(response)) {
    return response.join('\n')
  }
  if (typeof response === 'string') {
    return response
  }
  if (response) {
    return JSON.stringify(response, null, 2)
  }
  return ''
}

function restBusinessFailed(data: any): boolean {
  if (data?.ok === false) return true
  const text = restResponseText(data).toLowerCase()
  return text.includes('invalid command')
    || text.includes('invalid syntax')
    || text.includes('not have permission')
    || text.includes('you do not have access')
    || text.includes('could not find')
    || text.includes('failed')
    || text.includes('error')
}

function notificationText(text: string) {
  return text.length > 220 ? `${text.slice(0, 220)}...` : text
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

async function handleTokenTest() {
  tokenTestLoading.value = true
  tokenTestResult.value = ''
  try {
    const resp = await tshockRestApi.tokenTest(props.serverId)
    tokenTestResult.value = JSON.stringify(resp.data, null, 2)
    notification.success('REST Token 可用', '')
  } catch (e: any) {
    tokenTestResult.value = e?.response?.data ? JSON.stringify(e.response.data, null, 2) : ''
    notification.error('REST Token 测试失败', e?.response?.data?.error || '')
  } finally {
    tokenTestLoading.value = false
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

function handleRestRestart() {
  dialog.warning({
    title: '确认 REST 重启',
    content: '确定通过 TShock REST API 重启服务器吗？在线玩家会短暂断开。',
    positiveText: '重启',
    negativeText: '取消',
    onPositiveClick: async () => {
      actionLoading.restRestart = true
      try {
        await tshockRestApi.serverRestart(props.serverId)
        notification.success('REST 重启已发送', '')
        setTimeout(refreshAll, 8000)
      } catch (e: any) {
        notification.error('REST 重启失败', e?.response?.data?.error || '')
      } finally {
        actionLoading.restRestart = false
      }
    }
  })
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

function handleBanPlayer(player: string) {
  dialog.error({
    title: '确认封禁',
    content: `确定要封禁玩家「${player}」吗？`,
    positiveText: '封禁',
    negativeText: '取消',
    onPositiveClick: async () => {
      try {
        await tshockRestApi.playerBan(props.serverId, player, '被管理员封禁')
        notification.success('已封禁玩家', player)
        loadPlayers()
        loadBans()
      } catch (e: any) {
        notification.error('封禁失败', e?.response?.data?.error || '')
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
        const message = restResponseText(resp.data)
        itemGiveResult.value = JSON.stringify(resp.data, null, 2)
        if (restBusinessFailed(resp.data)) {
          notification.error('发放失败', message || 'TShock 返回失败')
          return
        }
        notification.success('物品已发放', message || `${player} <- ${stack} x ${itemDisplayName(item)}`)
      } catch (e: any) {
        const data = e?.response?.data
        itemGiveResult.value = data ? JSON.stringify(data, null, 2) : ''
        notification.error('请求失败', data?.error || e?.message || '')
      } finally {
        itemGiveLoading.value = false
      }
    }
  })
}

function applySelectedBuff() {
  if (!quickCmdPlayer.value || !selectedBuffId.value) return
  const buffId = Number(selectedBuffId.value)
  if (!Number.isFinite(buffId) || buffId <= 0) {
    notification.error('Buff ID 无效', '请选择列表中的 Buff，或输入一个正整数 ID')
    return
  }
  quickCmd(`/gbuff ${quickCmdPlayer.value} ${buffId} ${customBuffDuration.value}`)
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
  try {
    const resp = await tshockRestApi.serverRawcmd(props.serverId, rawCmd.value)
    const data = resp.data as any
    rawCmdResult.value = restResponseText(data) || JSON.stringify(data, null, 2)
    if (restBusinessFailed(data)) {
      notification.error('命令执行失败', rawCmdResult.value)
      return
    }
    notification.success('命令已执行', notificationText(rawCmdResult.value || rawCmd.value))
    rawCmd.value = ''
  } catch (e: any) {
    notification.error('请求失败', e?.response?.data?.error || e?.message || '')
  } finally {
    actionLoading.rawcmd = false
  }
}

// ─── Quick Commands ───

async function quickCmd(cmd: string) {
  if (!cmd.trim()) return
  quickCmdLoading.value = true
  try {
    const resp = await tshockRestApi.serverRawcmd(props.serverId, cmd)
    const data = resp.data as any
    quickCmdResult.value = restResponseText(data) || JSON.stringify(data, null, 2)
    if (restBusinessFailed(data)) {
      notification.error('命令执行失败', quickCmdResult.value)
    } else {
      notification.success('命令已执行', notificationText(quickCmdResult.value || cmd))
    }
  } catch (e: any) {
    notification.error('请求失败', e?.response?.data?.error || e?.message || '')
  } finally {
    quickCmdLoading.value = false
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
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 16px;
}

.rest-card {
  border: 1px solid var(--border-color);
}

.compact-card :deep(.n-card__content) {
  padding-top: 12px;
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

.world-actions.compact {
  margin-top: 0;
  padding-top: 0;
  border-top: 0;
  align-items: center;
}

.world-actions.compact :deep(.n-button) {
  min-width: 88px;
  justify-content: center;
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
  flex-wrap: wrap;
  align-items: center;
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

.quick-cmd-group {
  margin-bottom: 12px;
  padding-bottom: 10px;
  border-bottom: 1px solid var(--border-color);
}

.quick-cmd-group:last-of-type {
  border-bottom: none;
}

.quick-cmd-group h4 {
  margin: 0 0 8px 0;
  font-size: 13px;
  color: var(--text-secondary);
}

.world-actions.compact {
  gap: 6px;
}

@media (max-width: 900px) {
  .rest-cards {
    grid-template-columns: 1fr;
  }

  .item-form {
    grid-template-columns: 1fr;
  }
}

@media (max-width: 768px) {
  .tshock-rest-panel {
    padding: 12px;
    border-radius: 8px;
  }

  .status-grid {
    grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
    gap: 8px;
  }

  .stat-item {
    padding: 8px;
  }

  .world-actions {
    gap: 6px;
  }

  .input-row {
    flex-direction: column;
  }

  .command-section h4 {
    font-size: 14px;
  }

  .sub-section-header {
    flex-wrap: wrap;
  }
}

@media (max-width: 480px) {
  .status-grid {
    grid-template-columns: 1fr 1fr;
  }
}
</style>
