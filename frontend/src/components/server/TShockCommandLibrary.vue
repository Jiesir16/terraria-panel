<template>
  <div class="tshock-command-library">
    <div class="section-header">
      <div>
        <h3>TShock 命令库</h3>
        <p class="hint-text">
          基于 TShock REST 与 AllowServer 语义整理；仅游戏内命令只展示不发送。
        </p>
      </div>
      <n-button text type="primary" :loading="contextLoading" @click="refreshContext">
        刷新上下文
      </n-button>
    </div>

    <n-alert v-if="setupMessage" type="warning" :show-icon="true" class="setup-alert">
      {{ setupMessage }}
    </n-alert>

    <div class="library-layout">
      <n-card class="command-sidebar" title="筛选命令">
        <n-space vertical size="small">
          <n-input v-model:value="searchText" placeholder="搜索命令、权限或说明" clearable />
          <n-select v-model:value="selectedCategory" :options="categoryOptions" />
          <n-select v-model:value="selectedExecution" :options="executionOptions" />
          <n-select v-model:value="selectedRisk" :options="riskOptions" />
          <div class="switch-row">
            <span>只看可执行</span>
            <n-switch v-model:value="onlyExecutable" size="small" />
          </div>
        </n-space>

        <n-divider />

        <div class="command-count">{{ filteredCommands.length }} 条命令</div>
        <div class="command-list">
          <button
            v-for="command in filteredCommands"
            :key="command.id"
            type="button"
            class="command-item"
            :class="{ active: command.id === selectedCommandId, blocked: command.execution === 'in_game' }"
            @click="selectCommand(command.id)"
          >
            <span class="command-title-line">
              <strong>{{ command.title }}</strong>
              <n-tag size="tiny" :type="riskTagType(command.risk)">
                {{ TSHOCK_COMMAND_RISK_LABELS[command.risk] }}
              </n-tag>
            </span>
            <span class="command-code">{{ command.command }}</span>
            <span class="command-desc">{{ command.description }}</span>
          </button>
        </div>
      </n-card>

      <n-card v-if="selectedCommand" class="command-detail">
        <template #header>
          <div class="detail-title">
            <span>{{ selectedCommand.title }}</span>
            <code>{{ selectedCommand.command }}</code>
          </div>
        </template>

        <n-spin :show="setupLoading || executing">
          <n-space class="tag-row" size="small">
            <n-tag size="small">{{ TSHOCK_COMMAND_CATEGORY_LABELS[selectedCommand.category] }}</n-tag>
            <n-tag size="small" :type="executionTagType(selectedCommand.execution)">
              {{ TSHOCK_COMMAND_EXECUTION_LABELS[selectedCommand.execution] }}
            </n-tag>
            <n-tag size="small" :type="selectedCommand.allowServer ? 'success' : 'warning'">
              {{ selectedCommand.allowServer ? '允许远程执行' : '需要游戏内玩家' }}
            </n-tag>
            <n-tag size="small" :type="riskTagType(selectedCommand.risk)">
              {{ TSHOCK_COMMAND_RISK_LABELS[selectedCommand.risk] }}
            </n-tag>
          </n-space>

          <p class="description">{{ selectedCommand.description }}</p>

          <div class="meta-list">
            <div v-if="selectedCommand.permission">
              <span>命令权限</span>
              <code>{{ selectedCommand.permission }}</code>
            </div>
            <div v-if="selectedCommand.restPermission">
              <span>REST 权限</span>
              <code>{{ selectedCommand.restPermission }}</code>
            </div>
            <div v-if="selectedCommand.aliases?.length">
              <span>别名</span>
              <code>{{ selectedCommand.aliases.join(', ') }}</code>
            </div>
          </div>

          <n-alert v-if="selectedCommand.execution === 'in_game'" type="warning" :show-icon="true" class="inline-alert">
            {{ selectedCommand.disabledReason || '该命令需要游戏内玩家上下文，面板不会发送。' }}
          </n-alert>

          <n-divider />

          <n-form v-if="selectedCommand.params?.length" label-placement="top" class="param-form">
            <n-form-item v-for="param in selectedCommand.params" :key="param.key" :label="fieldLabel(param)">
              <n-select
                v-if="param.type === 'player'"
                :value="fieldValue(param.key)"
                :options="playerOptions"
                :loading="playersLoading"
                filterable
                tag
                clearable
                :placeholder="param.placeholder || '选择在线玩家，或输入玩家名'"
                @update:value="value => setFieldValue(param.key, value)"
              />
              <n-select
                v-else-if="param.type === 'group'"
                :value="fieldValue(param.key)"
                :options="groupOptions"
                :loading="groupsLoading"
                filterable
                tag
                clearable
                :placeholder="param.placeholder || '选择或输入 TShock 用户组'"
                @update:value="value => setFieldValue(param.key, value)"
              />
              <n-select
                v-else-if="param.type === 'item'"
                :value="fieldValue(param.key)"
                :options="itemOptions"
                :loading="itemsLoading"
                filterable
                clearable
                :placeholder="param.placeholder || '选择物品'"
                @update:value="value => setFieldValue(param.key, value)"
              />
              <n-select
                v-else-if="param.type === 'buff'"
                :value="fieldValue(param.key)"
                :options="buffOptions"
                filterable
                tag
                clearable
                :placeholder="param.placeholder || '选择 Buff / Debuff，或输入 ID'"
                @update:value="value => setFieldValue(param.key, value)"
              />
              <n-select
                v-else-if="param.type === 'permission'"
                :value="fieldValue(param.key)"
                :options="permissionOptions"
                multiple
                filterable
                tag
                clearable
                :placeholder="param.placeholder || '选择或输入权限节点'"
                @update:value="value => setFieldValue(param.key, value)"
              />
              <n-select
                v-else-if="param.type === 'select' || param.type === 'time'"
                :value="fieldValue(param.key)"
                :options="selectParamOptions(param)"
                filterable
                clearable
                :placeholder="param.placeholder || '请选择'"
                @update:value="value => setFieldValue(param.key, value)"
              />
              <n-input-number
                v-else-if="param.type === 'number'"
                :value="fieldValue(param.key)"
                :min="param.min"
                :max="param.max"
                :placeholder="param.placeholder || '输入数字'"
                @update:value="value => setFieldValue(param.key, value)"
              />
              <div v-else-if="param.type === 'boolean'" class="boolean-field">
                <n-switch
                  :value="Boolean(fieldValue(param.key))"
                  @update:value="value => setFieldValue(param.key, value)"
                />
                <span>{{ fieldValue(param.key) ? '开启' : '关闭' }}</span>
              </div>
              <n-input
                v-else
                :value="textFieldValue(param.key)"
                :placeholder="param.placeholder || '输入参数'"
                clearable
                @update:value="value => setFieldValue(param.key, value)"
              />
            </n-form-item>
          </n-form>
          <div v-else class="empty-note">该命令不需要参数。</div>

          <n-card size="small" title="执行预览" class="preview-card">
            <pre>{{ commandPreview }}</pre>
          </n-card>

          <div class="action-row">
            <n-button
              type="primary"
              :disabled="!canExecuteSelected"
              :loading="executing"
              @click="executeCommand"
            >
              执行命令
            </n-button>
            <n-button secondary @click="resetCurrentParams">
              重置参数
            </n-button>
          </div>

          <n-card v-if="resultText" size="small" title="最近结果" class="result-card">
            <pre>{{ resultText }}</pre>
          </n-card>
        </n-spin>
      </n-card>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import {
  NAlert,
  NButton,
  NCard,
  NDivider,
  NForm,
  NFormItem,
  NInput,
  NInputNumber,
  NSelect,
  NSpace,
  NSpin,
  NSwitch,
  NTag,
  useDialog,
} from 'naive-ui'
import { tshockRestApi } from '../../api/tshockRest'
import type { TerrariaItem, TShockRestPlayer } from '../../api/tshockRest'
import { useNotification } from '../../composables/useNotification'
import { TERRARIA_BUFF_OPTIONS } from '../../constants/terrariaBuffs'
import {
  TSHOCK_COMMAND_CATALOG,
  TSHOCK_COMMAND_CATEGORY_LABELS,
  TSHOCK_COMMAND_EXECUTION_LABELS,
  TSHOCK_COMMAND_RISK_LABELS,
} from '../../constants/tshockCommandCatalog'
import type {
  TShockCommandCategory,
  TShockCommandDefinition,
  TShockCommandExecution,
  TShockCommandParam,
  TShockCommandRisk,
} from '../../constants/tshockCommandCatalog'
import { ALL_PERMISSION_KEYS } from '../../constants/tshockPermissions'

