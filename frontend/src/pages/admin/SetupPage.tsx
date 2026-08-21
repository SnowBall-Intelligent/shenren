import { useEffect, useState } from 'react'
import { Link as RouterLink, useNavigate } from 'react-router-dom'
import {
  Alert,
  Box,
  Button,
  CircularProgress,
  Paper,
  Stack,
  TextField,
  Typography,
} from '@mui/material'
import { adminApi } from '../../api'
import { ApiError } from '../../api/client'
import { useToast } from '../../components/AppToast'

export default function SetupPage() {
  const navigate = useNavigate()
  const toast = useToast()
  const [checking, setChecking] = useState(true)
  const [closed, setClosed] = useState(false)
  const [username, setUsername] = useState('')
  const [password, setPassword] = useState('')
  const [password2, setPassword2] = useState('')
  const [submitting, setSubmitting] = useState(false)

  useEffect(() => {
    adminApi
      .bootstrapStatus()
      .then((s) => {
        if (!s.needs_setup) {
          setClosed(true)
        }
      })
      .catch((err) => {
        if (err instanceof ApiError && (err.status === 401 || err.status === 403)) {
          setClosed(true)
        }
      })
      .finally(() => setChecking(false))
  }, [])

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    if (password !== password2) {
      toast.error('两次输入的密码不一致')
      return
    }
    if (password.length < 6) {
      toast.error('密码至少 6 位')
      return
    }
    setSubmitting(true)
    try {
      const data = await adminApi.setup(username.trim(), password)
      toast.fromSuccess(data)
      navigate('/admin/quotes/review', { replace: true })
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

  if (closed) {
    return (
      <Box sx={{ maxWidth: 420, mx: 'auto', mt: 8, px: 2 }}>
        <Alert severity="info" sx={{ mb: 2 }}>
          站点已完成初始化，初始化页已关闭。
        </Alert>
        <Button component={RouterLink} to="/admin/login" variant="contained" fullWidth>
          前往登录
        </Button>
      </Box>
    )
  }

  return (
    <Box sx={{ maxWidth: 420, mx: 'auto', mt: 8, px: 2 }}>
      <Paper sx={{ p: 3 }}>
        <Typography variant="h5" sx={{ fontWeight: 700, mb: 1 }}>
          初始化超级管理员
        </Typography>
        <Typography variant="body2" color="text.secondary" sx={{ mb: 3 }}>
          首次部署时创建第一个管理员账号。创建后此页将不可用。
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
            autoComplete="new-password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
          />
          <TextField
            label="确认密码"
            type="password"
            required
            autoComplete="new-password"
            value={password2}
            onChange={(e) => setPassword2(e.target.value)}
          />
          <Button type="submit" variant="contained" size="large" disabled={submitting}>
            {submitting ? '创建中…' : '创建并进入后台'}
          </Button>
        </Stack>
      </Paper>
    </Box>
  )
}
