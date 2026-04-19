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

export interface SscInventoryItem {
  netID: number
  prefix: number
  stack: number
  favorited: boolean
}

export interface SscConfigSettings {
  Enabled: boolean
  ServerSideCharacterSave: number
  LogonDiscardThreshold: number
  StartingHealth: number
  StartingMana: number
  StartingInventory: SscInventoryItem[]
  WarnPlayersAboutBypassPermission: boolean
  KeepPlayerAppearance: boolean
}

export interface SscConfig {
  Settings: SscConfigSettings
}

export interface TShockUserAccount {
  username: string
  group_name?: string
  is_superadmin: boolean
  ignores_ssc: boolean
}

export interface TShockGroupSummary {
  name: string
  permission_count: number
  ignores_ssc: boolean
  is_registration_group: boolean
  is_guest_group: boolean
}

export interface TShockSecurityOverview {
  ssc_enabled: boolean
  ssc_source: string
  default_registration_group?: string
  default_guest_group?: string
  database_exists: boolean
  users: TShockUserAccount[]
  groups: TShockGroupSummary[]
}

export interface TShockGroupDetail {
  name: string
  parent?: string
  permissions: string[]
  member_count: number
}

export interface TShockSscCharacterSummary {
  account: number
  username?: string
  health: number
  max_health: number
  mana: number
  max_mana: number
  quests_completed: number
}

export interface TShockSscCharacter extends TShockSscCharacterSummary {
  inventory?: string
  extra_slot?: number
  spawn_x?: number
  spawn_y?: number
  skin_variant?: number
  hair?: number
  hair_dye?: number
  hair_color?: number
  pants_color?: number
  shirt_color?: number
  under_shirt_color?: number
  shoe_color?: number
  skin_color?: number
  eye_color?: number
  hide_visuals?: string
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

  getTshockSecurity: (id: string) =>
    api.get<TShockSecurityOverview>(`/servers/${id}/tshock-security`),

  getConfig: (id: string) =>
    api.get<ServerConfig>(`/servers/${id}/config`),

  updateConfig: (id: string, config: ServerConfig) =>
    api.put(`/servers/${id}/config`, config),

  getSscConfig: (id: string) =>
    api.get<SscConfig>(`/servers/${id}/ssc-config`),

  updateSscConfig: (id: string, config: SscConfig) =>
    api.put(`/servers/${id}/ssc-config`, config),

  importConfig: (id: string, config: ServerConfig) =>
    api.post(`/servers/${id}/config/import`, config),

  exportConfig: (id: string) =>
    api.get<ServerConfig>(`/servers/${id}/config/export`),

  listWorlds: (id: string) =>
    api.get<WorldFile[]>(`/servers/${id}/worlds`),

  // TShock user management
  updateTshockUserGroup: (id: string, username: string, group: string) =>
    api.put(`/servers/${id}/tshock-users/${encodeURIComponent(username)}/group`, { group }),

  deleteTshockUser: (id: string, username: string) =>
    api.delete(`/servers/${id}/tshock-users/${encodeURIComponent(username)}`),

  // TShock group management
  getTshockGroup: (id: string, groupName: string) =>
    api.get<TShockGroupDetail>(`/servers/${id}/tshock-groups/${encodeURIComponent(groupName)}`),

  createTshockGroup: (id: string, name: string, parent?: string) =>
    api.post(`/servers/${id}/tshock-groups`, { name, parent }),

  deleteTshockGroup: (id: string, groupName: string) =>
    api.delete(`/servers/${id}/tshock-groups/${encodeURIComponent(groupName)}`),

  // TShock permission management
  addTshockPermission: (id: string, groupName: string, permission: string) =>
    api.post(`/servers/${id}/tshock-groups/${encodeURIComponent(groupName)}/permissions`, { permission }),

  removeTshockPermission: (id: string, groupName: string, permission: string) =>
    api.post(`/servers/${id}/tshock-groups/${encodeURIComponent(groupName)}/permissions/remove`, { permission }),

  // SSC character management
  listSscCharacters: (id: string) =>
    api.get<TShockSscCharacterSummary[]>(`/servers/${id}/ssc-characters`),

  exportSscCharacter: (id: string, accountId: number) =>
    api.get<TShockSscCharacter>(`/servers/${id}/ssc-characters/${accountId}`),

  backupSscCharacters: (id: string) =>
    api.post(`/servers/${id}/ssc-characters/backup`)
}
