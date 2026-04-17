import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { serverApi, ServerStatus, ServerDetailResponse, CreateServerRequest, UpdateServerRequest, ServerConfig } from '../api/server'

export const useServersStore = defineStore('servers', () => {
  const servers = ref<ServerStatus[]>([])
  const currentServer = ref<ServerStatus | null>(null)
  const loading = ref(false)

  const runningCount = computed(() =>
    servers.value.filter(s => s.status === 'running').length
  )

  const totalPlayers = computed(() =>
    servers.value.reduce((sum, s) => sum + s.player_count, 0)
  )

  async function fetchServers() {
    loading.value = true
    try {
      const response = await serverApi.getList()
      // list_servers returns Server[] (no player_count), normalize to ServerStatus[]
      servers.value = response.data.map((s: any) => ({
        ...s,
        player_count: s.player_count ?? 0,
        uptime_seconds: s.uptime_seconds ?? 0
      }))
      return servers.value
    } finally {
      loading.value = false
    }
  }

  async function fetchServer(id: string) {
    try {
      const response = await serverApi.getDetail(id)
      // Backend returns { server: {...}, player_count, uptime_seconds }
      // Flatten it into a single ServerStatus object
      const detail = response.data as ServerDetailResponse
      const flat: ServerStatus = {
        ...detail.server,
        player_count: detail.player_count ?? 0,
        uptime_seconds: detail.uptime_seconds ?? 0
      }
      currentServer.value = flat
      const index = servers.value.findIndex(s => s.id === id)
      if (index >= 0) {
        servers.value[index] = flat
      }
      return flat
    } catch (error) {
      throw error
    }
  }

  async function createServer(data: CreateServerRequest) {
    const response = await serverApi.create(data)
    const server: ServerStatus = { ...response.data, player_count: 0, uptime_seconds: 0 }
    servers.value.push(server)
    return server
  }

  async function updateServer(id: string, data: UpdateServerRequest) {
    const response = await serverApi.update(id, data)
    const server: ServerStatus = { ...response.data, player_count: 0, uptime_seconds: 0 }
    const index = servers.value.findIndex(s => s.id === id)
    if (index >= 0) {
      servers.value[index] = server
    }
    if (currentServer.value?.id === id) {
      currentServer.value = server
    }
    return server
  }

  async function deleteServer(id: string) {
    await serverApi.delete(id)
    servers.value = servers.value.filter(s => s.id !== id)
    if (currentServer.value?.id === id) {
      currentServer.value = null
    }
  }

  async function startServer(id: string) {
    const response = await serverApi.start(id)
    await fetchServer(id)
    return response.data
  }

  async function stopServer(id: string) {
    const response = await serverApi.stop(id)
    await fetchServer(id)
    return response.data
  }

  async function restartServer(id: string) {
    const response = await serverApi.restart(id)
    await fetchServer(id)
    return response.data
  }

  async function killServer(id: string) {
    const response = await serverApi.kill(id)
    await fetchServer(id)
    return response.data
  }

  async function refreshServerRuntime(id: string) {
    const [detailResponse, statusResponse] = await Promise.all([
      serverApi.getDetail(id),
      serverApi.getStatus(id)
    ])

    const detail = detailResponse.data as ServerDetailResponse
    const flat: ServerStatus = {
      ...detail.server,
      status: statusResponse.data.status,
      player_count: detail.player_count ?? 0,
      uptime_seconds: detail.uptime_seconds ?? 0
    }

    currentServer.value = flat
    const index = servers.value.findIndex(s => s.id === id)
    if (index >= 0) {
      servers.value[index] = flat
    }

    return {
      server: flat,
      runtime: statusResponse.data
    }
  }

  async function sendCommand(id: string, command: string) {
    return serverApi.sendCommand(id, command)
  }

  async function getConfig(id: string) {
    const response = await serverApi.getConfig(id)
    return response.data
  }

  async function updateConfig(id: string, config: ServerConfig) {
    return serverApi.updateConfig(id, config)
  }

  async function importConfig(id: string, config: ServerConfig) {
    return serverApi.importConfig(id, config)
  }

  async function exportConfig(id: string) {
    const response = await serverApi.exportConfig(id)
    return response.data
  }

  function getServerById(id: string) {
    return servers.value.find(s => s.id === id)
  }

  return {
    servers,
    currentServer,
    loading,
    runningCount,
    totalPlayers,
    fetchServers,
    fetchServer,
    createServer,
    updateServer,
    deleteServer,
    startServer,
    stopServer,
    restartServer,
    killServer,
    refreshServerRuntime,
    sendCommand,
    getConfig,
    updateConfig,
    importConfig,
    exportConfig,
    getServerById
  }
})
