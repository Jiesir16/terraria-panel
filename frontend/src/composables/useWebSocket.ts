import { ref, onMounted, onUnmounted } from 'vue'
import { useAuthStore } from '../stores/auth'

export interface UseWebSocketOptions {
  onMessage?: (data: string) => void
  onError?: (error: Event) => void
  onOpen?: () => void
  onClose?: () => void
  reconnectAttempts?: number
  reconnectDelay?: number
}

export function useWebSocket(
  serverId: string,
  options: UseWebSocketOptions = {}
) {
  const authStore = useAuthStore()
  const ws = ref<WebSocket | null>(null)
  const connected = ref(false)
  const messages = ref<string[]>([])
  let reconnectCount = 0
  let reconnectTimeout: ReturnType<typeof setTimeout> | null = null

  const reconnectAttempts = options.reconnectAttempts ?? 5
  const reconnectDelay = options.reconnectDelay ?? 3000

  function connect() {
    if (ws.value?.readyState === WebSocket.OPEN) {
      return
    }

    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
    const url = `${protocol}//${window.location.host}/api/servers/${serverId}/console`

    ws.value = new WebSocket(url)

    ws.value.onopen = () => {
      connected.value = true
      reconnectCount = 0
      if (authStore.token) {
        ws.value?.send(JSON.stringify({ type: 'auth', token: authStore.token }))
      }
      options.onOpen?.()
    }

    ws.value.onmessage = (event) => {
      const message = event.data
      messages.value.push(message)
      options.onMessage?.(message)
    }

    ws.value.onerror = (error) => {
      connected.value = false
      options.onError?.(error)
    }

    ws.value.onclose = () => {
      connected.value = false
      options.onClose?.()

      if (reconnectCount < reconnectAttempts) {
        reconnectCount++
        reconnectTimeout = setTimeout(() => {
          connect()
        }, reconnectDelay)
      }
    }
  }

  function sendCommand(command: string) {
    if (ws.value?.readyState === WebSocket.OPEN) {
      ws.value.send(JSON.stringify({ type: 'command', data: command }))
    }
  }

  function clearMessages() {
    messages.value = []
  }

  function disconnect() {
    if (reconnectTimeout) {
      clearTimeout(reconnectTimeout)
    }
    if (ws.value) {
      ws.value.close()
      ws.value = null
    }
    connected.value = false
  }

  onMounted(() => {
    connect()
  })

  onUnmounted(() => {
    disconnect()
  })

  return {
    connected,
    messages,
    sendCommand,
    clearMessages,
    disconnect,
    connect
  }
}
