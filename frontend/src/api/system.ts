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

export const systemApi = {
  getSystemInfo: () =>
    api.get<SystemInfo>('/system/info'),

  getOperationLogs: (limit?: number, offset?: number) =>
    api.get<OperationLog[]>('/system/logs', {
      params: { limit, offset }
    }),

  getUsers: () =>
    api.get<UserInfo[]>('/users'),

  createUser: (data: CreateUserRequest) =>
    api.post<UserInfo>('/users', data),

  updateUser: (id: string, data: UpdateUserRequest) =>
    api.put<UserInfo>(`/users/${id}`, data),

  deleteUser: (id: string) =>
    api.delete(`/users/${id}`)
}
