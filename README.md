# Terraria Panel

中文 | [English](README.en.md)

Terraria Panel 是一个面向 Terraria / TShock 服务器的 Web 管理面板。后端使用 Rust + Axum，前端使用 Vue 3 + TypeScript + Naive UI，提供服务器实例管理、实时控制台、TShock REST 管理、存档备份、FRP 穿透、用户权限等能力。

> 当前项目仍在迭代中，建议先在测试环境验证后再部署到公网。

## 功能预览

### 仪表盘

集中展示服务器总数、运行状态、在线玩家、系统负载和最近操作日志，适合快速判断面板整体状态。

![仪表盘概览](images/1777196841538.jpg)

### 服务器管理

服务器列表支持查看 TShock 版本、端口、玩家数和运行状态，并提供启动、停止、详情、强制结束和删除等操作。

![服务器管理列表](images/1777196849652.jpg)

![服务器管理操作](images/1777196855915.jpg)

### 服务器详情与实时控制台

服务器详情页提供控制台、配置、TShock 权限、实时管理、命令库、FRP、Mod 管理和存档等标签页。控制台支持实时日志、常用命令快捷入口和命令发送。

![服务器详情实时控制台](images/1777196861213.jpg)

### TShock 版本管理

版本管理页展示已下载版本和可用版本，支持 GitHub 代理设置和版本下载，便于维护多个 TShock 运行版本。

![TShock 版本管理](images/1777196865720.jpg)

### 存档与备份包

存档管理按服务器分组展示备份归档包，支持上传、下载和删除，便于迁移、恢复和长期留存世界存档。

![存档与备份包管理](images/1777196870424.jpg)

### 备份策略

备份策略支持自动备份、SSC 数据库备份、每日归档、本地保留策略，以及 NAS / 本地目录或云对象存储同步。

![备份策略设置](images/1777196874524.jpg)

### 用户与角色

用户管理页支持创建用户、删除用户，并为用户分配管理员、操作员等面板角色。

![用户管理](images/1777196643650.jpg)

![用户角色管理](images/1777196879066.jpg)

## 功能特性

- 服务器生命周期管理：创建、启动、停止、强制结束、重启、删除 Terraria / TShock 实例。
- 实时控制台：通过 WebSocket 查看服务器输出并发送控制台命令。
- TShock REST 管理：自动配置 REST Token，管理玩家、用户、用户组、封禁、世界事件和广播。
- 命令库：常用 TShock 命令参数化执行，支持物品和 Buff 下拉选择。
- 物品与 Buff：物品清单缓存、中文名展示、发放物品、一键配置 Buff、读取并清除玩家激活 Buff。
- TShock 安全管理：用户组、权限、SSC 配置和角色调整。
- 版本管理：查看已安装 TShock 版本，下载可用版本。
- 存档管理：上传、导入、下载、手动备份。
- 自动备份：定时备份、每日归档、本地保留策略，可选 NAS / 腾讯云 COS 同步。
- FRP 设置：全局 FRP 配置、面板隧道、单服务器隧道恢复和状态管理。
- 用户权限：`admin`、`operator`、`viewer` 三类面板角色。
- Telegram Bot：可选远程查看服务器状态、启动/停止/重启服务器、发送命令。

## 技术栈

### 后端

- Rust 2021
- Axum 0.7
- Tokio
- SQLite / rusqlite
- JWT / Argon2
- Reqwest
- WebSocket

### 前端

- Vue 3
- TypeScript
- Vite
- Naive UI
- Pinia
- Vue Router
- Axios

## 环境要求

- Linux 服务器或开发机
- Rust stable toolchain
- Node.js 18+ 和 npm
- TShock 6.x 所需的 .NET Runtime
- 可选：用于兼容旧版 TShock 的 Mono
- 可选：用于 FRP 穿透的 frpc

## 快速开始

### 1. 克隆项目

```bash
git clone https://github.com/your-name/terraria-panel.git
cd terraria-panel
```

### 2. 启动后端

```bash
cd backend
cargo run
```

首次启动时，如果 `backend/config.toml` 不存在，后端会自动生成默认配置文件。

也可以显式指定配置文件：

```bash
cd backend
TERRARIA_CONSOLE_CONFIG=./config.toml cargo run
```

后端默认监听：

```text
http://localhost:3000
```

### 3. 启动前端

另开一个终端：

```bash
cd frontend
npm install
npm run dev
```

前端默认访问：

```text
http://localhost:5173
```

Vite 开发服务器会把 `/api` 代理到 `http://localhost:3000`。

### 4. 登录

首次启动会创建默认管理员账号：

```text
用户名：admin
密码：admin123
```

生产部署前必须修改默认密码。

## 生产构建

### 后端

```bash
cd backend
cargo build --release
```

构建产物：

```text
backend/target/release/terraria-console-backend
```

### 前端

```bash
cd frontend
npm install
npm run build
```

构建产物：

```text
frontend/dist
```

## 配置说明

后端默认读取 `backend/config.toml`，也可以通过环境变量指定：

```bash
TERRARIA_CONSOLE_CONFIG=/path/to/config.toml
```

| 配置段 | 说明 |
| --- | --- |
| `[server]` | 后端监听地址、端口、数据目录、日志目录 |
| `[auth]` | JWT 密钥、Token 有效期、是否允许注册 |
| `[tshock]` | dotnet / mono 路径、GitHub 镜像、服务器端口范围 |
| `[telegram]` | Telegram Bot 开关、Bot Token、允许的 chat id |
| `[backup]` | 自动备份、归档、保留策略 |
| `[backup.oss]` | NAS / 腾讯云 COS 远端备份配置 |

部署到公网前至少需要修改：

```toml
[auth]
jwt_secret = "replace-with-a-long-random-secret"
allow_register = false
```

## 目录结构

```text
.
├── backend/
│   ├── src/
│   │   ├── auth/              # JWT、密码哈希、认证中间件
│   │   ├── handlers/          # Axum API 处理器
│   │   ├── models/            # 数据模型
│   │   ├── services/          # 进程、备份、FRP、TShock REST 服务
│   │   ├── config.rs          # 后端配置
│   │   ├── db.rs              # SQLite 初始化
│   │   └── main.rs            # 应用入口与路由
│   ├── config.toml            # 本地配置
│   └── data/                  # 运行时数据，默认忽略
├── frontend/
│   ├── src/
│   │   ├── api/               # HTTP API 封装
│   │   ├── components/        # Vue 组件
│   │   ├── constants/         # 命令、权限、物品和 Buff 常量
│   │   ├── router/            # 路由
│   │   ├── stores/            # Pinia 状态
│   │   ├── styles/            # 全局样式
│   │   └── views/             # 页面
│   └── package.json
├── images/                    # README 截图
└── scripts/                   # 运维辅助脚本
```

## 数据与运行时文件

默认运行时数据位于 `backend/data`。

这里会包含服务器实例、TShock 配置、世界存档、备份、日志、FRP 运行配置和 SQLite 数据库。这些文件可能包含真实服务器信息、Token、玩家数据或存档，不应提交到公开仓库。

## 常用开发命令

后端：

```bash
cd backend
cargo run
cargo test
cargo check
cargo build --release
```

前端：

```bash
cd frontend
npm install
npm run dev
npm run build
```

