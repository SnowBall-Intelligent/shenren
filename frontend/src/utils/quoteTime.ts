import type { Quote } from '../api/types'

const localDateTimeFormatter = new Intl.DateTimeFormat('zh-CN', {
  year: 'numeric',
  month: '2-digit',
  day: '2-digit',
  hour: '2-digit',
  minute: '2-digit',
  hourCycle: 'h23',
})

export function formatQuotePublishedAt(quote: Quote) {
  const primary = quote.published_at ?? quote.created_at
  const primaryDate = new Date(primary)
  const date = Number.isNaN(primaryDate.getTime()) ? new Date(quote.created_at) : primaryDate
  if (Number.isNaN(date.getTime())) return '时间未知'

  const parts = Object.fromEntries(
    localDateTimeFormatter
      .formatToParts(date)
      .filter((part) => part.type !== 'literal')
      .map((part) => [part.type, part.value]),
  )
  return `${parts.year}-${parts.month}-${parts.day} ${parts.hour}:${parts.minute}`
}

export function quoteSummary(content: string, maxLength = 100) {
  const plain = content.replace(/\s+/g, ' ').trim()
  return plain.length > maxLength ? `${plain.slice(0, maxLength)}...` : plain
}
