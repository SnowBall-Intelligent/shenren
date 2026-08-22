/// <reference types="vite/client" />

interface ImportMetaEnv {
  /** API origin, e.g. `https://api.shenren.de5.net`. Empty = same-origin. */
  readonly VITE_API_URL?: string
}

interface ImportMeta {
  readonly env: ImportMetaEnv
}
