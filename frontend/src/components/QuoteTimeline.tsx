import { useEffect, useRef, useState } from 'react'
import MyLocationIcon from '@mui/icons-material/MyLocation'
import CloseIcon from '@mui/icons-material/Close'
import {
  Box,
  Button,
  IconButton,
  Paper,
  Stack,
  Tooltip,
  Typography,
  useMediaQuery,
  useTheme,
} from '@mui/material'
import type { Quote } from '../api/types'
import { formatQuotePublishedAt, quoteSummary } from '../utils/quoteTime'

interface QuoteTimelineProps {
  quotes: Quote[]
  activeIndex: number
  onLocate: (index: number) => void
}

const MARKER_SLOT_HEIGHT = 10
const MAX_VISIBLE_MARKERS = 25

function markerWidth(distance: number, mobile: boolean) {
  const widths = mobile ? [19, 16, 13, 10, 8] : [26, 21, 17, 14, 12]
  return widths[Math.min(distance, widths.length - 1)] ?? widths[widths.length - 1]
}

function personName(quote: Quote) {
  return quote.person?.name ?? quote.proposed_person_name ?? '未知神人'
}

function TimelinePreview({ quote, index }: { quote: Quote; index: number }) {
  return (
    <Stack spacing={0.5} sx={{ minWidth: 0 }}>
      <Typography variant="caption" sx={{ color: 'text.secondary' }}>
        #{index + 1} · {formatQuotePublishedAt(quote)}
      </Typography>
      <Typography variant="body2" sx={{ fontWeight: 600 }}>
        {personName(quote)}
      </Typography>
      <Typography variant="body2" sx={{ color: 'text.secondary', wordBreak: 'break-word' }}>
        {quoteSummary(quote.content)}
      </Typography>
    </Stack>
  )
}

