# Terraria Server Console - Frontend

泰拉瑞亚开服控制台前端 - 基于 Vue 3 + Naive UI 的现代化 Web 管理界面。

## 项目特点

- **Vue 3 Composition API**: 使用 `<script setup>` 语法的现代化 Vue 3 开发
- **TypeScript**: 全面的类型安全支持
- **Naive UI**: 专业的 UI 组件库
- **暗色主题**: "像素冒险"游戏风暗色设计主题
- **实时通信**: WebSocket 实时控制台输出
- **状态管理**: Pinia 全局状态管理
- **路由系统**: Vue Router 4 页面路由

## 技术栈

- Vue 3.4+ (Composition API)
- TypeScript 5.3+
- Vite 5.0+ (构建工具)
- Naive UI 2.38+
- Axios 1.6+ (HTTP 客户端)
- Pinia 2.1+ (状态管理)
- Vue Router 4.3+ (路由)

## 安装和开发

### 安装依赖

```bash
npm install
```

### 开发模式

```bash
npm run dev
```

访问 http://localhost:5173

开发模式下会自动代理 `/api` 请求到 `http://localhost:3000`

### 生产构建

```bash
npm run build
```

输出目录: `dist/`

## 项目结构

```
src/
├── api/                    # HTTP API 接口定义
│   ├── index.ts           # Axios 实例配置
│   ├── auth.ts            # 认证 API
│   ├── server.ts          # 服务器管理 API
│   ├── version.ts         # 版本管理 API
│   ├── mods.ts            # Mod 管理 API
│   ├── saves.ts           # 存档管理 API
│   └── system.ts          # 系统信息 API
├── components/            # Vue 组件
│   ├── layout/            # 布局组件
│   │   ├── AppLayout.vue
│   │   ├── Sidebar.vue
│   │   └── TopBar.vue
│   ├── server/            # 服务器相关组件
│   ├── mod/               # Mod 管理组件
│   ├── save/              # 存档管理组件
│   ├── user/              # 用户管理组件
│   └── common/            # 通用组件
├── composables/           # Vue Composable
│   ├── useWebSocket.ts    # WebSocket 连接
│   └── useNotification.ts # 通知提示
├── router/                # 路由配置
├── stores/                # Pinia 状态管理
│   ├── auth.ts            # 认证状态
│   ├── servers.ts         # 服务器状态
│   └── app.ts             # 应用全局状态
├── views/                 # 页面组件
│   ├── Login.vue          # 登录页
│   ├── Dashboard.vue      # 仪表盘
│   ├── ServerList.vue     # 服务器列表
│   ├── ServerDetail.vue   # 服务器详情
│   ├── VersionManager.vue # 版本管理
│   ├── SaveManager.vue    # 存档管理
│   ├── Settings.vue       # 系统设置
│   └── UserManager.vue    # 用户管理
├── styles/                # 全局样式
│   └── global.css         # 全局 CSS
├── App.vue                # 根组件
└── main.ts                # 应用入口
```

## 主要功能

### 登录认证
- 用户名/密码登录
- JWT Token 管理
- 自动重定向到登录页

### 仪表盘
- 服务器统计卡片
- 服务器状态网格
- 最近操作日志

### 服务器管理
- 服务器列表查看
- 服务器创建/编辑/删除
- 服务器启动/停止/重启
- 实时控制台输出

### 版本管理
- 已下载版本列表
- GitHub 可用版本列表
- 版本下载/删除

### Mod 管理
- Mod 文件上传
- Mod 启用/禁用
- Mod 删除

### 存档管理
- 存档上传
- 存档导入到服务器
- 存档下载
- 自动备份

### 用户管理 (管理员)
- 用户列表
- 用户创建
- 用户删除
- 角色管理 (admin/operator/viewer)

### 系统设置
- 系统信息展示
- 密码修改

## 配置

### Axios 代理配置

在 `vite.config.ts` 中配置：

```typescript
proxy: {
  '/api': {
    target: 'http://localhost:3000',
    changeOrigin: true
  }
}
```

### 主题色定制

在 `src/App.vue` 中修改 `themeOverrides` 对象：

```typescript
const themeOverrides = {
  common: {
    primaryColor: '#50C878',      // 主色
    bodyColor: '#0D1117',         // 背景色
    // ...
  }
}
```

## 路由

| 路径 | 名称 | 说明 |
|------|------|------|
| `/login` | Login | 登录页 |
| `/` | Dashboard | 仪表盘 |
| `/servers` | ServerList | 服务器列表 |
| `/servers/:id` | ServerDetail | 服务器详情 |
| `/versions` | VersionManager | 版本管理 |
| `/mods` | ModManager | Mod 管理 |
| `/saves` | SaveManager | 存档管理 |
| `/settings` | Settings | 系统设置 |
| `/users` | UserManager | 用户管理 (仅管理员) |

## 状态管理 (Pinia)

### authStore
- `token`: JWT Token
- `user`: 当前用户信息
- `isAuthenticated`: 是否已认证
- `isAdmin`: 是否管理员
- `login()`: 登录
- `logout()`: 登出
- `fetchCurrentUser()`: 获取当前用户

### serversStore
- `servers`: 服务器列表
- `currentServer`: 当前服务器
- `fetchServers()`: 获取服务器列表
- `startServer()`: 启动服务器
- `stopServer()`: 停止服务器
- `sendCommand()`: 发送命令

### appStore
- `notifications`: 通知列表
- `addNotification()`: 添加通知
- `removeNotification()`: 移除通知

## WebSocket 连接

使用 `useWebSocket` composable 连接服务器控制台：

```typescript
const { messages, sendCommand, connected } = useWebSocket(serverId, {
  onMessage: (data) => {
    console.log(data)
  },
  onError: (error) => {
    console.error(error)
  }
})

sendCommand('/save')
```

## 样式指南

### 颜色系统

| 用途 | 颜色 |
|------|------|
| 主色 | `#50C878` |
| 背景 | `#0D1117` |
| 卡片 | `#161B22` |
| 边框 | `#30363D` |
| 文字主 | `#E8E8E8` |
| 文字次 | `#B0B0B0` |

### 响应式

使用 CSS Grid 和 Flexbox 实现响应式布局。

移动设备宽度 < 768px 时，侧边栏自动折叠。

## 浏览器兼容性

- Chrome/Edge 90+
- Firefox 88+
- Safari 14+

## 默认账号

- 用户名: `admin`
- 密码: `admin123`

首次登录后请立即修改密码！

## 许可证

MIT