type CommandParamScalar = string | number | boolean
type CommandParamValue = CommandParamScalar | CommandParamScalar[] | null
type CommandParamValues = Record<string, CommandParamValue>
type SelectOption = { label: string; value: string | number }

const props = defineProps<{
  serverId: string
}>()

const dialog = useDialog()
const notification = useNotification()

const searchText = ref('')
const selectedCategory = ref<TShockCommandCategory | 'all'>('all')
const selectedExecution = ref<TShockCommandExecution | 'all'>('all')
const selectedRisk = ref<TShockCommandRisk | 'all'>('all')
const onlyExecutable = ref(true)
const selectedCommandId = ref(TSHOCK_COMMAND_CATALOG.find(command => command.execution !== 'in_game')?.id || TSHOCK_COMMAND_CATALOG[0]?.id || '')
const paramValues = ref<CommandParamValues>({})
const resultText = ref('')
const setupLoading = ref(false)
const setupMessage = ref('')
const executing = ref(false)
const playersLoading = ref(false)
const groupsLoading = ref(false)
const itemsLoading = ref(false)
const players = ref<TShockRestPlayer[]>([])
const groupOptions = ref<SelectOption[]>([])
const items = ref<TerrariaItem[]>([])
const buffOptions = TERRARIA_BUFF_OPTIONS

