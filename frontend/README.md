# 神人网前端

Vite + React + TypeScript + MUI。深色聊天气泡首页与管理后台。

## 开发

后端默认 `http://127.0.0.1:3000`。Vite 已将 `/api` 与 `/uploads` 代理到该地址。

```bash
cd frontend
npm install
npm run dev
```

浏览器打开 Vite 提示的地址（通常 `http://127.0.0.1:5173`）。请求带 Cookie（`credentials: 'include'`）以支持后台 Session。

## 构建

```bash
npm run build
```

产物在 `dist/`，生产可由 Axum 静态托管。

## 页面

| 路径 | 说明 |
|------|------|
| `/` | 已审核言论气泡流 |
| `/submit` | 投稿 |
| `/admin/setup` | 首次初始化超管 |
| `/admin/login` | 登录 |
| `/admin/quotes` | 言论审核 |
| `/admin/persons` | 神人 CRUD |
| `/admin/settings` | 站点设置 |
| `/admin/admins` | 管理员 |
