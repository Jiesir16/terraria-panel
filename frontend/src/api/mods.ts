import api from './index'

export interface ModInfo {
  name: string
  file_path: string
  file_size: number
  enabled: boolean
  uploaded_at: string
}

export interface ModUploadResponse {
  message: string
  mod: ModInfo
}

export const modsApi = {
  getList: (serverId: string) =>
    api.get<ModInfo[]>(`/servers/${serverId}/mods`),

  upload: (serverId: string, file: File) => {
    const formData = new FormData()
    formData.append('file', file)
    return api.post<ModUploadResponse>(
      `/servers/${serverId}/mods`,
      formData,
      {
        headers: {
          'Content-Type': 'multipart/form-data'
        }
      }
    )
  },

  toggle: (serverId: string, modName: string) =>
    api.put(`/servers/${serverId}/mods/${modName}/toggle`),

  delete: (serverId: string, modName: string) =>
    api.delete(`/servers/${serverId}/mods/${modName}`)
}