const categories = Object.keys(TSHOCK_COMMAND_CATEGORY_LABELS) as TShockCommandCategory[]
const executions = Object.keys(TSHOCK_COMMAND_EXECUTION_LABELS) as TShockCommandExecution[]
const risks = Object.keys(TSHOCK_COMMAND_RISK_LABELS) as TShockCommandRisk[]

const categoryOptions = computed<SelectOption[]>(() => [
  { label: '全部分类', value: 'all' },
  ...categories.map(category => ({ label: TSHOCK_COMMAND_CATEGORY_LABELS[category], value: category })),
])

const executionOptions = computed<SelectOption[]>(() => [
  { label: '全部执行方式', value: 'all' },
  ...executions.map(execution => ({ label: TSHOCK_COMMAND_EXECUTION_LABELS[execution], value: execution })),
])

const riskOptions = computed<SelectOption[]>(() => [
  { label: '全部风险', value: 'all' },
  ...risks.map(risk => ({ label: TSHOCK_COMMAND_RISK_LABELS[risk], value: risk })),
])

const permissionOptions = computed<SelectOption[]>(() => ALL_PERMISSION_KEYS.map(permission => ({
  label: permission,
  value: permission,
})))

const playerOptions = computed<SelectOption[]>(() => players.value
  .map(player => player.nickname || player.username)
  .filter((name): name is string => typeof name === 'string' && name.trim().length > 0)
  .map(name => ({ label: name, value: name })))

const itemOptions = computed<SelectOption[]>(() => items.value.map(item => ({
  label: `#${item.id} ${item.zh_name ? `${item.zh_name} / ${item.name}` : item.name}`,
  value: item.id,
})))

const filteredCommands = computed(() => {
  const keyword = searchText.value.trim().toLowerCase()
  return TSHOCK_COMMAND_CATALOG.filter(command => {
    if (onlyExecutable.value && command.execution === 'in_game') return false
    if (selectedCategory.value !== 'all' && command.category !== selectedCategory.value) return false
    if (selectedExecution.value !== 'all' && command.execution !== selectedExecution.value) return false
    if (selectedRisk.value !== 'all' && command.risk !== selectedRisk.value) return false
    if (!keyword) return true

    const haystack = [
      command.title,
      command.command,
      command.description,
      command.permission,
      command.restPermission,
      ...(command.aliases || []),
    ].filter(Boolean).join(' ').toLowerCase()

    return haystack.includes(keyword)
  })
})

