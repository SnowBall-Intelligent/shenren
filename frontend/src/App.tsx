import { lazy, Suspense } from 'react'
import { BrowserRouter, Navigate, Route, Routes } from 'react-router-dom'
import { Box, CircularProgress } from '@mui/material'
import PublicLayout from './layouts/PublicLayout'
import HomePage from './pages/HomePage'
import { ToastProvider } from './components/AppToast'
import { ThemeModeProvider } from './components/ThemeModeProvider'

const AdminLayout = lazy(() => import('./layouts/AdminLayout'))
const SetupPage = lazy(() => import('./pages/admin/SetupPage'))
const LoginPage = lazy(() => import('./pages/admin/LoginPage'))
const QuotesPage = lazy(() => import('./pages/admin/QuotesPage'))
const PersonsPage = lazy(() => import('./pages/admin/PersonsPage'))
const AccountPage = lazy(() => import('./pages/admin/AccountPage'))
const SettingsPage = lazy(() => import('./pages/admin/SettingsPage'))
const CaptchaSettingsPage = lazy(() => import('./pages/admin/CaptchaSettingsPage'))
const AdminsPage = lazy(() => import('./pages/admin/AdminsPage'))
const ApiKeysPage = lazy(() => import('./pages/admin/ApiKeysPage'))

function RouteFallback() {
  return (
    <Box sx={{ display: 'flex', justifyContent: 'center', py: 10 }}>
      <CircularProgress />
    </Box>
  )
}

export default function App() {
  return (
    <ThemeModeProvider>
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
            <Route path="account" element={<AccountPage />} />
            <Route path="settings" element={<SettingsPage />} />
            <Route path="settings/captcha" element={<CaptchaSettingsPage />} />
            <Route path="admins" element={<AdminsPage />} />
            <Route path="api-keys" element={<ApiKeysPage />} />
          </Route>

          <Route path="*" element={<Navigate to="/" replace />} />
        </Routes>
        </Suspense>
      </BrowserRouter>
      </ToastProvider>
    </ThemeModeProvider>
  )
}
