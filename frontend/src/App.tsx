import { lazy, Suspense } from 'react'
import { BrowserRouter, Navigate, Route, Routes } from 'react-router-dom'
import { Box, CircularProgress, CssBaseline, ThemeProvider } from '@mui/material'
import { theme } from './theme'
import PublicLayout from './layouts/PublicLayout'
import HomePage from './pages/HomePage'
import { ToastProvider } from './components/AppToast'

const AdminLayout = lazy(() => import('./layouts/AdminLayout'))
const SetupPage = lazy(() => import('./pages/admin/SetupPage'))
const LoginPage = lazy(() => import('./pages/admin/LoginPage'))
const QuotesPage = lazy(() => import('./pages/admin/QuotesPage'))
const PersonsPage = lazy(() => import('./pages/admin/PersonsPage'))
const SettingsPage = lazy(() => import('./pages/admin/SettingsPage'))
const CaptchaSettingsPage = lazy(() => import('./pages/admin/CaptchaSettingsPage'))
const AdminsPage = lazy(() => import('./pages/admin/AdminsPage'))

function RouteFallback() {
  return (
    <Box sx={{ display: 'flex', justifyContent: 'center', py: 10 }}>
      <CircularProgress />
    </Box>
  )
}

export default function App() {
  return (
    <ThemeProvider theme={theme}>
      <CssBaseline />
      <ToastProvider>
      <BrowserRouter>
        <Suspense fallback={<RouteFallback />}>
        <Routes>
          <Route element={<PublicLayout />}>
            <Route index element={<HomePage />} />
            <Route path="submit" element={<HomePage />} />
          </Route>

          <Route path="/admin/setup" element={<SetupPage />} />
          <Route path="/admin/login" element={<LoginPage />} />

          <Route path="/admin" element={<AdminLayout />}>
            <Route index element={<Navigate to="quotes/review" replace />} />
            <Route path="quotes" element={<Navigate to="review" replace />} />
            <Route path="quotes/review" element={<QuotesPage variant="review" />} />
            <Route path="quotes/list" element={<QuotesPage variant="list" />} />
            <Route path="quotes/new" element={<Navigate to="/admin/quotes/list" replace />} />
            <Route path="persons" element={<PersonsPage />} />
            <Route path="settings" element={<SettingsPage />} />
            <Route path="settings/captcha" element={<CaptchaSettingsPage />} />
            <Route path="admins" element={<AdminsPage />} />
          </Route>

          <Route path="*" element={<Navigate to="/" replace />} />
        </Routes>
        </Suspense>
      </BrowserRouter>
      </ToastProvider>
    </ThemeProvider>
  )
}