const selectedCommand = computed<TShockCommandDefinition | null>(() => {
  return filteredCommands.value.find(command => command.id === selectedCommandId.value) || filteredCommands.value[0] || null
})

const commandPreview = computed(() => {
  if (!selectedCommand.value) return ''
  if (selectedCommand.value.execution === 'rest' && selectedCommand.value.action !== 'rawcmd') {
    return `${selectedCommand.value.title} (${TSHOCK_COMMAND_EXECUTION_LABELS.rest})`
  }
  return buildRawCommand(selectedCommand.value)
})

const contextLoading = computed(() => setupLoading.value || playersLoading.value || groupsLoading.value || itemsLoading.value)

const canExecuteSelected = computed(() => {
  if (!selectedCommand.value) return false
  if (selectedCommand.value.execution === 'in_game') return false
  return !missingRequiredParam(selectedCommand.value)
})

watch(filteredCommands, (commands) => {
  if (commands.length > 0 && !commands.some(command => command.id === selectedCommandId.value)) {
    selectedCommandId.value = commands[0].id
  }
})

watch(selectedCommand, (command) => {
  if (!command) return
  resetParams(command)
  resultText.value = ''
  loadContextForCommand(command)
}, { immediate: true })

onMounted(async () => {
  await checkRestSetup()
  if (selectedCommand.value) {
    await loadContextForCommand(selectedCommand.value)
  }
})

function selectCommand(id: string) {
  selectedCommandId.value = id
}

function fieldLabel(param: TShockCommandParam) {
  return param.required ? `${param.label} *` : param.label
}

function fieldValue(key: string): any {
  return paramValues.value[key]
}

function textFieldValue(key: string) {
  const value = paramValues.value[key]
  return typeof value === 'string' ? value : ''
}

function setFieldValue(key: string, value: any) {
  paramValues.value[key] = value as CommandParamValue
}

function selectParamOptions(param: TShockCommandParam): any[] {
  return param.options || []
}

function resetCurrentParams() {
  if (selectedCommand.value) {
    resetParams(selectedCommand.value)
  }
}

function resetParams(command: TShockCommandDefinition) {
  const values: CommandParamValues = {}
  for (const param of command.params || []) {
    if (param.type === 'boolean') {
      values[param.key] = Boolean(param.default)
    } else if (param.type === 'permission') {
      values[param.key] = typeof param.default === 'string' ? [param.default] : []
    } else if (param.default !== undefined) {
      values[param.key] = param.default
    } else if (param.type === 'number' || param.type === 'item' || param.type === 'buff') {
      values[param.key] = null
    } else {
      values[param.key] = ''
    }
  }
  paramValues.value = values
}

async function checkRestSetup() {
  setupLoading.value = true
  setupMessage.value = ''
  try {
    const resp = await tshockRestApi.setup(props.serverId)
    if (!resp.data.ready) {
      setupMessage.value = resp.data.message || 'REST 配置已写入，需要重启服务器后生效。'
    }
  } catch (e: any) {
    setupMessage.value = e?.response?.data?.error || '无法检查 REST 配置，请确认服务器已初始化。'
  } finally {
    setupLoading.value = false
  }
}

async function refreshContext() {
  await Promise.all([loadPlayers(), loadGroups(), maybeLoadItems(selectedCommand.value)])
}

async function loadContextForCommand(command: TShockCommandDefinition) {
  const tasks: Promise<void>[] = []
  if (hasParamType(command, 'player')) tasks.push(loadPlayers())
  if (hasParamType(command, 'group')) tasks.push(loadGroups())
  if (hasParamType(command, 'item')) tasks.push(maybeLoadItems(command))
  await Promise.all(tasks)
}

function hasParamType(command: TShockCommandDefinition, type: TShockCommandParam['type']) {
  return (command.params || []).some(param => param.type === type)
}

