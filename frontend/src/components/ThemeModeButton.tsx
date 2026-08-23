import { useState, type ReactNode } from 'react'
import { IconButton, ListItemIcon, ListItemText, Menu, MenuItem, Tooltip } from '@mui/material'
import BrightnessAutoIcon from '@mui/icons-material/BrightnessAuto'
import DarkModeIcon from '@mui/icons-material/DarkMode'
import LightModeIcon from '@mui/icons-material/LightMode'
import CheckIcon from '@mui/icons-material/Check'
import { useThemeMode, type ThemePreference } from './ThemeModeProvider'

const choices: Array<{ value: ThemePreference; label: string; icon: ReactNode }> = [
  { value: 'system', label: '跟随系统', icon: <BrightnessAutoIcon fontSize="small" /> },
  { value: 'light', label: '浅色', icon: <LightModeIcon fontSize="small" /> },
  { value: 'dark', label: '深色', icon: <DarkModeIcon fontSize="small" /> },
]

export default function ThemeModeButton() {
  const { preference, resolvedMode, setPreference } = useThemeMode()
  const [anchor, setAnchor] = useState<HTMLElement | null>(null)
  const CurrentIcon = preference === 'system' ? BrightnessAutoIcon : resolvedMode === 'dark' ? DarkModeIcon : LightModeIcon

  return (
    <>
      <Tooltip title="外观模式">
        <IconButton color="inherit" aria-label="外观模式" onClick={(event) => setAnchor(event.currentTarget)}>
          <CurrentIcon fontSize="small" />
        </IconButton>
      </Tooltip>
      <Menu anchorEl={anchor} open={Boolean(anchor)} onClose={() => setAnchor(null)}>
        {choices.map((choice) => (
          <MenuItem
            key={choice.value}
            selected={choice.value === preference}
            onClick={() => {
              setPreference(choice.value)
              setAnchor(null)
            }}
          >
            <ListItemIcon>{choice.icon}</ListItemIcon>
            <ListItemText>{choice.label}</ListItemText>
            {choice.value === preference ? <CheckIcon fontSize="small" /> : null}
          </MenuItem>
        ))}
      </Menu>
    </>
  )
}
