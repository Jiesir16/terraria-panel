import api from './index'

export interface SystemInfo {
  cpu_usage: number
  memory_total: number
  memory_used: number
  memory_usage: number
  disk_total: number
  disk_used: number
  disk_usage: number
  uptime: number
  // optional fields for future extensions
  hostname?: string
  os_name?: string
  os_version?: string
  dotnet_version?: string
  mono_version?: string
  cpu_count?: number
}

export interface OperationLog {
  id: number
  user_id?: string
  username?: string
  action: string
  target?: string
  details?: string
  created_at: string
}

export interface UserInfo {
  id: string
  username: string
  role: 'admin' | 'operator' | 'viewer'
  created_at: string
  updated_at: string
}

export interface CreateUserRequest {
  username: string
  password: string
  role: 'admin' | 'operator' | 'viewer'
}

export interface UpdateUserRequest {
  role?: 'admin' | 'operator' | 'viewer'
}

export interface BackupSettings {
  enabled: boolean
  interval_minutes: number
  max_backups_per_server: number
  local_retention_days: number
  backup_ssc: boolean
  archive_daily_enabled: boolean
  archive_hour: number
  archive_after_days: number
  oss: {
    enabled: boolean
    provider: string
    endpoint: string
    bucket: string
    region: string
    access_key_id: string
    access_key_secret: string
    local_path: string
    prefix: string
  }
}

export interface FrpSettings {
  enabled: boolean
  frpc_bin: string
  server_addr: string
  server_port: number
  auth_token: string
  transport_protocol: string
  tls_enable: boolean
  log_level: string
  panel_tunnel: {
    enabled: boolean
    local_port: number
    remote_port: number
    proxy_name: string
  }
}

export interface FrpStatus {
  key: string
  running: boolean
  pid?: number | null
  config_path?: string | null
  remote_port?: number | null
  last_error?: string | null
}

export const systemApi = {
  getSystemInfo: () =>
    api.get<SystemInfo>('/system/info'),

  getOperationLogs: (limit?: number, offset?: number) =>
    api.get<OperationLog[]>('/system/logs', {
      params: { limit, offset }
    }),

  getBackupSettings: () =>
    api.get<BackupSettings>('/settings/backup'),

  updateBackupSettings: (data: BackupSettings) =>
    api.put('/settings/backup', data),

  getFrpSettings: () =>
    api.get<FrpSettings>('/settings/frp'),

  updateFrpSettings: (data: FrpSettings) =>
    api.put('/settings/frp', data),

  getPanelFrpStatus: () =>
    api.get<FrpStatus>('/settings/frp/panel/status'),

  restartPanelFrp: () =>
    api.post('/settings/frp/panel/restart'),

  getUsers: () =>
    api.get<UserInfo[]>('/users'),

  createUser: (data: CreateUserRequest) =>
    api.post<UserInfo>('/users', data),

  updateUser: (id: string, data: UpdateUserRequest) =>
    api.put<UserInfo>(`/users/${id}`, data),

  deleteUser: (id: string) =>
    api.delete(`/users/${id}`)
}
