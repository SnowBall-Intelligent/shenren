import { lazy, Suspense, useEffect, useState } from 'react'
import { Link as RouterLink, Outlet, useLocation, useNavigate } from 'react-router-dom'
import {
  AppBar,
  Box,
  Button,
  Container,
  Link,
  Toolbar,
  Typography,
  CircularProgress,
} from '@mui/material'
import { publicApi } from '../api'
import type { SiteInfo } from '../api/types'
import { uploadUrl } from '../api/client'

const SubmitDialog = lazy(() => import('../components/SubmitDialog'))

export default function PublicLayout() {
  const location = useLocation()
  const navigate = useNavigate()
  const [site, setSite] = useState<SiteInfo | null>(null)
  const [loading, setLoading] = useState(true)
  const [submitOpen, setSubmitOpen] = useState(false)

  useEffect(() => {
    let cancelled = false
    publicApi
      .getSite()
      .then((s) => {
        if (!cancelled) setSite(s)
      })
      .catch(() => {
        if (!cancelled) {
          setSite({
            site_name: '神人网',
            description: null,
            logo_url: null,
            footer: null,
            allow_propose_person: false,
          })
        }
      })
      .finally(() => {
        if (!cancelled) setLoading(false)
      })
    return () => {
      cancelled = true
    }
  }, [])

  useEffect(() => {
    if (site?.site_name) {
      document.title = site.site_name
    }
  }, [site?.site_name])

  useEffect(() => {
    const state = location.state as { submit?: boolean } | null
    if (location.pathname === '/submit' || state?.submit) {
      setSubmitOpen(true)
      if (location.pathname !== '/' || state?.submit) {
        navigate('/', { replace: true })
      }
    }
  }, [location.pathname, location.state, navigate])

  if (loading) {
    return (
      <Box sx={{ display: 'flex', justifyContent: 'center', py: 8 }}>
        <CircularProgress size={28} />
      </Box>
    )
  }

  const logo = uploadUrl(site?.logo_url)

  return (
    <Box sx={{ minHeight: '100vh', bgcolor: 'background.default', display: 'flex', flexDirection: 'column' }}>
      <AppBar position="sticky" elevation={0} sx={{ bgcolor: '#1a1a1a', borderBottom: '1px solid #2a2a2a' }}>
        <Toolbar sx={{ maxWidth: 720, width: '100%', mx: 'auto', gap: 1 }}>
          <Box
            component={RouterLink}
            to="/"
            sx={{ display: 'flex', alignItems: 'center', gap: 1, textDecoration: 'none', color: 'inherit', flexGrow: 1 }}
          >
            {logo ? (
              <Box component="img" src={logo} alt="" sx={{ width: 32, height: 32, borderRadius: 1, objectFit: 'cover' }} />
            ) : null}
            <Typography variant="h6" sx={{ fontWeight: 700, letterSpacing: 0.5, color: 'inherit' }}>
              {site?.site_name ?? '神人网'}
            </Typography>
          </Box>
          <Button color="inherit" size="small" onClick={() => setSubmitOpen(true)}>
            投稿
          </Button>
        </Toolbar>
      </AppBar>

      <Container maxWidth={false} sx={{ maxWidth: 720, flex: 1, py: 2, px: { xs: 1.5, sm: 2 } }}>
        {site?.description ? (
          <Typography variant="body2" color="text.secondary" sx={{ mb: 2, px: 0.5 }}>
            {site.description}
          </Typography>
        ) : null}
        <Outlet context={{ site }} />
      </Container>

      {submitOpen ? (
        <Suspense fallback={null}>
          <SubmitDialog open={submitOpen} onClose={() => setSubmitOpen(false)} site={site} />
        </Suspense>
      ) : null}

      <Box component="footer" sx={{ py: 2, textAlign: 'center', color: 'text.secondary', fontSize: 13 }}>
        {site?.footer ? (
          <Typography variant="caption" sx={{ display: 'block' }}>
            {site.footer}
          </Typography>
        ) : null}
        <Box sx={{ mt: 0.5 }}>
          <Link component={RouterLink} to="/admin/login" underline="hover" color="inherit" variant="caption">
            管理
          </Link>
        </Box>
      </Box>
    </Box>
  )
}
