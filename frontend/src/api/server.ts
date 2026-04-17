import api from './index'

export interface ServerStatus {
  id: string
  name: string
  port: number
  tshock_version: string
  world_name?: string
  status: 'stopped' | 'starting' | 'running' | 'stopping' | 'error'
  password?: string
  max_players: number
  player_count: number
  auto_start: boolean
  created_by: string
  created_at: string
  updated_at: string
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

export interface ServerConfig {
  [key: string]: any
}

export const serverApi = {
  getList: () =>
    api.get<ServerStatus[]>('/servers'),

  getDetail: (id: string) =>
    api.get<ServerStatus>(`/servers/${id}`),

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

  restart: (id: string) =>
    api.post(`/servers/${id}/restart`),

  sendCommand: (id: string, command: string) =>
    api.post(`/servers/${id}/command`, { command }),

  getStatus: (id: string) =>
    api.get<ServerStatus>(`/servers/${id}/status`),

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
