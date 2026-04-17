<template>
  <div class="console-container">
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
      <n-space>
        <n-button text type="primary" size="small" @click="sendCommand('/save')">
          /save
        </n-button>
        <n-button text type="primary" size="small" @click="sendCommand('/kick')">
          /kick
        </n-button>
        <n-button text type="primary" size="small" @click="sendCommand('/ban')">
          /ban
        </n-button>
        <n-button text type="primary" size="small" @click="sendCommand('/who')">
          /who
        </n-button>
        <n-button text type="primary" size="small" @click="sendCommand('/time day')">
          Day
        </n-button>
        <n-button text type="primary" size="small" @click="sendCommand('/time night')">
          Night
        </n-button>
      </n-space>

      <div class="input-row">
        <n-input
          ref="inputRef"
          v-model:value="commandInput"
          placeholder="输入命令..."
          :on-keyup="handleKeyup"
        />
        <n-button type="primary" @click="sendCurrentCommand">
          发送
        </n-button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, nextTick, onUnmounted } from 'vue'
import { NInput, NButton, NSpace } from 'naive-ui'
import { useWebSocket } from '../../composables/useWebSocket'
import { useNotification } from '../../composables/useNotification'

interface Props {
  serverId: string
}

const props = defineProps<Props>()
const notification = useNotification()

const consoleRef = ref<HTMLElement>()
const inputRef = ref()
const commandInput = ref('')
const messages = ref<string[]>([])
const commandHistory = ref<string[]>([])
const historyIndex = ref(-1)

const { sendCommand: wsSendCommand } = useWebSocket(
  props.serverId,
  {
    onMessage: (data) => {
      messages.value.push(data)
      nextTick(() => {
        scrollToBottom()
      })
    },
    onError: () => {
      notification.error('连接错误', '无法连接到 WebSocket')
    }
  }
)

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

onMounted(() => {
  scrollToBottom()
})

onUnmounted(() => {
  // Cleanup is handled by useWebSocket
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
}

.placeholder {
  color: var(--text-muted);
  text-align: center;
  padding: 20px;
}

.console-input {
  border-top: 1px solid var(--border-color);
  padding: 12px;
  display: flex;
  flex-direction: column;
  gap: 8px;
  background-color: var(--bg-card);
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
