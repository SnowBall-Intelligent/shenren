# AGENTS.md

## 仓库结构

- `backend/`：Rust Axum API、SeaORM 实体与迁移。
- `frontend/`：Vite + React + MUI 前台和管理后台。
- `e2e/`：独立 Playwright 测试套件，包含真实 API 与浏览器测试。
- `.github/workflows/test.yml`：CI 的 API + Web 全量测试。
- `.github/workflows/build.yml`：构建产物并推送 GHCR。

## 修改原则

- 测试应验证真实产品行为。不要为了让测试通过而在 `backend/src` 或前端页面中加入仅测试使用的分支。
- 保持 SQLite 与 MySQL 迁移兼容；数据结构变更必须新增迁移并同步 SeaORM 实体。
- 迁移变更必须维护 `backend/migration/tests/`，并确保 CI 的 SQLite 与 MySQL 迁移任务均通过；测试只能使用专用空数据库。
- 前后端接口字段、校验与错误语义必须同步，避免只修改一端。
- 新增后端功能时必须评估并补充合适等级的运行日志；新增登录流程或后台写操作时必须写入管理员审计日志。
- 日志不得记录密码、Cookie、Session、Authorization、完整 API Key、验证码密钥、令牌、完整请求体或言论正文；可能含敏感信息的字段必须使用统一脱敏函数。
- 日志行为变更必须覆盖成功路径，以及轮转、配置错误、审计失败结果或脱敏等至少一条边界路径。
- 不要覆盖或回退工作区中与当前任务无关的已有修改。

## 测试要求

- 新功能或行为变更必须在 `e2e/` 增加成功路径，以及至少一条失败或边界路径。
- API 用例放在 `e2e/api/*.spec.ts`，通过 Playwright `request` 访问真实 `/api/*`。
- Web 用例放在 `e2e/web/*.spec.ts`，只在 CI 运行。
- 本地不安装或启动浏览器，不运行 `playwright install`、Chromium 或 Vite 浏览器测试。
- 所有改动完成后必须在仓库根目录运行 `npm test`，不能只做编译检查。
- CI 使用 `npm --prefix e2e run test:ci` 运行 API + Web。

## 按改动范围追加检查

- Rust 代码或迁移：运行 `cargo fmt --all -- --check` 和 `cargo check`（工作目录 `backend/`）。
- 前端代码：运行 `npm run lint` 和 `npm run build`（工作目录 `frontend/`）。
- 不要求把 Clippy 作为合并门禁；若主动运行，应区分既有告警与本次新增问题。
