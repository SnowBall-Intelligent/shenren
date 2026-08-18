import { useCallback, useEffect, useState } from 'react'
import { Link as RouterLink, Navigate, Outlet, useLocation, useNavigate } from 'react-router-dom'
import {
  AppBar,
  Box,
  Divider,
  Drawer,
  IconButton,
  List,
  ListItemButton,
  ListItemIcon,
  ListItemText,
  Toolbar,
  Typography,
  Button,
  CircularProgress,
  useMediaQuery,
  useTheme,
} from '@mui/material'
import MenuIcon from '@mui/icons-material/Menu'
import FormatQuoteIcon from '@mui/icons-material/FormatQuote'
import PeopleIcon from '@mui/icons-material/People'
import SettingsIcon from '@mui/icons-material/Settings'
import AdminPanelSettingsIcon from '@mui/icons-material/AdminPanelSettings'
import LogoutIcon from '@mui/icons-material/Logout'
import HomeIcon from '@mui/icons-material/Home'
import { adminApi } from '../api'
import type { AdminMe } from '../api/types'
import { ApiError } from '../api/client'

const DRAWER_WIDTH = 240

const navItems = [
  { to: '/admin/quotes', label: '言论审核', icon: <FormatQuoteIcon /> },
  { to: '/admin/persons', label: '神人管理', icon: <PeopleIcon /> },
  { to: '/admin/settings', label: '站点设置', icon: <SettingsIcon /> },
  { to: '/admin/admins', label: '管理员', icon: <AdminPanelSettingsIcon /> },
]

export default function AdminLayout() {
  const theme = useTheme()
  const isMobile = useMediaQuery(theme.breakpoints.down('md'))
  const [mobileOpen, setMobileOpen] = useState(false)
  const [me, setMe] = useState<AdminMe | null>(null)
  const [loading, setLoading] = useState(true)
  const [unauthorized, setUnauthorized] = useState(false)
  const location = useLocation()
  const navigate = useNavigate()

  const loadMe = useCallback(async () => {
    setLoading(true)
    try {
      const user = await adminApi.me()
      setMe(user)
      setUnauthorized(false)
    } catch (e) {
      if (e instanceof ApiError && (e.status === 401 || e.status === 403)) {
        setUnauthorized(true)
      } else {
        setUnauthorized(true)
      }
      setMe(null)
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    void loadMe()
  }, [loadMe])

  const handleLogout = async () => {
    try {
      await adminApi.logout()
    } catch {
      /* ignore */
    }
    navigate('/admin/login', { replace: true })
  }

  if (loading) {
    return (
      <Box sx={{ display: 'flex', justifyContent: 'center', py: 10 }}>
        <CircularProgress />
      </Box>
    )
  }

  if (unauthorized || !me) {
    return <Navigate to="/admin/login" replace state={{ from: location.pathname }} />
  }

  const drawer = (
    <Box sx={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      <Toolbar>
        <Typography variant="subtitle1" sx={{ fontWeight: 700 }}>
          管理后台
        </Typography>
      </Toolbar>
      <Divider />
      <List sx={{ flex: 1 }}>
        {navItems.map((item) => (
          <ListItemButton
            key={item.to}
            component={RouterLink}
            to={item.to}
            selected={location.pathname.startsWith(item.to)}
            onClick={() => setMobileOpen(false)}
          >
            <ListItemIcon sx={{ minWidth: 40 }}>{item.icon}</ListItemIcon>
            <ListItemText primary={item.label} />
          </ListItemButton>
        ))}
      </List>
      <Divider />
      <List>
        <ListItemButton component={RouterLink} to="/" onClick={() => setMobileOpen(false)}>
          <ListItemIcon sx={{ minWidth: 40 }}>
            <HomeIcon />
          </ListItemIcon>
          <ListItemText primary="返回前台" />
        </ListItemButton>
        <ListItemButton onClick={() => void handleLogout()}>
          <ListItemIcon sx={{ minWidth: 40 }}>
            <LogoutIcon />
          </ListItemIcon>
          <ListItemText primary="退出登录" />
        </ListItemButton>
      </List>
    </Box>
  )

  return (
    <Box sx={{ display: 'flex', minHeight: '100vh', bgcolor: 'background.default' }}>
      <AppBar
        position="fixed"
        sx={{
          width: { md: `calc(100% - ${DRAWER_WIDTH}px)` },
          ml: { md: `${DRAWER_WIDTH}px` },
          bgcolor: '#1a1a1a',
          borderBottom: '1px solid #2a2a2a',
        }}
        elevation={0}
      >
        <Toolbar>
          {isMobile ? (
            <IconButton color="inherit" edge="start" onClick={() => setMobileOpen(true)} sx={{ mr: 1 }}>
              <MenuIcon />
            </IconButton>
          ) : null}
          <Typography variant="h6" sx={{ flexGrow: 1 }}>
            {navItems.find((n) => location.pathname.startsWith(n.to))?.label ?? '后台'}
          </Typography>
          <Typography variant="body2" color="text.secondary" sx={{ mr: 2 }}>
            {me.username}
          </Typography>
          <Button color="inherit" size="small" onClick={() => void handleLogout()} startIcon={<LogoutIcon />}>
            退出
          </Button>
        </Toolbar>
      </AppBar>

      <Box component="nav" sx={{ width: { md: DRAWER_WIDTH }, flexShrink: { md: 0 } }}>
        <Drawer
          variant="temporary"
          open={mobileOpen}
          onClose={() => setMobileOpen(false)}
          ModalProps={{ keepMounted: true }}
          sx={{
            display: { xs: 'block', md: 'none' },
            '& .MuiDrawer-paper': { width: DRAWER_WIDTH, boxSizing: 'border-box' },
          }}
        >
          {drawer}
        </Drawer>
        <Drawer
          variant="permanent"
          open
          sx={{
            display: { xs: 'none', md: 'block' },
            '& .MuiDrawer-paper': { width: DRAWER_WIDTH, boxSizing: 'border-box' },
          }}
        >
          {drawer}
        </Drawer>
      </Box>

      <Box
        component="main"
        sx={{
          flexGrow: 1,
          p: { xs: 2, md: 3 },
          width: { md: `calc(100% - ${DRAWER_WIDTH}px)` },
          mt: 8,
        }}
      >
        <Outlet context={{ me, reloadMe: loadMe }} />
      </Box>
    </Box>
  )
}
