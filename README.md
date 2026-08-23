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
| `LOG_ENABLED` | `true` | 是否启用控制台、系统文件和管理员审计日志。支持 `true/false`、`1/0`、`yes/no` |
| `LOG_LEVEL` | `info` | 最低日志等级：`error` / `warn` / `info` / `debug` / `trace` |
| `LOG_TIMEZONE` | `UTC` | 日志时间及文件名日期使用的 IANA 时区名，例如 `Asia/Hong_Kong` |
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

### 系统日志

日志启用时同时输出到控制台和文件。通用运行及 HTTP 摘要写入 `data/logs/system-YYYY-MM-DD-NNN.log`，登录、退出和后台写操作写入 `data/logs/admin/audit-YYYY-MM-DD-NNN.log`。审计日志不会重复写入系统文件。

每个文件最大 `1 MiB`，达到上限后递增三位分段号；日期按 `LOG_TIMEZONE` 计算。程序启动和跨日首次写入时自动清理 30 天前、且符合上述命名规则的日志。日志初始化失败会阻止启动，运行期写盘失败则降级到 `stderr`。

请求日志只记录方法、不含查询参数的路径、状态、客户端 IP 和耗时。管理员审计日志只记录操作者、动作、目标 ID 和结果，不记录请求体、言论正文、密码、Cookie、Session、Authorization、完整 API Key 或验证码密钥。

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

前台与管理后台均支持跟随系统、浅色和深色三种外观模式。默认跟随系统，手动选择会保存在浏览器中；原有深色外观作为夜间模式保留。

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
$env:LOG_TIMEZONE = "Asia/Hong_Kong"
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

## 测试

独立套件在 `e2e/`（不改后端源码）：**API**（`e2e/api`）+ **浏览器**（`e2e/web`）。

本地默认只跑 API（不起浏览器、不起 Vite）：

```powershell
npm --prefix e2e ci
npm test
```

浏览器用例只在 CI 跑（`.github/workflows/test.yml` 会装 Chromium 并执行 `npm run test:ci`）。Test 工作流还会从旧版数字语录 ID 结构开始，分别在 SQLite 与 MySQL 8.4 上插入测试数据并执行完整迁移。

## CI

- **Test**（`.github/workflows/test.yml`）：每次提交跑上述 API + 浏览器 e2e。
- **Build**（`.github/workflows/build.yml`）：Linux 二进制 + Docker 镜像推 GHCR。

| 事件 | 推镜像 | 镜像 tag | MySQL 冒烟 | Release 附件 |
|------|--------|----------|------------|--------------|
| `push` / 同仓库 PR | 会 | `dev-sha-<short>`，PR 另有 `dev-pr-N` | 否 | 否 |
| 手动 `workflow_dispatch` | 默认会 | `dev-sha-<short>` + `dev-manual` | 默认会 | 否 |
| **任意 Release**（含预发布） | 会 | **Release 的 tag 名**（正式版另打 `latest`） | 会 | 会 |
| 预发布改为正式版 | 会 | 补打 `latest` | 会 | 会 |

开发镜像只保留最近 10 个。删除前会对照 Release 的 tag 名和名称，命中的以及 `latest` 不删。Fork PR 不推 GHCR。

镜像：`ghcr.io/<owner>/<repo>`。手动运行可取消「Use Compose MySQL」以跳过探测。

## 站点 API

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/site` | 站点信息（含 `allow_propose_person`） |
| GET | `/api/quotes?page=&page_size=` | 已通过言论（与首页展示顺序一致） |
| GET | `/api/persons` | 神人列表（投稿下拉） |
| POST | `/api/submissions` | 投稿（按 IP 内存限流） |
| GET | `/uploads/...` | 头像静态文件 |

投稿提出新神人时可传 `proposed_person_qq`。管理端创建或更新神人、审核时可在 multipart 中传 `qq`，JSON 审核也支持 `qq`。服务端只保存以下 QQ 头像 CDN 链接，不保存 QQ 号，也不会下载头像文件：

```text
https://q2.qlogo.cn/headimg_dl?dst_uin=<QQ>&spec=0
```

## 外部语录 API v1

外部 API 使用管理后台生成的 API Key，调用时通过 `Authorization: Bearer <key>` 传入。完整 Key 只在创建响应中显示一次，数据库仅保存 SHA-256 哈希和前缀。

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/v1/quotes` | 按首页顺序分页获取已通过语录 |
| GET | `/api/v1/quotes/random` | 随机获取一条已通过语录；无匹配项时返回 `404` |

分页接口参数：

| 参数 | 默认 | 说明 |
|------|------|------|
| `page` | `1` | 从 1 开始 |
| `page_size` | `20` | 1-100 |
| `from` | 无 | RFC3339 时间，包含边界，按 `published_at` 筛选 |
| `to` | 无 | RFC3339 时间，包含边界，按 `published_at` 筛选 |
| `person_id` | 无 | 为后续版本预留；v1 传入会明确返回 `400` |

随机接口支持相同的 `from`、`to` 和预留 `person_id` 参数。

```bash
curl -H "Authorization: Bearer srk_xxx" \
  "https://api.example.com/api/v1/quotes?page=1&page_size=20"

curl -H "Authorization: Bearer srk_xxx" \
  "https://api.example.com/api/v1/quotes/random?from=2026-01-01T00%3A00%3A00Z"
```

每个 Key 可独立设置：

- 滑动窗口频率 `N / window_seconds`，留空表示不限。
- 总额度，留空表示无限；使用量持久化到数据库，可由管理员重置。
- IP 白名单，支持精确 IP 和 CIDR。
- 来源域名白名单，按 hostname 匹配并忽略协议、端口、大小写和末尾点；`example.com` 精确匹配，`*.example.com` 只匹配子域。
- 单实例并发上限。频率与并发状态保存在当前进程内，多实例部署时各实例独立计算。

响应可能包含 `X-RateLimit-Limit`、`X-RateLimit-Remaining`、`X-RateLimit-Reset`、`X-Quota-Limit` 和 `X-Quota-Remaining`。触发限制时返回 `429`，频率或并发限制还会尽可能返回 `Retry-After`。

## 管理 API

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/admin/bootstrap-status` | 是否需要初始化 |
| POST | `/api/admin/setup` | 无管理员时创建超管并登录 |
| POST | `/api/admin/login` / `logout` | Cookie Session（HttpOnly + SameSite=Lax） |
| GET | `/api/admin/me` | 当前管理员 |
| CRUD | `/api/admin/persons` | 神人（multipart 头像） |
| CRUD | `/api/admin/api-keys` | 外部 API Key 与限制配置 |
| POST | `/api/admin/api-keys/:id/reset-usage` | 重置 Key 的持久化使用量 |
| GET/PUT | `/api/admin/settings` | 站点配置 |
| CRUD | `/api/admin/admins` | 管理员（不可删光最后一个；删自己时若仍有他人则允许并注销） |
| GET | `/api/admin/quotes` | 审核列表 |
| POST | `/api/admin/quotes/:id/approve` | multipart：新神人需 `avatar`，或 `person_id` 绑定 |
| POST | `/api/admin/quotes/:id/approve-json` | JSON 绑定已有神人 / 已有 `person_id` 直接通过 |
| POST | `/api/admin/quotes/:id/reject` | 驳回 |

密码使用 Argon2。
