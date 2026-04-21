import api from './index'

export interface SaveInfo {
  id: string
  name: string
  file_path: string
  file_size: number
  source_server_id?: string
  source_server_name?: string
  source_type?: 'manual_upload' | 'server_backup' | 'server_archive'
  created_at: string
}

export interface SaveUploadResponse {
  message: string
  save: SaveInfo
}

export const savesApi = {
  getList: (params?: { server_id?: string; include_other_servers?: boolean }) =>
    api.get<SaveInfo[]>('/saves', { params }),

  upload: (file: File) => {
    const formData = new FormData()
    formData.append('file', file)
    return api.post<SaveUploadResponse>(
      '/saves/upload',
      formData,
      {
        headers: {
          'Content-Type': 'multipart/form-data'
        }
      }
    )
  },

  importToServer: (saveId: string, serverId: string) =>
    api.post(`/saves/${saveId}/import/${serverId}`),

  download: (saveId: string) =>
    api.get(`/saves/${saveId}/download`, {
      responseType: 'blob'
    }),

  delete: (saveId: string) =>
    api.delete(`/saves/${saveId}`),

  backup: (serverId: string) =>
    api.post(`/servers/${serverId}/backup`)
}