async function loadPlayers() {
  playersLoading.value = true
  try {
    const resp = await tshockRestApi.playerList(props.serverId)
    players.value = extractPlayers(resp.data)
  } catch (e: any) {
    notification.error('获取玩家列表失败', e?.response?.data?.error || '')
  } finally {
    playersLoading.value = false
  }
}

function extractPlayers(data: any): TShockRestPlayer[] {
  const rawPlayers = data?.players ?? data?.response?.players ?? data?.response ?? []
  return Array.isArray(rawPlayers) ? rawPlayers : []
}

async function loadGroups() {
  groupsLoading.value = true
  try {
    const resp = await tshockRestApi.groupList(props.serverId)
    const names = extractGroupNames(resp.data)
    groupOptions.value = names.map(name => ({ label: name, value: name }))
  } catch {
    const names = Array.from(new Set(players.value.map(player => player.group).filter(Boolean)))
    groupOptions.value = names.map(name => ({ label: name, value: name }))
  } finally {
    groupsLoading.value = false
  }
}

function extractGroupNames(data: any): string[] {
  const rawGroups = data?.groups ?? data?.response?.groups ?? data?.response ?? []
  if (!Array.isArray(rawGroups)) return []
  return rawGroups
    .map((group: any) => {
      if (typeof group === 'string') return group
      return group?.name ?? group?.Name ?? group?.group ?? group?.Group ?? group?.GroupName ?? group?.groupname
    })
    .filter((name: any): name is string => typeof name === 'string' && name.trim().length > 0)
}

async function maybeLoadItems(command: TShockCommandDefinition | null) {
  if (!command || !hasParamType(command, 'item')) return
  if (items.value.length > 0) return
  await loadItems()
}

async function loadItems() {
  itemsLoading.value = true
  try {
    const resp = await tshockRestApi.itemList(props.serverId, undefined, 10000)
    items.value = resp.data.items || []
  } catch (e: any) {
    notification.error('获取物品清单失败', e?.response?.data?.error || '')
  } finally {
    itemsLoading.value = false
  }
}

function missingRequiredParam(command: TShockCommandDefinition) {
  return (command.params || []).find(param => param.required && isEmptyValue(paramValues.value[param.key]))
}

function isEmptyValue(value: CommandParamValue | undefined) {
  if (Array.isArray(value)) return value.length === 0
  if (typeof value === 'string') return value.trim().length === 0
  return value === null || value === undefined
}

function textValue(key: string) {
  const value = paramValues.value[key]
  if (Array.isArray(value)) return value.join(',')
  if (value === null || value === undefined) return ''
  return String(value).trim()
}

function optionalTextValue(key: string) {
  const value = textValue(key)
  return value.length > 0 ? value : undefined
}

function boolValue(key: string) {
  return Boolean(paramValues.value[key])
}

function numberValue(key: string, fallback: number) {
  const value = paramValues.value[key]
  const numeric = Number(value)
  return Number.isFinite(numeric) ? numeric : fallback
}

function permissionValue(key: string) {
  const value = paramValues.value[key]
  if (Array.isArray(value)) return value.join(',')
  return optionalTextValue(key)
}

function buildRawCommand(command: TShockCommandDefinition) {
  let template = command.rawTemplate || command.command
  for (const param of command.params || []) {
    const value = formatCommandValue(param, paramValues.value[param.key])
    template = template.replace(new RegExp(`\\{${escapeRegExp(param.key)}\\}`, 'g'), value)
  }
  return template.replace(/\s+/g, ' ').trim()
}

function formatCommandValue(param: TShockCommandParam, value: CommandParamValue | undefined) {
  if (isEmptyValue(value)) return ''
  if (Array.isArray(value)) return value.join(',')
  const raw = String(value).trim()
  if (param.quote === false) return raw
  if (param.type === 'player' || param.type === 'group' || param.type === 'text') {
    return quoteCommandArg(raw)
  }
  return raw
}

function quoteCommandArg(value: string) {
  return `"${value.replace(/\\/g, '\\\\').replace(/"/g, '\\"')}"`
}

function escapeRegExp(value: string) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
}

