import api from './index'

export interface VersionInfo {
  version: string
  tag_name: string
  name: string
  download_url: string
  published_at: string
  size: number
  downloaded: boolean
}

export interface LocalVersion {
  version: string
  name: string
  path: string
  size: number
  is_dotnet: boolean
  installed_at: string
}

export interface AvailableVersionsResponse {
  versions: VersionInfo[]
  total: number
  page: number
  per_page: number
  has_more: boolean
}

export const versionApi = {
  getDownloaded: () =>
    api.get<LocalVersion[]>('/versions'),

  getAvailable: (page = 1, perPage = 10) =>
    api.get<AvailableVersionsResponse>('/versions/available', {
      params: { page, per_page: perPage }
    }),

  download: (tagName: string, downloadUrl: string) =>
    api.post('/versions/download', { tag_name: tagName, download_url: downloadUrl }, {
      timeout: 600000 // 10 minutes for large downloads
    }),

  delete: (version: string) =>
    api.delete(`/versions/${version}`),

  getProxy: () =>
    api.get<{ mirror: string }>('/versions/proxy'),

  setProxy: (mirror: string) =>
    api.put('/versions/proxy', { mirror }),

  getDownloadProgress: (version: string) =>
    api.get<{ progress: number }>(`/versions/${version}/progress`)
}
