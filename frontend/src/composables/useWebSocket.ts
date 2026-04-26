import { ref, onMounted, onUnmounted } from 'vue'
import { useAuthStore } from '../stores/auth'

export interface UseWebSocketOptions {
  onMessage?: (data: string) => void
  onError?: (error: Event) => void
  onOpen?: () => void
  onClose?: () => void
  reconnectAttempts?: number
  reconnectDelay?: number
  historyLines?: number
  /** If false, don't auto-connect on mount */
  autoConnect?: boolean
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
  let stopped = false

  const reconnectAttempts = options.reconnectAttempts ?? 3
  const reconnectDelay = options.reconnectDelay ?? 3000
  const historyLines = options.historyLines ?? 200
  const autoConnect = options.autoConnect ?? true

  function connect() {
    if (stopped) return
    if (ws.value?.readyState === WebSocket.OPEN || ws.value?.readyState === WebSocket.CONNECTING) {
      return
    }

    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
    // Pass JWT token as query parameter — backend expects ?token=xxx
    const token = authStore.token || ''
    // In dev mode, connect directly to backend (port 3000) to avoid Vite proxy issues;
    // in production, the frontend is served by the backend so use same host
    const wsHost = import.meta.env.DEV ? `${window.location.hostname}:3000` : window.location.host
    const url = `${protocol}//${wsHost}/api/servers/${serverId}/console?token=${encodeURIComponent(token)}&history=${historyLines}`

    try {
      ws.value = new WebSocket(url)
    } catch {
      return
    }

    ws.value.onopen = () => {
      connected.value = true
      reconnectCount = 0
      options.onOpen?.()
    }

    ws.value.onmessage = (event) => {
      const raw = event.data
      // Backend sends JSON: {"type":"log","data":"..."}
      // Parse it and extract the log line
      let message = raw
      try {
        const parsed = JSON.parse(raw)
        if (parsed.type === 'log' && typeof parsed.data === 'string') {
          message = parsed.data
        }
      } catch {
        // Not JSON, use raw string
      }
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

      // Only reconnect if not manually stopped
      if (!stopped && reconnectCount < reconnectAttempts) {
        reconnectCount++
        reconnectTimeout = setTimeout(() => {
          connect()
        }, reconnectDelay)
      }
    }
  }

  function sendCommand(command: string) {
    if (ws.value?.readyState === WebSocket.OPEN) {
      // Backend expects: {"command": "the command"} or raw text
      ws.value.send(JSON.stringify({ command }))
    }
  }

  function clearMessages() {
    messages.value = []
  }

  function disconnect() {
    stopped = true
    if (reconnectTimeout) {
      clearTimeout(reconnectTimeout)
      reconnectTimeout = null
    }
    if (ws.value) {
      ws.value.close()
      ws.value = null
    }
    connected.value = false
  }

  /** Re-enable connection (e.g. after server starts) */
  function reconnect() {
    stopped = false
    reconnectCount = 0
    connect()
  }

  onMounted(() => {
    if (autoConnect) {
      connect()
    }
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
    connect,
    reconnect
  }
}