function executeCommand() {
  if (!selectedCommand.value) return
  const command = selectedCommand.value
  const missing = missingRequiredParam(command)
  if (missing) {
    notification.warning('缺少必填参数', missing.label)
    return
  }
  if (command.execution === 'in_game') {
    notification.warning('命令未发送', command.disabledReason || '该命令需要游戏内玩家上下文。')
    return
  }
  if (command.risk === 'danger') {
    dialog.warning({
      title: '确认执行危险命令',
      content: `${command.title} 会直接影响服务器或持久数据，确认继续？`,
      positiveText: '执行',
      negativeText: '取消',
      onPositiveClick: () => runSelectedCommand(command),
    })
    return
  }
  runSelectedCommand(command)
}

async function runSelectedCommand(command: TShockCommandDefinition) {
  executing.value = true
  resultText.value = ''
  try {
    const resp = await executeAction(command)
    const data = resp?.data ?? resp
    resultText.value = restResponseText(data) || JSON.stringify(data, null, 2)
    if (restBusinessFailed(data)) {
      notification.error('命令执行失败', notificationText(resultText.value))
      return
    }
    notification.success('命令已执行', notificationText(resultText.value || commandPreview.value || command.title))
  } catch (e: any) {
    resultText.value = e?.response?.data ? JSON.stringify(e.response.data, null, 2) : ''
    notification.error('请求失败', e?.response?.data?.error || e?.message || '')
  } finally {
    executing.value = false
  }
}

async function executeAction(command: TShockCommandDefinition) {
  switch (command.action) {
    case 'broadcast':
      return tshockRestApi.serverBroadcast(props.serverId, textValue('message'))
    case 'serverReload':
      return tshockRestApi.serverReload(props.serverId)
    case 'serverRestart':
      return tshockRestApi.serverRestart(props.serverId)
    case 'serverOff':
      return tshockRestApi.serverOff(props.serverId, optionalTextValue('message'), boolValue('nosave'))
    case 'playerKick':
      return tshockRestApi.playerKick(props.serverId, textValue('player'), optionalTextValue('reason'))
    case 'playerBan':
      return tshockRestApi.playerBan(props.serverId, textValue('player'), optionalTextValue('reason'))
    case 'playerKill':
      return tshockRestApi.playerKill(props.serverId, textValue('player'))
    case 'playerMute':
      return tshockRestApi.playerMute(props.serverId, textValue('player'))
    case 'playerUnmute':
      return tshockRestApi.playerUnmute(props.serverId, textValue('player'))
    case 'userCreate':
      return tshockRestApi.userCreate(props.serverId, textValue('user'), textValue('password'), optionalTextValue('group'))
    case 'userUpdate':
      return tshockRestApi.userUpdate(props.serverId, textValue('user'), optionalTextValue('password'), optionalTextValue('group'))
    case 'userDestroy':
      return tshockRestApi.userDestroy(props.serverId, textValue('user'))
    case 'groupCreate':
      return tshockRestApi.groupCreate(props.serverId, textValue('group'), optionalTextValue('parent'), permissionValue('permissions'))
    case 'groupUpdate':
      return tshockRestApi.groupUpdate(props.serverId, textValue('group'), optionalTextValue('parent'), permissionValue('permissions'))
    case 'groupDestroy':
      return tshockRestApi.groupDestroy(props.serverId, textValue('group'))
    case 'banCreate':
      return tshockRestApi.banCreate(props.serverId, textValue('identifier'), optionalTextValue('reason'), optionalTextValue('duration'))
    case 'banDestroy':
      return tshockRestApi.banDestroy(props.serverId, textValue('ticket'))
    case 'worldSave':
      return tshockRestApi.worldSave(props.serverId)
    case 'worldButcher':
      return tshockRestApi.worldButcher(props.serverId, boolValue('kill_friendly'))
    case 'worldBloodmoon':
      return tshockRestApi.worldBloodmoon(props.serverId, boolValue('state'))
    case 'worldMeteor':
      return tshockRestApi.worldMeteor(props.serverId)
    case 'worldAutosave':
      return tshockRestApi.worldAutosave(props.serverId, boolValue('state'))
    case 'itemGive':
      return tshockRestApi.itemGive(props.serverId, {
        player: textValue('player'),
        item_id: numberValue('item', 0),
        stack: numberValue('stack', 1),
      })
    case 'rawcmd':
      return tshockRestApi.serverRawcmd(props.serverId, buildRawCommand(command))
    default:
      return tshockRestApi.serverRawcmd(props.serverId, buildRawCommand(command))
  }
}

