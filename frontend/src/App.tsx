import { BrowserRouter, Navigate, Route, Routes } from 'react-router-dom'
import { CssBaseline, ThemeProvider } from '@mui/material'
import { theme } from './theme'
import PublicLayout from './layouts/PublicLayout'
import AdminLayout from './layouts/AdminLayout'
import HomePage from './pages/HomePage'
import SubmitPage from './pages/SubmitPage'
import SetupPage from './pages/admin/SetupPage'
import LoginPage from './pages/admin/LoginPage'
import QuotesPage from './pages/admin/QuotesPage'
import PersonsPage from './pages/admin/PersonsPage'
import SettingsPage from './pages/admin/SettingsPage'
import AdminsPage from './pages/admin/AdminsPage'

export default function App() {
  return (
    <ThemeProvider theme={theme}>
      <CssBaseline />
      <BrowserRouter>
        <Routes>
          <Route element={<PublicLayout />}>
            <Route index element={<HomePage />} />
            <Route path="submit" element={<SubmitPage />} />
          </Route>

          <Route path="/admin/setup" element={<SetupPage />} />
          <Route path="/admin/login" element={<LoginPage />} />

          <Route path="/admin" element={<AdminLayout />}>
            <Route index element={<Navigate to="quotes" replace />} />
            <Route path="quotes" element={<QuotesPage />} />
            <Route path="persons" element={<PersonsPage />} />
            <Route path="settings" element={<SettingsPage />} />
            <Route path="admins" element={<AdminsPage />} />
          </Route>

          <Route path="*" element={<Navigate to="/" replace />} />
        </Routes>
      </BrowserRouter>
    </ThemeProvider>
  )
}
