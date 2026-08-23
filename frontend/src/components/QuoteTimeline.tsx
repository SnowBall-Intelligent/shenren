import { useEffect, useRef, useState } from 'react'
import MyLocationIcon from '@mui/icons-material/MyLocation'
import {
  Box,
  Button,
  Popover,
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
  const [previewAnchor, setPreviewAnchor] = useState<HTMLElement | null>(null)
  const [previewIndex, setPreviewIndex] = useState<number | null>(null)

  useEffect(() => {
    const rail = railRef.current
    const marker = markerRefs.current[activeIndex]
    if (!rail || !marker) return
    const markerTop = marker.offsetTop
    const markerBottom = markerTop + marker.offsetHeight
    if (markerTop < rail.scrollTop) rail.scrollTop = markerTop
    else if (markerBottom > rail.scrollTop + rail.clientHeight) {
      rail.scrollTop = markerBottom - rail.clientHeight
    }
  }, [activeIndex])

  useEffect(() => {
    if (!mobile) {
      setPreviewAnchor(null)
      setPreviewIndex(null)
    }
  }, [mobile])

  const handleMarker = (event: React.MouseEvent<HTMLElement>, index: number) => {
    if (mobile) {
      setPreviewAnchor(event.currentTarget)
      setPreviewIndex(index)
    } else {
      onLocate(index)
    }
  }

  const closePreview = () => {
    setPreviewAnchor(null)
    setPreviewIndex(null)
  }

  const locatePreview = () => {
    if (previewIndex != null) onLocate(previewIndex)
    closePreview()
  }

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
          maxHeight: { xs: 'calc(100vh - 96px)', sm: 'calc(100vh - 112px)' },
          overflowY: 'auto',
          overflowX: 'hidden',
          py: 0.75,
          scrollbarWidth: 'none',
          '&::-webkit-scrollbar': { display: 'none' },
        }}
      >
        <Stack spacing={0.25} sx={{ alignItems: 'flex-start' }}>
          {quotes.map((quote, index) => {
            const active = index === activeIndex
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
                data-testid={`timeline-marker-${index + 1}`}
                onClick={(event: React.MouseEvent<HTMLElement>) => handleMarker(event, index)}
                sx={{
                  appearance: 'none',
                  border: 0,
                  bgcolor: 'transparent',
                  width: '100%',
                  height: 9,
                  p: 0,
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'flex-start',
                  cursor: 'pointer',
                  '&::before': {
                    content: '""',
                    display: 'block',
                    height: active ? 3 : 2,
                    width: active ? { xs: 18, sm: 24 } : { xs: 8, sm: 12 },
                    borderRadius: 1,
                    bgcolor: active ? 'primary.main' : 'text.disabled',
                    transition: theme.transitions.create(['width', 'background-color']),
                  },
                  '&:hover::before, &:focus-visible::before': {
                    width: { xs: 18, sm: 24 },
                    bgcolor: active ? 'primary.main' : 'text.secondary',
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

      <Popover
        open={mobile && previewAnchor != null && previewIndex != null}
        anchorEl={previewAnchor}
        onClose={closePreview}
        anchorOrigin={{ vertical: 'center', horizontal: 'right' }}
        transformOrigin={{ vertical: 'center', horizontal: 'left' }}
        slotProps={{
          paper: {
            sx: {
              ml: 1,
              width: 'min(300px, calc(100vw - 56px))',
              maxHeight: 'min(320px, calc(100vh - 32px))',
              p: 1.5,
              borderRadius: 2,
            },
          },
        }}
      >
        {previewIndex != null ? (
          <Stack spacing={1.25} data-testid="timeline-mobile-preview">
            <TimelinePreview quote={quotes[previewIndex]} index={previewIndex} />
            <Button size="small" variant="contained" startIcon={<MyLocationIcon />} onClick={locatePreview}>
              定位
            </Button>
          </Stack>
        ) : null}
      </Popover>
    </>
  )
}
