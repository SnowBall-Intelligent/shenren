import { createTheme, type PaletteMode } from '@mui/material/styles'

export function createAppTheme(mode: PaletteMode) {
  const dark = mode === 'dark'
  return createTheme({
    palette: {
      mode,
      primary: {
        main: dark ? '#90caf9' : '#1976d2',
      },
      secondary: {
        main: dark ? '#ce93d8' : '#8e24aa',
      },
      background: {
        default: dark ? '#121212' : '#f6f7f9',
        paper: dark ? '#1e1e1e' : '#ffffff',
      },
      text: {
        primary: dark ? '#e0e0e0' : '#202124',
        secondary: dark ? '#9e9e9e' : '#667085',
      },
      divider: dark ? '#2a2a2a' : '#dfe3e8',
    },
    shape: {
      borderRadius: 12,
    },
    typography: {
      fontFamily:
        '"Noto Sans SC", "PingFang SC", "Microsoft YaHei", "Segoe UI", Roboto, Helvetica, Arial, sans-serif',
    },
    components: {
      MuiCssBaseline: {
        styleOverrides: {
          body: {
            backgroundColor: dark ? '#121212' : '#f6f7f9',
            minHeight: '100vh',
          },
          a: {
            color: 'inherit',
          },
        },
      },
      MuiButton: {
        styleOverrides: {
          root: {
            textTransform: 'none',
          },
        },
      },
    },
  })
}

export const AVATAR_COL = 44
