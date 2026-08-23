import { Avatar, Box, Typography } from '@mui/material'
import type { Quote } from '../api/types'
import { nameInitial, uploadUrl } from '../api/client'
import { AVATAR_COL } from '../theme'
import { formatQuotePublishedAt } from '../utils/quoteTime'
import QuoteMarkdown from './QuoteMarkdown'

interface QuoteBubbleProps {
  quote: Quote
  sequence: number
  showIdentity: boolean
  tightGap: boolean
}

export default function QuoteBubble({ quote, sequence, showIdentity, tightGap }: QuoteBubbleProps) {
  const name = quote.person?.name ?? quote.proposed_person_name ?? '未知神人'
  const avatar = uploadUrl(quote.person?.avatar_url)

  return (
    <Box
      sx={{
        display: 'flex',
        gap: 1.25,
        alignItems: 'flex-start',
        mt: tightGap ? 0.5 : 2,
      }}
    >
      <Box sx={{ width: AVATAR_COL, flexShrink: 0, display: 'flex', justifyContent: 'center' }}>
        {showIdentity ? (
          <Avatar
            src={avatar}
            alt={name}
            sx={{ width: 36, height: 36, bgcolor: 'action.selected', fontSize: 14 }}
          >
            {nameInitial(name)}
          </Avatar>
        ) : null}
      </Box>

      <Box sx={{ flex: 1, minWidth: 0 }}>
        {showIdentity ? (
          <Typography
            variant="caption"
            sx={{ color: 'text.secondary', display: 'block', mb: 0.5, pl: 0.25 }}
          >
            {name}
          </Typography>
        ) : null}
        <Box
          sx={{
            display: 'inline-block',
            maxWidth: '100%',
            bgcolor: (theme) => theme.palette.mode === 'dark' ? '#2c2c2c' : '#e9eef5',
            color: 'text.primary',
            borderRadius: '12px',
            px: 1.5,
            py: 1,
            wordBreak: 'break-word',
          }}
        >
          <QuoteMarkdown content={quote.content} />
          {quote.source ? (
            <Typography
              variant="caption"
              sx={{ color: 'text.secondary', display: 'block', mt: 0.75 }}
            >
              来源：{quote.source}
            </Typography>
          ) : null}
        </Box>
        <Typography
          variant="caption"
          data-testid="quote-meta"
          sx={{ color: 'text.secondary', display: 'block', mt: 0.5, pl: 0.25 }}
        >
          #{sequence} · {formatQuotePublishedAt(quote)}
        </Typography>
      </Box>
    </Box>
  )
}
