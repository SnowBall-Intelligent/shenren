import { useCallback, useEffect, useMemo, useState } from 'react'
import { Alert, Box, Button, CircularProgress, Stack, TextField, Typography } from '@mui/material'
import { useOutletContext } from 'react-router-dom'
import { adminApi } from '../../api'
import type { AdminMe, CaptchaPayload } from '../../api/types'
import { ApiError } from '../../api/client'
import { useToast } from '../../components/AppToast'
import CaptchaWidget, { publicCaptchaList } from '../../components/CaptchaWidget'

type AccountOutletContext = {
  me: AdminMe
  reloadMe: () => Promise<void>
}

export default function AccountPage() {
  const { me, reloadMe } = useOutletContext<AccountOutletContext>()
  const toast = useToast()
  const [accountInfo, setAccountInfo] = useState<AdminMe | null>(null)
  const [loadError, setLoadError] = useState<string | null>(null)
  const [username, setUsername] = useState(me.username)
  const [currentPassword, setCurrentPassword] = useState('')
  const [newPassword, setNewPassword] = useState('')
  const [confirmPassword, setConfirmPassword] = useState('')
  const [captcha, setCaptcha] = useState<CaptchaPayload | null>(null)
  const [captchaKey, setCaptchaKey] = useState(0)
  const [skipSignal, setSkipSignal] = useState(0)
  const [saving, setSaving] = useState(false)

  useEffect(() => {
    adminApi
      .me()
      .then((current) => {
        setAccountInfo(current)
        setUsername(current.username)
      })
      .catch((error) =>
        setLoadError(error instanceof ApiError ? error.message : '无法加载账号设置'),
      )
  }, [])

  const captchaProviders = useMemo(
    () => publicCaptchaList(accountInfo?.captcha),
    [accountInfo?.captcha],
  )
  const captchaRequired = captchaProviders.length > 0
  const usernameChanged = username.trim() !== accountInfo?.username
  const passwordChanged = newPassword.length > 0
  const passwordsMatch = newPassword === confirmPassword
  const canSubmit =
    username.trim().length > 0 &&
    currentPassword.length > 0 &&
    (usernameChanged || passwordChanged) &&
    passwordsMatch &&
    (!captchaRequired || captcha != null)

  const resetCaptcha = () => {
    setCaptcha(null)
    setCaptchaKey((value) => value + 1)
  }

  const handleCaptchaChange = useCallback((payload: CaptchaPayload | null) => {
    setCaptcha(payload)
  }, [])

  const handleSubmit = async (event: React.FormEvent) => {
    event.preventDefault()
    if (!canSubmit) return
    setSaving(true)
    try {
      const updated = await adminApi.updateMe({
        username: username.trim(),
        current_password: currentPassword,
        new_password: newPassword || null,
        captcha: captcha ?? undefined,
      })
      setAccountInfo(updated)
      setUsername(updated.username)
      setCurrentPassword('')
      setNewPassword('')
      setConfirmPassword('')
      resetCaptcha()
      toast.fromSuccess(updated)
      await reloadMe()
    } catch (error) {
      if (error instanceof ApiError && error.body?.captcha_fallback) {
        setCaptcha(null)
        setSkipSignal((value) => value + 1)
      } else {
        toast.fromError(error)
        resetCaptcha()
      }
    } finally {
      setSaving(false)
    }
  }

  if (loadError) {
    return <Alert severity="error">{loadError}</Alert>
  }

  if (!accountInfo) {
    return (
      <Box sx={{ display: 'flex', justifyContent: 'center', py: 6 }}>
        <CircularProgress />
      </Box>
    )
  }

  return (
    <Box component="form" onSubmit={(event) => void handleSubmit(event)} sx={{ maxWidth: 560 }}>
      <Stack spacing={2.5}>
        <Alert severity="info">
          修改用户名或密码时需要当前密码。更新成功后当前登录状态保持不变。
        </Alert>
        <TextField
          label="用户名"
          required
          autoComplete="username"
          value={username}
          onChange={(event) => setUsername(event.target.value)}
          slotProps={{ htmlInput: { maxLength: 64 } }}
        />
        <TextField
          label="当前密码"
          type="password"
          required
          autoComplete="current-password"
          value={currentPassword}
          onChange={(event) => setCurrentPassword(event.target.value)}
        />
        <TextField
          label="新密码（可选）"
          type="password"
          autoComplete="new-password"
          value={newPassword}
          onChange={(event) => setNewPassword(event.target.value)}
          helperText="留空表示不修改密码；新密码至少 6 位。"
          slotProps={{ htmlInput: { maxLength: 128 } }}
        />
        <TextField
          label="确认新密码"
          type="password"
          autoComplete="new-password"
          value={confirmPassword}
          onChange={(event) => setConfirmPassword(event.target.value)}
          error={confirmPassword.length > 0 && !passwordsMatch}
          helperText={confirmPassword.length > 0 && !passwordsMatch ? '两次输入的新密码不一致' : ' '}
          slotProps={{ htmlInput: { maxLength: 128 } }}
        />

        {captchaRequired ? (
          <Stack spacing={1.5}>
            <Typography variant="body2" color="text.secondary">
              系统已要求账号修改前完成人机验证。
            </Typography>
            <CaptchaWidget
              key={captchaKey}
              providers={captchaProviders}
              skipSignal={skipSignal}
              onChange={handleCaptchaChange}
              onExhausted={(message) => toast.error(message)}
            />
          </Stack>
        ) : null}

        <Button
          type="submit"
          variant="contained"
          disabled={!canSubmit || saving}
          sx={{ alignSelf: 'flex-start' }}
        >
          {saving ? '保存中…' : '保存账号'}
        </Button>
      </Stack>
    </Box>
  )
}
