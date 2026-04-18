export type CommandScope = 'viewer' | 'operator' | 'owner' | 'admin'

export interface CommandItem {
  label: string
  command: string
  scope: CommandScope
  color: 'primary' | 'warning' | 'error'
}

export interface CommandSection {
  label: string
  scope: CommandScope
  items: CommandItem[]
}

export const SERVER_COMMAND_SECTIONS: CommandSection[] = [
  {
    label: '常用',
    scope: 'viewer',
    items: [
      { label: '/who', command: '/who', scope: 'viewer', color: 'primary' },
      { label: '/playing', command: '/playing', scope: 'viewer', color: 'primary' },
      { label: '/time', command: '/time', scope: 'viewer', color: 'primary' },
      { label: '/world', command: '/world', scope: 'viewer', color: 'primary' },
      { label: '/rules', command: '/rules', scope: 'viewer', color: 'primary' },
      { label: '/help', command: '/help', scope: 'viewer', color: 'primary' },
    ],
  },
  {
    label: '管理',
    scope: 'operator',
    items: [
      { label: '/save', command: '/save', scope: 'operator', color: 'warning' },
      { label: '白天', command: '/time day', scope: 'operator', color: 'warning' },
      { label: '黑夜', command: '/time night', scope: 'operator', color: 'warning' },
      { label: '/butcher', command: '/butcher', scope: 'operator', color: 'warning' },
      { label: '/broadcast', command: '/broadcast', scope: 'operator', color: 'warning' },
      { label: '/kick', command: '/kick', scope: 'operator', color: 'warning' },
      { label: '/ban', command: '/ban', scope: 'operator', color: 'warning' },
      { label: '/mute', command: '/mute', scope: 'operator', color: 'warning' },
      { label: '/tp', command: '/tp', scope: 'operator', color: 'warning' },
      { label: '/tphere', command: '/tphere', scope: 'operator', color: 'warning' },
      { label: '/settle', command: '/settle', scope: 'operator', color: 'warning' },
    ],
  },
  {
    label: '服主',
    scope: 'owner',
    items: [
      { label: '/off', command: '/off', scope: 'owner', color: 'warning' },
      { label: '/reload', command: '/reload', scope: 'owner', color: 'warning' },
      { label: '白名单', command: '/whitelist', scope: 'owner', color: 'warning' },
      { label: '区域', command: '/region', scope: 'owner', color: 'warning' },
    ],
  },
  {
    label: '超管',
    scope: 'admin',
    items: [
      { label: '/group list', command: '/group list', scope: 'admin', color: 'error' },
      { label: '/user list', command: '/user list', scope: 'admin', color: 'error' },
      { label: '白名单列表', command: '/whitelist list', scope: 'admin', color: 'error' },
      { label: '/region list', command: '/region list', scope: 'admin', color: 'error' },
      { label: '/gbuff', command: '/gbuff', scope: 'admin', color: 'error' },
      { label: '/grow', command: '/grow', scope: 'admin', color: 'error' },
      { label: '/spawnmob', command: '/spawnmob', scope: 'admin', color: 'error' },
      { label: '/give', command: '/give', scope: 'admin', color: 'error' },
      { label: '/antibuild', command: '/antibuild', scope: 'admin', color: 'error' },
      { label: '/godmode', command: '/godmode', scope: 'admin', color: 'error' },
    ],
  },
]
