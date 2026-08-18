# 神人网

收录逆天言论的站点：React + MUI 前台，Rust（Axum + SeaORM）后端。同一套迁移同时支持 **SQLite（开发）** 与 **MySQL（生产）**。

> **Frontend note:** 若仓库里还没有 `frontend/`，请另行搭建 Vite + MUI 工程；后端可独立运行。生产构建后把 `frontend/dist` 交给 Axum 静态托管。

## 目录

```
shenren/
  backend/           # Axum API + SeaORM + migrations
  frontend/          # Vite + MUI（可由前端任务创建）
  docker-compose.yml # 可选本地 MySQL
  scripts/dev.ps1    # 同时启动前后端热重载
  package.json       # concurrently 脚本
```

## 数据库

默认（开发，**不推荐生产**）：

```bash
DATABASE_URL=sqlite://data/shenren.db?mode=rwc
```

相对路径相对于 `backend/` 工作目录。启动时自动跑迁移并创建默认站点配置。

MySQL（生产 / 可选本地）：

```bash
# 启动可选 MySQL
docker compose up -d

DATABASE_URL=mysql://shenren:shenren@127.0.0.1:3306/shenren
```

两库使用同一套 `sea-orm-migration`，无 SQLite-only / MySQL-only SQL。

## 后端

### 依赖

- Rust（stable）
- 热重载（开发）：`cargo install watchexec-cli`

### 运行

```powershell
cd backend
# 可选：复制并修改环境变量
# $env:DATABASE_URL = "sqlite://data/shenren.db?mode=rwc"
# $env:BIND_ADDR = "127.0.0.1:3000"
cargo run
```

热重载（改 `.rs` / `.toml` 后杀进程再编译启动，接近 Go air）：

```powershell
cd backend
watchexec -e rs,toml -r cargo run
```

默认监听 `http://127.0.0.1:3000`。

### 环境变量

| 变量 | 默认 | 说明 |
|------|------|------|
| `DATABASE_URL` | `sqlite://data/shenren.db?mode=rwc` | SQLite 或 MySQL |
| `BIND_ADDR` | `127.0.0.1:3000` | 监听地址 |
| `UPLOADS_DIR` | `uploads` | 头像目录（相对 backend cwd） |
| `FRONTEND_DIST` | `../frontend/dist` | 存在 `index.html` 时由 Axum 托管 |
| `COOKIE_SECURE` | `false` | 生产 HTTPS 设为 `true` |

## 前后端一起开发

根目录：

```powershell
npm install
cargo install watchexec-cli   # 若尚未安装
.\scripts\dev.ps1
# 或
npm run dev
```

- 前台：`npm --prefix frontend run dev`（Vite HMR；请把 `/api` 与 `/uploads` proxy 到 `127.0.0.1:3000`）
- 后台：`watchexec -e rs,toml -r --cwd backend cargo run`

## 生产

```powershell
npm --prefix frontend run build
cd backend
$env:DATABASE_URL = "mysql://user:pass@host:3306/shenren"
$env:COOKIE_SECURE = "true"
$env:FRONTEND_DIST = "../frontend/dist"
cargo run --release
```

Axum 在检测到 `FRONTEND_DIST/index.html` 后会用 `tower-http` `ServeDir` 托管静态资源与 SPA fallback。

## 公开 API

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/site` | 站点信息（含 `allow_propose_person`） |
| GET | `/api/quotes?page=&page_size=` | 已通过言论（新→旧） |
| GET | `/api/persons` | 神人列表（投稿下拉） |
| POST | `/api/submissions` | 投稿（按 IP 内存限流） |
| GET | `/uploads/...` | 头像静态文件 |

## 管理 API

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/admin/bootstrap-status` | 是否需要初始化 |
| POST | `/api/admin/setup` | 无管理员时创建超管并登录 |
| POST | `/api/admin/login` / `logout` | Cookie Session（HttpOnly + SameSite=Lax） |
| GET | `/api/admin/me` | 当前管理员 |
| CRUD | `/api/admin/persons` | 神人（multipart 头像） |
| GET/PUT | `/api/admin/settings` | 站点配置 |
| CRUD | `/api/admin/admins` | 管理员（不可删光最后一个；删自己时若仍有他人则允许并注销） |
| GET | `/api/admin/quotes` | 审核列表 |
| POST | `/api/admin/quotes/:id/approve` | multipart：新神人需 `avatar`，或 `person_id` 绑定 |
| POST | `/api/admin/quotes/:id/approve-json` | JSON 绑定已有神人 / 已有 `person_id` 直接通过 |
| POST | `/api/admin/quotes/:id/reject` | 驳回 |

密码使用 Argon2。
