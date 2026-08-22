# 神人网

收录逆天言论的站点：React + MUI 前台，Rust（Axum + SeaORM）**纯 API** 后端。同一套迁移同时支持 **SQLite（开发）** 与 **MySQL（生产）**。后端不托管任何前端页面；前台用 Vite 开发，生产由 Nginx / Pages 等单独部署。

## 目录

```
shenren/
  backend/                 # Axum API + SeaORM + migrations
  frontend/                # Vite + MUI
  Dockerfile               # API 镜像（Linux 二进制）
  docker-compose.yml       # 默认：应用连宿主机 MySQL
  docker-compose.mysql.yml # 可选：Compose 内再起 MySQL
  .github/workflows/       # Linux 二进制 + GHCR 镜像
  scripts/dev.ps1
  package.json
```

## 数据库

默认（开发，**不推荐生产**）：

```bash
DATABASE_URL=sqlite://data/shenren.db?mode=rwc
```

相对路径相对于 `backend/` 工作目录。启动时自动跑迁移并创建默认站点配置。

MySQL（生产）：**默认使用宿主机上已有的 MySQL**，不要用 Compose 再起一套，除非你显式加上 `docker-compose.mysql.yml`。

```bash
# 宿主机 MySQL（Compose 默认）
docker compose up -d --build
# 等价于 DATABASE_URL=mysql://shenren:shenren@host.docker.internal:3306/shenren

# 可选：用容器里的 MySQL（本地没有 MySQL，或 CI 手动/发版探测）
docker compose -f docker-compose.yml -f docker-compose.mysql.yml up -d --build
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
| `DATABASE_MAX_CONNECTIONS` | MySQL `32` / SQLite `8` | 连接池上限 |
| `BIND_ADDR` | `127.0.0.1:3000` | 监听地址。非 loopback 时 Cookie 默认 `Secure` |
| `UPLOADS_DIR` | `uploads` | 头像目录（相对 backend cwd；不能是 `/` 或盘符根） |
| `COOKIE_SECURE` | 随监听地址 | 显式 `true`/`false` 可覆盖；非 127.0.0.1/`::1` 默认 `true` |
| `COOKIE_SAMESITE` | `Lax` | `Lax` / `Strict` / `None`（跨站前端用 `None`，且须 HTTPS） |
| `SESSION_TTL_SECS` | `43200` | 管理端会话空闲过期（秒） |
| `CORS_ORIGINS` | （空） | 额外允许的前端 Origin，逗号分隔。`http://localhost:*` 与 `http://127.0.0.1:*` 始终允许 |
| `REQUEST_TIMEOUT_SECS` | `15` | 请求超时 |
| `MAX_CONCURRENCY` | `256` | 同时处理的请求上限 |
| `RATE_LIMIT_TRUST_PROXY` | `false` | `true` 时用 `CF-Connecting-IP` 或 `X-Forwarded-For` 最左一跳 |
| `RATE_LIMIT_HOME` / `_WINDOW` | `120` / `60` | 公开 GET `/api/site`、`/quotes`、`/persons` |
| `RATE_LIMIT_SUBMIT` / `_WINDOW` | `10` / `600` | `POST /api/submissions` |
| `RATE_LIMIT_LOGIN` / `_WINDOW` | `5` / `60` | `POST /api/admin/login`、`/setup` |
| `RATE_LIMIT_ADMIN` / `_WINDOW` | `120` / `60` | 其余 `/api/admin/*` |
| `RATE_LIMIT_UPLOADS` / `_WINDOW` | `240` / `60` | `GET /uploads/*` |

## 前后端一起开发

根目录：

```powershell
npm install
cargo install watchexec-cli   # 若尚未安装
.\scripts\dev.ps1
# 或
npm run dev
```

- 前台：`npm --prefix frontend run dev`（`VITE_API_URL` 为空时，`/api` 与 `/uploads` proxy 到 `127.0.0.1:3000`）
- 后台：`watchexec -e rs,toml -r --cwd backend cargo run`（只提供 API，不返回 HTML）

### 前端环境变量

Vite 会在**构建时**写入 `VITE_*`（Cloudflare Pages 在项目环境变量里设置即可覆盖）。

| 变量 | 默认 | 说明 |
|------|------|------|
| `VITE_API_URL` | 开发为空；生产 `https://api.shenren.de5.net` | API 源站，不要带路径。空 = 同源（走 Vite proxy）。末尾 `/` 可有可无 |

本地覆盖：复制 `frontend/.env.example` 为 `frontend/.env.development.local`，不要提交。

## 生产

```powershell
cd backend
$env:DATABASE_URL = "mysql://user:pass@host:3306/shenren"
$env:COOKIE_SECURE = "true"
$env:COOKIE_SAMESITE = "None"
$env:CORS_ORIGINS = "https://your.pages.example"
cargo run --release
```

前台：`npm --prefix frontend run build`（读取 `frontend/.env.production` 的 `VITE_API_URL`），产物交给 Nginx / Cloudflare Pages，不要再塞进 Axum。

Pages 构建示例：根目录，输出 `frontend/dist`，命令 `npm --prefix frontend ci && npm --prefix frontend run build`。项目环境变量设 `VITE_API_URL=https://api.shenren.de5.net` 可覆盖文件默认值。前后端不同源时，后端必须把 Pages Origin 写进 `CORS_ORIGINS`，并设 `COOKIE_SAMESITE=None`（管理端 Cookie 才能跨站带上）。

或 Docker（API 镜像，默认连宿主机 MySQL）：

```bash
export DATABASE_URL=mysql://user:pass@host.docker.internal:3306/shenren
export COOKIE_SECURE=true
export COOKIE_SAMESITE=None
export CORS_ORIGINS=https://your.pages.example
docker compose up -d --build
```

## CI

GitHub Actions（`.github/workflows/build.yml`）会构建：

- Linux `x86_64` 可执行文件 `shenren-x86_64-unknown-linux-gnu`
- Docker 镜像（仅 API：`cargo build --release`）

触发：

| 事件 | 行为 |
|------|------|
| `push` / `pull_request` | 构建镜像与二进制，上传 artifact；**不**推 GHCR，**不**起 MySQL |
| **手动** `workflow_dispatch` | 默认用 Compose MySQL 做 `/api/site` 启动探测，并推 GHCR（可在输入里关掉） |
| **任意 Release**（含预发布） | 同上，并把二进制挂到 Release；正式版额外打 `latest` |
| 预发布改为正式版 | `release.edited` 且 `prerelease: true → false` 时再跑一遍，补打 `latest` |

镜像：`ghcr.io/<owner>/<repo>`。手动运行可取消「Use Compose MySQL」以跳过探测（假定连宿主机 MySQL）。

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
