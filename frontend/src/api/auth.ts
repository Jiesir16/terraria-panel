import api from './index'

export interface LoginRequest {
  username: string
  password: string
}

export interface LoginResponse {
  token: string
  user: UserInfo
}

export interface UserInfo {
  id: string
  username: string
  role: 'admin' | 'operator' | 'viewer'
  created_at: string
}

export interface RegisterRequest {
  username: string
  password: string
}

export interface ChangePasswordRequest {
  old_password: string
  new_password: string
}

export const authApi = {
  login: (data: LoginRequest) =>
    api.post<LoginResponse>('/auth/login', data),

  register: (data: RegisterRequest) =>
    api.post<LoginResponse>('/auth/register', data),

  refreshToken: () =>
    api.post<LoginResponse>('/auth/refresh'),

  changePassword: (data: ChangePasswordRequest) =>
    api.put('/auth/password', data),

  getCurrentUser: () =>
    api.get<UserInfo>('/auth/me')
}
