# AGENTS.md

## 仓库

- `backend/`：Axum API。不要为了测试改 `backend/src`。
- `frontend/`：Vite + MUI。不要为了测试去改页面实现。
- `e2e/`：独立测试套件（API + Web）。
- `.github/workflows/test.yml`：CI 跑全部 e2e。
- `.github/workflows/build.yml`：构建并推 GHCR。

## 测试

- 新功能或行为变更必须在 `e2e/` 补用例：成功路径 + 至少一条失败/边界。
- API：`e2e/api/*.spec.ts`，用 Playwright `request` 打真实 `/api/*`。
- Web：`e2e/web/*.spec.ts`，只在 **CI** 跑。
- **本地不跑浏览器**：不要 `playwright install`、不要起 Chromium / Vite 来测。
- 做完后必须跑：`npm test`（只跑 API）。不要只编译。
- CI 跑 `npm --prefix e2e run test:ci`（API + Web）。
