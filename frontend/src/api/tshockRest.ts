import api from './index'

// ─── Types ───

export interface TShockRestPlayer {
  nickname: string
  username: string
  group: string
  active: boolean
  state: number
  team: number
  index: number
}

export interface TShockServerStatus {
  name: string
  serverversion: string
  tshockversion: string
  port: number
  playercount: number
  maxplayers: number
  world: string
  uptime: string
  serverpassword: boolean
  players?: TShockRestPlayer[]
  rules?: Record<string, string>
  status: string
}

export interface TShockWorldInfo {
  name: string
  size: string
  time: number
  daytime: boolean
  bloodmoon: boolean
  invasionsize: number
  status: string
}

export interface TShockBan {
  ticketNumber: string
  identifier: string
  reason: string
  banningUser: string
  date: string
  expiration: string
}

export interface TerrariaItem {
  id: number
  name: string
  internal_name: string
}

export interface TerrariaItemListResponse {
  version: string
  source: string
  items: TerrariaItem[]
}

// ─── API methods ───

const base = (serverId: string) => `/servers/${serverId}/rest`

export interface RestSetupResult {
  ready: boolean
  message: string
}

export const tshockRestApi = {
  // ── Setup ──
  setup: (serverId: string) =>
    api.post<RestSetupResult>(`${base(serverId)}/setup`),

  // ── Server ──
  serverStatus: (serverId: string) =>
    api.get<TShockServerStatus>(`${base(serverId)}/server/status`),

  serverBroadcast: (serverId: string, msg: string) =>
    api.post(`${base(serverId)}/server/broadcast`, { msg }),

  serverReload: (serverId: string) =>
    api.post(`${base(serverId)}/server/reload`),

  serverRawcmd: (serverId: string, cmd: string) =>
    api.post(`${base(serverId)}/server/rawcmd`, { cmd }),

  serverOff: (serverId: string, message?: string, nosave = false) =>
    api.post(`${base(serverId)}/server/off`, { message, nosave }),

  itemList: (serverId: string, q?: string, limit = 10000) =>
    api.get<TerrariaItemListResponse>(`${base(serverId)}/items`, { params: { q, limit } }),

  itemSync: (serverId: string) =>
    api.post<TerrariaItemListResponse>(`${base(serverId)}/items/sync`),

  itemGive: (serverId: string, data: { player: string; item_id?: number; item_name?: string; stack?: number }) =>
    api.post(`${base(serverId)}/items/give`, data),

  serverMotd: (serverId: string) =>
    api.get(`${base(serverId)}/server/motd`),

  serverRules: (serverId: string) =>
    api.get(`${base(serverId)}/server/rules`),

  // ── Players ──
  playerList: (serverId: string) =>
    api.get(`${base(serverId)}/players/list`),

  playerRead: (serverId: string, player: string) =>
    api.get(`${base(serverId)}/players/${encodeURIComponent(player)}`),

  playerKick: (serverId: string, player: string, reason?: string) =>
    api.post(`${base(serverId)}/players/kick`, { player, reason }),

  playerKill: (serverId: string, player: string) =>
    api.post(`${base(serverId)}/players/kill`, { player }),

  playerMute: (serverId: string, player: string) =>
    api.post(`${base(serverId)}/players/mute`, { player }),

  playerUnmute: (serverId: string, player: string) =>
    api.post(`${base(serverId)}/players/unmute`, { player }),

  // ── Users ──
  userList: (serverId: string) =>
    api.get(`${base(serverId)}/users/list`),

  userActiveList: (serverId: string) =>
    api.get(`${base(serverId)}/users/activelist`),

  userRead: (serverId: string, user: string) =>
    api.get(`${base(serverId)}/users/${encodeURIComponent(user)}`),

  userCreate: (serverId: string, user: string, password: string, group?: string) =>
    api.post(`${base(serverId)}/users/create`, { user, password, group }),

  userUpdate: (serverId: string, user: string, password?: string, group?: string) =>
    api.post(`${base(serverId)}/users/update`, { user, password, group }),

  userDestroy: (serverId: string, user: string) =>
    api.delete(`${base(serverId)}/users/${encodeURIComponent(user)}`),

  // ── Groups ──
  groupList: (serverId: string) =>
    api.get(`${base(serverId)}/groups/list`),

  groupRead: (serverId: string, name: string) =>
    api.get(`${base(serverId)}/groups/${encodeURIComponent(name)}`),

  groupCreate: (serverId: string, group: string, parent?: string, permissions?: string) =>
    api.post(`${base(serverId)}/groups/create`, { group, parent, permissions }),

  groupUpdate: (serverId: string, name: string, parent?: string, permissions?: string) =>
    api.put(`${base(serverId)}/groups/${encodeURIComponent(name)}`, { parent, permissions }),

  groupDestroy: (serverId: string, name: string) =>
    api.delete(`${base(serverId)}/groups/${encodeURIComponent(name)}`),

  // ── Bans ──
  banList: (serverId: string) =>
    api.get(`${base(serverId)}/bans/list`),

  banRead: (serverId: string, ticket: string) =>
    api.get(`${base(serverId)}/bans/${encodeURIComponent(ticket)}`),

  banCreate: (serverId: string, identifier: string, reason?: string, duration?: string) =>
    api.post(`${base(serverId)}/bans/create`, { identifier, reason, duration }),

  banDestroy: (serverId: string, ticket: string) =>
    api.delete(`${base(serverId)}/bans/${encodeURIComponent(ticket)}`),

  // ── World ──
  worldRead: (serverId: string) =>
    api.get<TShockWorldInfo>(`${base(serverId)}/world/read`),

  worldSave: (serverId: string) =>
    api.post(`${base(serverId)}/world/save`),

  worldButcher: (serverId: string, killFriendly = false) =>
    api.post(`${base(serverId)}/world/butcher`, { kill_friendly: killFriendly }),

  worldBloodmoon: (serverId: string, state: boolean) =>
    api.post(`${base(serverId)}/world/bloodmoon`, { state }),

  worldMeteor: (serverId: string) =>
    api.post(`${base(serverId)}/world/meteor`),

  worldAutosave: (serverId: string, state: boolean) =>
    api.post(`${base(serverId)}/world/autosave`, { state }),
}
