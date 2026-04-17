import api from './index'

export interface VersionInfo {
  version: string
  name: string
  release_date: string
  download_url: string
  size?: number
  is_downloaded: boolean
}

export const versionApi = {
  getDownloaded: () =>
    api.get<VersionInfo[]>('/versions'),

  getAvailable: () =>
    api.get<VersionInfo[]>('/versions/available'),

  download: (version: string) =>
    api.post('/versions/download', { version }),

  delete: (version: string) =>
    api.delete(`/versions/${version}`),

  getDownloadProgress: (version: string) =>
    api.get<{ progress: number }>(`/versions/${version}/progress`)
}
