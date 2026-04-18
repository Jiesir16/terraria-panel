<template>
  <div class="console-container">
    <div class="console-status-bar">
      <span :class="connected ? 'status-connected' : 'status-disconnected'">
        {{ connected ? '已连接' : '未连接' }}
      </span>
      <n-select
        v-model:value="historyLines"
        size="tiny"
        :options="historyOptions"
        style="width: 120px"
      />
      <n-button text type="primary" size="tiny" @click="handleReloadHistory">
        刷新日志
      </n-button>
      <n-button text size="tiny" @click="clearMessages">
        清空
      </n-button>
      <n-button v-if="!connected" text type="primary" size="tiny" @click="handleReconnect">
        重新连接
      </n-button>
    </div>

    <div class="console-output" ref="consoleRef">
      <div class="console-terminal">
        <div
          v-for="(message, index) in messages"
          :key="index"
          :class="getLogClass(message)"
        >
          {{ message }}
        </div>
        <div v-if="messages.length === 0" class="placeholder">
          等待服务器输出...
        </div>
      </div>
    </div>

    <div class="console-input">
      <!-- 基础命令（所有人可见） -->
      <div
        v-for="section in visibleCommandSections"
        :key="section.label"
        class="command-section"
      >
        <span class="command-label">{{ section.label }}</span>
        <n-space size="small" :wrap="true">
          <n-button
            v-for="item in section.items"
            :key="`${section.label}-${item.command}`"
            text
            :type="item.color"
            size="small"
            @click="sendCommand(item.command)"
          >
            {{ item.label }}
          </n-button>
        </n-space>
      </div>

      <div class="input-row">
        <n-input
          ref="inputRef"
          v-model:value="commandInput"
          placeholder="输入命令..."
          :on-keyup="handleKeyup"
          :disabled="!connected"
        />
        <n-button type="primary" @click="sendCurrentCommand" :disabled="!connected">
          发送
        </n-button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, nextTick, watch, onUnmounted } from 'vue'
import { NInput, NButton, NSpace, NSelect } from 'naive-ui'
import { useWebSocket } from '../../composables/useWebSocket'
import { useNotification } from '../../composables/useNotification'
import { useAuthStore } from '../../stores/auth'
import { useServersStore } from '../../stores/servers'
import { serverApi } from '../../api/server'
import { SERVER_COMMAND_SECTIONS, type CommandScope } from '../../constants/serverCommands'

interface Props {
  serverId: string
}

const props = defineProps<Props>()
const notification = useNotification()
const authStore = useAuthStore()
const serversStore = useServersStore()

const consoleRef = ref<HTMLElement>()
const inputRef = ref()
const commandInput = ref('')
const commandHistory = ref<string[]>([])
const historyIndex = ref(-1)
const historyLines = ref(200)
const historyOptions = [
  { label: '最近 100 行', value: 100 },
  { label: '最近 200 行', value: 200 },
  { label: '最近 500 行', value: 500 },
  { label: '最近 1000 行', value: 1000 }
]
const currentServer = computed(() => {
  if (serversStore.currentServer?.id === props.serverId) {
    return serversStore.currentServer
  }
  return serversStore.getServerById(props.serverId) || null
})
const isServerOwner = computed(() => currentServer.value?.created_by === authStore.user?.id)
const visibleCommandSections = computed(() => {
  const canUseScope = (scope: CommandScope) => {
    switch (scope) {
      case 'viewer':
        return true
      case 'operator':
        return authStore.isOperator
      case 'owner':
        return authStore.isAdmin || isServerOwner.value
      case 'admin':
        return authStore.isAdmin
    }
  }

  return SERVER_COMMAND_SECTIONS.filter(section => canUseScope(section.scope))
})

const { sendCommand: wsSendCommand, connected, messages, reconnect, clearMessages } = useWebSocket(
  props.serverId,
  {
    historyLines: historyLines.value,
    onMessage: () => {
      nextTick(() => {
        scrollToBottom()
      })
    },
    onError: () => {
      // Silently handle — status bar shows connection state
    }
  }
)

function handleReconnect() {
  reconnect()
  notification.success('正在重新连接...', '')
}

async function handleReloadHistory() {
  try {
    const response = await serverApi.getRecentLogs(props.serverId, historyLines.value)
    messages.value = response.data
    nextTick(() => {
      scrollToBottom()
    })
    if (!connected.value) {
      reconnect()
    }
  } catch (error: any) {
    notification.error('刷新日志失败', error?.response?.data?.error || '')
  }
}

