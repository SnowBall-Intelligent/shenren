import { useCallback, useEffect, useState, type ReactNode } from 'react'
import { Link as RouterLink, Navigate, Outlet, useLocation, useNavigate } from 'react-router-dom'
import {
  AppBar,
  Box,
  Collapse,
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
import RateReviewIcon from '@mui/icons-material/RateReview'
import ListAltIcon from '@mui/icons-material/ListAlt'
import ExpandLess from '@mui/icons-material/ExpandLess'
import ExpandMore from '@mui/icons-material/ExpandMore'
import PeopleIcon from '@mui/icons-material/People'
import SettingsIcon from '@mui/icons-material/Settings'
import InfoOutlinedIcon from '@mui/icons-material/InfoOutlined'
import VerifiedUserIcon from '@mui/icons-material/VerifiedUser'
import AdminPanelSettingsIcon from '@mui/icons-material/AdminPanelSettings'
import LogoutIcon from '@mui/icons-material/Logout'
import HomeIcon from '@mui/icons-material/Home'
import VpnKeyIcon from '@mui/icons-material/VpnKey'
import { adminApi } from '../api'
import type { AdminMe } from '../api/types'
import { ApiError } from '../api/client'
import ThemeModeButton from '../components/ThemeModeButton'

const DRAWER_WIDTH = 240

type NavLeaf = { to: string; label: string; icon: ReactNode }
type NavGroup = { id: string; label: string; icon: ReactNode; match: string; children: NavLeaf[] }
type NavItem = NavLeaf | NavGroup

function isNavGroup(item: NavItem): item is NavGroup {
  return 'children' in item
}

const navItems: NavItem[] = [
  {
    id: 'quotes',
    label: '言论管理',
    icon: <FormatQuoteIcon />,
    match: '/admin/quotes',
    children: [
      { to: '/admin/quotes/review', label: '言论审核', icon: <RateReviewIcon /> },
      { to: '/admin/quotes/list', label: '言论列表', icon: <ListAltIcon /> },
    ],
  },
  { to: '/admin/persons', label: '神人管理', icon: <PeopleIcon /> },
  { to: '/admin/api-keys', label: 'API Key', icon: <VpnKeyIcon /> },
  {
    id: 'settings',
    label: '站点设置',
    icon: <SettingsIcon />,
    match: '/admin/settings',
    children: [
      { to: '/admin/settings', label: '基本信息', icon: <InfoOutlinedIcon /> },
      { to: '/admin/settings/captcha', label: '人机验证', icon: <VerifiedUserIcon /> },
    ],
  },
  { to: '/admin/admins', label: '管理员', icon: <AdminPanelSettingsIcon /> },
]

function activeGroupId(pathname: string): string | null {
  const group = navItems.find((item) => isNavGroup(item) && pathname.startsWith(item.match))
  return group && isNavGroup(group) ? group.id : null
}

function pageTitle(pathname: string): string {
  for (const item of navItems) {
    if (isNavGroup(item)) {
      const child = item.children.find((c) => pathname === c.to)
      if (child) return child.label
    } else if (pathname.startsWith(item.to)) {
      return item.label
    }
  }
  return '后台'
}

export default function AdminLayout() {
  const theme = useTheme()
  const isMobile = useMediaQuery(theme.breakpoints.down('md'))
  const [mobileOpen, setMobileOpen] = useState(false)
  const [me, setMe] = useState<AdminMe | null>(null)
  const [loading, setLoading] = useState(true)
  const [unauthorized, setUnauthorized] = useState(false)
  const [openGroupId, setOpenGroupId] = useState<string | null>(null)
  const location = useLocation()
  const navigate = useNavigate()

  useEffect(() => {
    setOpenGroupId(activeGroupId(location.pathname))
  }, [location.pathname])

  const openGroup = (id: string) => {
    setOpenGroupId((current) => (current === id ? null : id))
  }

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
        {navItems.map((item) =>
          isNavGroup(item) ? (
            <Box key={item.id}>
              <ListItemButton
                selected={location.pathname.startsWith(item.match)}
                onClick={() => openGroup(item.id)}
              >
                <ListItemIcon sx={{ minWidth: 40 }}>{item.icon}</ListItemIcon>
                <ListItemText primary={item.label} />
                {openGroupId === item.id ? <ExpandLess /> : <ExpandMore />}
              </ListItemButton>
              <Collapse in={openGroupId === item.id} timeout="auto" unmountOnExit>
                <List disablePadding>
                  {item.children.map((child) => (
                    <ListItemButton
                      key={child.to}
                      component={RouterLink}
                      to={child.to}
                      selected={location.pathname === child.to}
                      onClick={() => {
                        setOpenGroupId(item.id)
                        setMobileOpen(false)
                      }}
                      sx={{ pl: 4 }}
                    >
                      <ListItemIcon sx={{ minWidth: 40 }}>{child.icon}</ListItemIcon>
                      <ListItemText primary={child.label} />
                    </ListItemButton>
                  ))}
                </List>
              </Collapse>
            </Box>
          ) : (
            <ListItemButton
              key={item.to}
              component={RouterLink}
              to={item.to}
              selected={location.pathname.startsWith(item.to)}
              onClick={() => {
                setOpenGroupId(null)
                setMobileOpen(false)
              }}
            >
              <ListItemIcon sx={{ minWidth: 40 }}>{item.icon}</ListItemIcon>
              <ListItemText primary={item.label} />
            </ListItemButton>
          ),
        )}
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
          bgcolor: 'background.paper',
          color: 'text.primary',
          borderBottom: 1,
          borderColor: 'divider',
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
            {pageTitle(location.pathname)}
          </Typography>
          <Typography variant="body2" color="text.secondary" sx={{ mr: 2 }}>
            {me.username}
          </Typography>
          <ThemeModeButton />
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
