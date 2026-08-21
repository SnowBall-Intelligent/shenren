import { useEffect, useState } from 'react'
import { Link as RouterLink, useNavigate, useLocation } from 'react-router-dom'
import {
  Box,
  Button,
  CircularProgress,
  Link,
  Paper,
  Stack,
  TextField,
  Typography,
} from '@mui/material'
import { adminApi } from '../../api'
import { useToast } from '../../components/AppToast'

export default function LoginPage() {
  const navigate = useNavigate()
  const location = useLocation()
  const toast = useToast()
  const from = (location.state as { from?: string } | null)?.from ?? '/admin/quotes/review'

  const [checking, setChecking] = useState(true)
  const [username, setUsername] = useState('')
  const [password, setPassword] = useState('')
  const [submitting, setSubmitting] = useState(false)

  useEffect(() => {
    let cancelled = false
    ;(async () => {
      try {
        await adminApi.me()
        if (!cancelled) navigate(from, { replace: true })
      } catch {
        /* show login */
      } finally {
        if (!cancelled) setChecking(false)
      }
    })()
    return () => {
      cancelled = true
    }
  }, [from, navigate])

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    setSubmitting(true)
    try {
      const data = await adminApi.login(username.trim(), password)
      toast.fromSuccess(data)
      navigate(from, { replace: true })
    } catch (err) {
      toast.fromError(err)
    } finally {
      setSubmitting(false)
    }
  }

  if (checking) {
    return (
      <Box sx={{ display: 'flex', justifyContent: 'center', py: 10 }}>
        <CircularProgress />
      </Box>
    )
  }

  return (
    <Box sx={{ maxWidth: 420, mx: 'auto', mt: 8, px: 2 }}>
      <Paper sx={{ p: 3 }}>
        <Typography variant="h5" sx={{ fontWeight: 700, mb: 2 }}>
          管理员登录
        </Typography>
        <Stack component="form" spacing={2} onSubmit={(e) => void handleSubmit(e)}>
          <TextField
            label="用户名"
            required
            autoComplete="username"
            value={username}
            onChange={(e) => setUsername(e.target.value)}
          />
          <TextField
            label="密码"
            type="password"
            required
            autoComplete="current-password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
          />
          <Button type="submit" variant="contained" size="large" disabled={submitting}>
            {submitting ? '登录中…' : '登录'}
          </Button>
        </Stack>
        <Box sx={{ mt: 2, textAlign: 'center' }}>
          <Link component={RouterLink} to="/admin/setup" underline="hover" variant="body2" color="text.secondary">
            首次安装
          </Link>
          {' · '}
          <Link component={RouterLink} to="/" underline="hover" variant="body2" color="text.secondary">
            返回首页
          </Link>
        </Box>
      </Paper>
    </Box>
  )
}