function getLogClass(message: string): string {
  if (message.includes('error') || message.includes('Error')) {
    return 'log-error'
  }
  if (message.includes('warn') || message.includes('Warn')) {
    return 'log-warn'
  }
  if (message.includes('success') || message.includes('Success')) {
    return 'log-success'
  }
  return 'log-info'
}

function scrollToBottom() {
  if (consoleRef.value) {
    consoleRef.value.scrollTop = consoleRef.value.scrollHeight
  }
}

function sendCommand(command: string) {
  commandInput.value = command
  // For commands that typically need arguments, just fill the input
  const needsArg = ['/kick', '/ban', '/mute', '/tp', '/tphere', '/broadcast', '/give', '/gbuff', '/spawnmob', '/grow']
  if (needsArg.some(c => command === c)) {
    // Focus the input so user can add arguments
    nextTick(() => {
      inputRef.value?.focus()
    })
    return
  }
  nextTick(() => {
    sendCurrentCommand()
  })
}

function sendCurrentCommand() {
  if (!commandInput.value.trim()) {
    return
  }

  wsSendCommand(commandInput.value)
  commandHistory.value.push(commandInput.value)
  commandInput.value = ''
  historyIndex.value = -1
}

function handleKeyup(e: KeyboardEvent) {
  if (e.key === 'Enter') {
    sendCurrentCommand()
  } else if (e.key === 'ArrowUp') {
    historyIndex.value = Math.min(
      historyIndex.value + 1,
      commandHistory.value.length - 1
    )
    if (historyIndex.value >= 0) {
      commandInput.value = commandHistory.value[commandHistory.value.length - 1 - historyIndex.value]
    }
  } else if (e.key === 'ArrowDown') {
    historyIndex.value = Math.max(historyIndex.value - 1, -1)
    if (historyIndex.value >= 0) {
      commandInput.value = commandHistory.value[commandHistory.value.length - 1 - historyIndex.value]
    } else {
      commandInput.value = ''
    }
  }
}

onUnmounted(() => {
  // Cleanup is handled by useWebSocket
})

watch(historyLines, () => {
  handleReloadHistory()
})
</script>

<style scoped>
.console-container {
  display: flex;
  flex-direction: column;
  height: 600px;
  background-color: var(--bg-card);
  border: 1px solid var(--border-color);
  border-radius: 12px;
  overflow: hidden;
  transition: background-color 0.3s, border-color 0.3s;
}

.console-status-bar {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 12px;
  background-color: var(--bg-body);
  border-bottom: 1px solid var(--border-color);
  font-size: 12px;
}

.status-connected {
  color: #50C878;
}

.status-connected::before {
  content: '';
  display: inline-block;
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background-color: #50C878;
  margin-right: 6px;
}

.status-disconnected {
  color: #FF6B6B;
}

.status-disconnected::before {
  content: '';
  display: inline-block;
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background-color: #FF6B6B;
  margin-right: 6px;
}

.console-output {
  flex: 1;
  overflow-y: auto;
  padding: 0;
}

.console-terminal {
  background-color: var(--console-bg);
  color: #50C878;
  font-family: "JetBrains Mono", "Fira Code", monospace;
  padding: 12px;
  white-space: pre-wrap;
  word-wrap: break-word;
  line-height: 1.5;
  font-size: 12px;
  min-height: 100%;
}

.placeholder {
  color: var(--text-muted);
  text-align: center;
  padding: 20px;
}

.console-input {
  border-top: 1px solid var(--border-color);
  padding: 10px 12px;
  display: flex;
  flex-direction: column;
  gap: 6px;
  background-color: var(--bg-card);
}

.command-section {
  display: flex;
  align-items: center;
  gap: 8px;
}

.command-label {
  font-size: 11px;
  color: var(--text-muted, #808080);
  min-width: 28px;
  font-weight: 600;
}

.input-row {
  display: flex;
  gap: 8px;
}

.input-row :deep(.n-input__input-el) {
  background-color: var(--bg-input);
  color: var(--text-primary);
  border-color: var(--border-color);
}

.log-error {
  color: #FF6B6B;
}

.log-warn {
  color: #FFB347;
}

.log-success {
  color: #50C878;
}

.log-info {
  color: #64B5F6;
}
</style>
