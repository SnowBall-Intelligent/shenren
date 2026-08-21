import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import { compression } from 'vite-plugin-compression2'

export default defineConfig({
  plugins: [
    react(),
    compression({
      algorithms: ['gzip'],
      threshold: 0,
      include: /\.(html|js|mjs|json|css|svg)$/i,
    }),
  ],
  build: {
    cssCodeSplit: true,
    rollupOptions: {
      output: {
        manualChunks(id) {
          if (!id.includes('node_modules')) return
          if (id.includes('react-markdown') || id.includes('remark-') || id.includes('rehype-')) {
            return 'markdown'
          }
          if (id.includes('@mui') || id.includes('@emotion')) {
            return 'mui'
          }
          if (id.includes('react-dom') || /[\\/]react[\\/]/.test(id) || id.includes('scheduler')) {
            return 'react'
          }
        },
      },
    },
  },
  server: {
    port: 5173,
    proxy: {
      '/api': {
        target: 'http://127.0.0.1:3000',
        changeOrigin: true,
      },
      '/uploads': {
        target: 'http://127.0.0.1:3000',
        changeOrigin: true,
      },
    },
  },
})
