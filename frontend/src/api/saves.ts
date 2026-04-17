import api from './index'

export interface SaveInfo {
  id: string
  name: string
  file_path: string
  file_size: number
  source_server_id?: string
  created_at: string
}

export interface SaveUploadResponse {
  message: string
  save: SaveInfo
}

export const savesApi = {
  getList: () =>
    api.get<SaveInfo[]>('/saves'),

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
