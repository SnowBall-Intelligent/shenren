import { BrowserRouter, Navigate, Route, Routes } from 'react-router-dom'
import { CssBaseline, ThemeProvider } from '@mui/material'
import { theme } from './theme'
import PublicLayout from './layouts/PublicLayout'
import AdminLayout from './layouts/AdminLayout'
import HomePage from './pages/HomePage'
import SetupPage from './pages/admin/SetupPage'
import LoginPage from './pages/admin/LoginPage'
import QuotesPage from './pages/admin/QuotesPage'
import PersonsPage from './pages/admin/PersonsPage'
import SettingsPage from './pages/admin/SettingsPage'
import AdminsPage from './pages/admin/AdminsPage'
import { ToastProvider } from './components/AppToast'

export default function App() {
  return (
    <ThemeProvider theme={theme}>
      <CssBaseline />
      <ToastProvider>
      <BrowserRouter>
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
            <Route path="admins" element={<AdminsPage />} />
          </Route>

          <Route path="*" element={<Navigate to="/" replace />} />
        </Routes>
      </BrowserRouter>
      </ToastProvider>
    </ThemeProvider>
  )
}