export default function QuoteTimeline({ quotes, activeIndex, onLocate }: QuoteTimelineProps) {
  const theme = useTheme()
  const mobile = useMediaQuery(theme.breakpoints.down('sm'))
  const railRef = useRef<HTMLDivElement | null>(null)
  const markerRefs = useRef<Array<HTMLButtonElement | null>>([])
  const [previewIndex, setPreviewIndex] = useState<number | null>(null)
  const railInteractionRef = useRef(false)
  const interactionTimerRef = useRef<number | null>(null)
  const scrollFrameRef = useRef(0)

  useEffect(() => {
    const rail = railRef.current
    const marker = markerRefs.current[activeIndex]
    if (!rail || !marker) return
    const centered = marker.offsetTop + marker.offsetHeight / 2 - rail.clientHeight / 2
    rail.scrollTop = Math.max(0, centered)
  }, [activeIndex])

  useEffect(() => {
    if (!mobile) {
      setPreviewIndex(null)
    }
  }, [mobile])

  useEffect(
    () => () => {
      if (interactionTimerRef.current != null) window.clearTimeout(interactionTimerRef.current)
      if (scrollFrameRef.current) window.cancelAnimationFrame(scrollFrameRef.current)
    },
    [],
  )

  const markRailInteraction = () => {
    railInteractionRef.current = true
    if (interactionTimerRef.current != null) window.clearTimeout(interactionTimerRef.current)
    interactionTimerRef.current = window.setTimeout(() => {
      railInteractionRef.current = false
    }, 180)
  }

  const selectCenteredMarker = () => {
    const rail = railRef.current
    if (!mobile || !rail || !railInteractionRef.current) return
    const center = rail.scrollTop + rail.clientHeight / 2
    let closestIndex = 0
    let closestDistance = Number.POSITIVE_INFINITY
    markerRefs.current.slice(0, quotes.length).forEach((marker, index) => {
      if (!marker) return
      const distance = Math.abs(marker.offsetTop + marker.offsetHeight / 2 - center)
      if (distance < closestDistance) {
        closestDistance = distance
        closestIndex = index
      }
    })
    setPreviewIndex(closestIndex)
    markRailInteraction()
  }

  const handleRailScroll = () => {
    if (!mobile || !railInteractionRef.current || scrollFrameRef.current) return
    scrollFrameRef.current = window.requestAnimationFrame(() => {
      scrollFrameRef.current = 0
      selectCenteredMarker()
    })
  }

  const handleMarker = (index: number) => {
    if (mobile) {
      setPreviewIndex(index)
    } else {
      onLocate(index)
    }
  }

  const closePreview = () => {
    setPreviewIndex(null)
  }

  const locatePreview = () => {
    if (previewIndex != null) onLocate(previewIndex)
    closePreview()
  }

  const focusIndex = mobile && previewIndex != null ? previewIndex : activeIndex

  return (
    <>
      <Box
        ref={railRef}
        component="nav"
        aria-label="言论时间轴"
        data-testid="quote-timeline"
        sx={{
          position: 'sticky',
          top: { xs: 72, sm: 80 },
          alignSelf: 'start',
          width: '100%',
          maxHeight: `min(${MARKER_SLOT_HEIGHT * MAX_VISIBLE_MARKERS}px, calc(100vh - 96px))`,
          overflowY: 'auto',
          overflowX: 'hidden',
          overscrollBehavior: 'contain',
          touchAction: 'pan-y',
          scrollbarWidth: 'none',
          '&::-webkit-scrollbar': { display: 'none' },
        }}
        onWheel={markRailInteraction}
        onPointerDown={markRailInteraction}
        onKeyDown={markRailInteraction}
        onScroll={handleRailScroll}
      >
        <Stack spacing={0} sx={{ alignItems: 'flex-start' }}>
          {quotes.map((quote, index) => {
            const active = index === activeIndex
            const selected = index === focusIndex
            const distance = Math.abs(index - focusIndex)
            const label = `定位到第 ${index + 1} 条言论：${personName(quote)}，${formatQuotePublishedAt(quote)}`
            const marker = (
              <Box
                key={quote.id}
                ref={(node: HTMLButtonElement | null) => {
                  markerRefs.current[index] = node
                }}
                component="button"
                type="button"
                aria-label={label}
                aria-current={active ? 'step' : undefined}
                data-selected={selected ? 'true' : undefined}
                data-testid={`timeline-marker-${index + 1}`}
                onClick={() => handleMarker(index)}
                sx={{
                  appearance: 'none',
                  border: 0,
                  bgcolor: 'transparent',
                  width: '100%',
                  height: MARKER_SLOT_HEIGHT,
                  p: 0,
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'flex-start',
                  cursor: 'pointer',
                  '&::before': {
                    content: '""',
                    display: 'block',
                    height: selected ? 3 : 2,
                    width: markerWidth(distance, mobile),
                    borderRadius: 1,
                    bgcolor: selected ? 'primary.main' : 'text.disabled',
                    transition: theme.transitions.create(['width', 'background-color']),
                  },
                  '&:hover::before, &:focus-visible::before': {
                    width: { xs: 19, sm: 26 },
                    bgcolor: selected ? 'primary.main' : 'text.secondary',
                  },
                  '&:focus-visible': {
                    outline: '2px solid',
                    outlineColor: 'primary.main',
                    outlineOffset: 1,
                  },
                }}
              />
            )
            return mobile ? (
              marker
            ) : (
              <Tooltip
                key={quote.id}
                placement="right"
                arrow
                title={
                  <Box sx={{ width: 260, maxWidth: 'min(260px, calc(100vw - 64px))', p: 0.5 }}>
                    <TimelinePreview quote={quote} index={index} />
                  </Box>
                }
              >
                {marker}
              </Tooltip>
            )
          })}
        </Stack>
      </Box>

      {mobile && previewIndex != null ? (
        <Paper
          elevation={10}
          data-testid="timeline-mobile-preview"
          role="dialog"
          aria-label={`第 ${previewIndex + 1} 条言论预览`}
          sx={{
            position: 'fixed',
            left: '50%',
            top: '50%',
            transform: 'translate(-50%, -50%)',
            zIndex: theme.zIndex.modal,
            width: 'min(320px, calc(100vw - 64px))',
            maxHeight: 'min(360px, calc(100vh - 96px))',
            overflowY: 'auto',
            p: 1.5,
            borderRadius: 2,
          }}
        >
          <Stack spacing={1.25}>
            <Box sx={{ display: 'flex', alignItems: 'flex-start', gap: 1 }}>
              <Box sx={{ flex: 1, minWidth: 0 }}>
                <TimelinePreview quote={quotes[previewIndex]} index={previewIndex} />
              </Box>
              <IconButton size="small" aria-label="关闭言论预览" onClick={closePreview}>
                <CloseIcon fontSize="small" />
              </IconButton>
            </Box>
            <Button variant="contained" startIcon={<MyLocationIcon />} onClick={locatePreview}>
              定位
            </Button>
          </Stack>
        </Paper>
      ) : null}
    </>
  )
}
