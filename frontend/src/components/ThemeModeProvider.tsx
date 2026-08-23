import { createContext, useContext, useEffect, useMemo, useState, type ReactNode } from 'react'
import { CssBaseline, ThemeProvider, useMediaQuery } from '@mui/material'
import { createAppTheme } from '../theme'

export type ThemePreference = 'system' | 'light' | 'dark'

type ThemeModeContextValue = {
  preference: ThemePreference
  resolvedMode: 'light' | 'dark'
  setPreference: (preference: ThemePreference) => void
}

const STORAGE_KEY = 'shenren-theme'
const ThemeModeContext = createContext<ThemeModeContextValue | null>(null)

function storedPreference(): ThemePreference {
  const value = window.localStorage.getItem(STORAGE_KEY)
  return value === 'light' || value === 'dark' || value === 'system' ? value : 'system'
}

export function ThemeModeProvider({ children }: { children: ReactNode }) {
  const systemDark = useMediaQuery('(prefers-color-scheme: dark)')
  const [preference, setPreferenceState] = useState<ThemePreference>(storedPreference)
  const resolvedMode = preference === 'system' ? (systemDark ? 'dark' : 'light') : preference
  const theme = useMemo(() => createAppTheme(resolvedMode), [resolvedMode])

  const setPreference = (next: ThemePreference) => {
    setPreferenceState(next)
    window.localStorage.setItem(STORAGE_KEY, next)
  }

  useEffect(() => {
    document.documentElement.style.colorScheme = resolvedMode
  }, [resolvedMode])

  const value = useMemo(
    () => ({ preference, resolvedMode, setPreference }),
    [preference, resolvedMode],
  )

  return (
    <ThemeModeContext.Provider value={value}>
      <ThemeProvider theme={theme}>
        <CssBaseline />
        {children}
      </ThemeProvider>
    </ThemeModeContext.Provider>
  )
}

export function useThemeMode() {
  const context = useContext(ThemeModeContext)
  if (!context) throw new Error('useThemeMode must be used inside ThemeModeProvider')
  return context
}