function restResponseText(data: any): string {
  const response = data?.response?.response ?? data?.response ?? data?.message ?? data?.error
  if (Array.isArray(response)) return response.join('\n')
  if (typeof response === 'string') return response
  if (response) return JSON.stringify(response, null, 2)
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

function riskTagType(risk: TShockCommandRisk) {
  if (risk === 'low') return 'success'
  if (risk === 'medium') return 'warning'
  return 'error'
}

function executionTagType(execution: TShockCommandExecution) {
  if (execution === 'rest') return 'info'
  if (execution === 'rawcmd') return 'warning'
  return 'default'
}
</script>

<style scoped>
.tshock-command-library {
  background-color: var(--bg-card);
  border: 1px solid var(--border-color);
  border-radius: 12px;
  padding: 20px;
}

.section-header {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  align-items: flex-start;
  margin-bottom: 16px;
}

.section-header h3 {
  margin: 0 0 6px;
}

.hint-text,
.empty-note {
  color: var(--text-secondary);
  font-size: 13px;
  margin: 0;
}

.setup-alert {
  margin-bottom: 12px;
}

.library-layout {
  display: grid;
  grid-template-columns: minmax(280px, 360px) minmax(0, 1fr);
  gap: 16px;
}

.command-sidebar,
.command-detail {
  min-width: 0;
}

.switch-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  color: var(--text-secondary);
  font-size: 13px;
}

.command-count {
  color: var(--text-secondary);
  font-size: 12px;
  margin-bottom: 8px;
}

.command-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
  max-height: 720px;
  overflow: auto;
  padding-right: 4px;
}

.command-item {
  width: 100%;
  border: 1px solid var(--border-color);
  border-radius: 10px;
  background: var(--bg-color);
  color: var(--text-primary);
  cursor: pointer;
  padding: 10px;
  text-align: left;
  transition: border-color 0.15s ease, background 0.15s ease;
}

.command-item:hover,
.command-item.active {
  border-color: var(--primary-color);
  background: rgba(24, 160, 88, 0.08);
}

.command-item.blocked {
  opacity: 0.72;
}

.command-title-line {
  display: flex;
  justify-content: space-between;
  gap: 8px;
  align-items: center;
}

.command-code,
.command-desc {
  display: block;
  margin-top: 4px;
  font-size: 12px;
}

.command-code {
  color: var(--primary-color);
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
}

.command-desc {
  color: var(--text-secondary);
  line-height: 1.4;
}

.detail-title {
  display: flex;
  gap: 10px;
  align-items: center;
  flex-wrap: wrap;
}

.detail-title code,
.meta-list code {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 12px;
}

.tag-row {
  margin-bottom: 12px;
}

.description {
  color: var(--text-primary);
  margin: 0 0 12px;
}

.meta-list {
  display: grid;
  gap: 6px;
  color: var(--text-secondary);
  font-size: 13px;
}

.meta-list div {
  display: flex;
  gap: 8px;
  align-items: center;
  flex-wrap: wrap;
}

.inline-alert {
  margin-top: 12px;
}

.param-form {
  max-width: 720px;
}

.boolean-field {
  display: flex;
  align-items: center;
  gap: 10px;
  color: var(--text-secondary);
}

.preview-card,
.result-card {
  margin-top: 14px;
}

.preview-card pre,
.result-card pre {
  margin: 0;
  white-space: pre-wrap;
  word-break: break-word;
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 12px;
}

.action-row {
  display: flex;
  gap: 10px;
  margin-top: 14px;
}

@media (max-width: 900px) {
  .library-layout {
    grid-template-columns: 1fr;
  }

  .command-list {
    max-height: 360px;
  }
}
</style>
