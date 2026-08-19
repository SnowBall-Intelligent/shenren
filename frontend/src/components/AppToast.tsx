import { createContext, useCallback, useContext, useMemo, useState, type ReactNode } from 'react'
import { Alert, Snackbar } from '@mui/material'
import { ApiError } from '../api/client'

type ToastSeverity = 'success' | 'error' | 'info'

type ToastApi = {
  success: (message: string) => void
  error: (message: string) => void
  info: (message: string) => void
  fromSuccess: (data: { message?: string } | null | undefined) => void
  fromError: (err: unknown) => void
}

const ToastContext = createContext<ToastApi | null>(null)

export function useToast(): ToastApi {
  const ctx = useContext(ToastContext)
  if (!ctx) {
    throw new Error('useToast must be used within ToastProvider')
  }
  return ctx
}

export function ToastProvider({ children }: { children: ReactNode }) {
  const [open, setOpen] = useState(false)
  const [severity, setSeverity] = useState<ToastSeverity>('success')
  const [message, setMessage] = useState('')

  const show = useCallback((next: ToastSeverity, nextMessage: string) => {
    if (!nextMessage) return
    setSeverity(next)
    setMessage(nextMessage)
    setOpen(true)
  }, [])

  const api = useMemo<ToastApi>(
    () => ({
      success: (m) => show('success', m),
      error: (m) => show('error', m),
      info: (m) => show('info', m),
      fromSuccess: (data) => {
        if (data?.message) show('success', data.message)
      },
      fromError: (err) => {
        if (err instanceof ApiError && err.message) {
          show('error', err.message)
          return
        }
        if (err instanceof Error && err.message) {
          show('error', err.message)
        }
      },
    }),
    [show],
  )

  const handleClose = (_?: unknown, reason?: string) => {
    if (reason === 'clickaway') return
    setOpen(false)
  }

  return (
    <ToastContext.Provider value={api}>
      {children}
      <Snackbar
        open={open}
        autoHideDuration={3000}
        onClose={handleClose}
        anchorOrigin={{ vertical: 'top', horizontal: 'right' }}
      >
        <Alert onClose={handleClose} severity={severity} variant="filled" sx={{ width: '100%' }}>
          {message}
        </Alert>
      </Snackbar>
    </ToastContext.Provider>
  )
}
