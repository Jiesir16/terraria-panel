import api from './index'

export interface Server {
  id: string
  name: string
  port: number
  tshock_version: string
  world_name?: string
  status: 'stopped' | 'starting' | 'running' | 'stopping' | 'error'
  password?: string
  max_players: number
  auto_start: boolean
  created_by: string
  created_at: string
  updated_at: string
}

// Backend get_server returns nested { server, player_count, uptime_seconds }
export interface ServerDetailResponse {
  server: Server
  player_count: number
  uptime_seconds: number
}

// Flattened version used throughout the frontend
export interface ServerStatus extends Server {
  player_count: number
  uptime_seconds?: number
}

export interface CreateServerRequest {
  name: string
  port?: number
  tshock_version: string
  world_name?: string
  password?: string
  max_players?: number
  auto_start?: boolean
}

export interface UpdateServerRequest {
  name?: string
  port?: number
  password?: string
  max_players?: number
  auto_start?: boolean
  world_name?: string
  tshock_version?: string
}

export interface WorldFile {
  name: string
  size: number
  modified: string
  is_backup: boolean
}

export interface ServerRuntimeStatus {
  status: 'stopped' | 'starting' | 'running'
  running: boolean
  process_running: boolean
  db_status?: string
}

export interface ServerConfig {
  [key: string]: any
}

export const serverApi = {
  getList: () =>
    api.get<ServerStatus[]>('/servers'),

  getDetail: (id: string) =>
    api.get<ServerDetailResponse>(`/servers/${id}`),

  create: (data: CreateServerRequest) =>
    api.post<ServerStatus>('/servers', data),

  update: (id: string, data: UpdateServerRequest) =>
    api.put<ServerStatus>(`/servers/${id}`, data),

  delete: (id: string) =>
    api.delete(`/servers/${id}`),

  start: (id: string) =>
    api.post(`/servers/${id}/start`),

  stop: (id: string) =>
    api.post(`/servers/${id}/stop`),

  kill: (id: string) =>
    api.post(`/servers/${id}/kill`),

  restart: (id: string) =>
    api.post(`/servers/${id}/restart`),

  sendCommand: (id: string, command: string) =>
    api.post(`/servers/${id}/command`, { command }),

  getStatus: (id: string) =>
    api.get<ServerRuntimeStatus>(`/servers/${id}/status`),

  getRecentLogs: (id: string, limit = 200) =>
    api.get<string[]>(`/servers/${id}/logs`, {
      params: { limit }
    }),

  getConfig: (id: string) =>
    api.get<ServerConfig>(`/servers/${id}/config`),

  updateConfig: (id: string, config: ServerConfig) =>
    api.put(`/servers/${id}/config`, config),

  importConfig: (id: string, config: ServerConfig) =>
    api.post(`/servers/${id}/config/import`, config),

  exportConfig: (id: string) =>
    api.get<ServerConfig>(`/servers/${id}/config/export`),

  listWorlds: (id: string) =>
    api.get<WorldFile[]>(`/servers/${id}/worlds`)
}
