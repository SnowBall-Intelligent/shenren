import Markdown from 'react-markdown'
import rehypeSanitize, { defaultSchema } from 'rehype-sanitize'
import remarkBreaks from 'remark-breaks'
import remarkGfm from 'remark-gfm'
import { Box } from '@mui/material'

const ALLOWED_TAGS = [
  'p',
  'blockquote',
  'strong',
  'em',
  'del',
  'a',
  'ul',
  'ol',
  'li',
  'code',
  'pre',
  'br',
  'hr',
  'h1',
  'h2',
  'h3',
  'h4',
] as const

const sanitizeSchema = {
  ...defaultSchema,
  tagNames: [...ALLOWED_TAGS],
  protocols: {
    ...defaultSchema.protocols,
    href: ['http', 'https', 'mailto'],
  },
}

export default function QuoteMarkdown({ content }: { content: string }) {
  if (!content.trim()) return null

  return (
    <Box
      sx={{
        color: 'inherit',
        fontSize: 'inherit',
        lineHeight: 1.55,
        wordBreak: 'break-word',
        '& p': { m: 0, '& + p': { mt: 0.75 } },
        '& blockquote': {
          m: 0,
          mb: 1,
          pl: 1.25,
          fontStyle: 'normal',
          color: 'inherit',
          borderLeft: '2px solid',
          borderLeftColor: 'text.secondary',
          '&:last-child': { mb: 0 },
          '& p': { m: 0, '& + p': { mt: 0.25 } },
        },
        '& blockquote blockquote': {
          borderLeftColor: 'divider',
          mt: 0.5,
        },
        '& ul, & ol': { m: 0, pl: 2.25 },
        '& li': { my: 0.1 },
        '& a': { color: 'primary.main', textDecoration: 'underline' },
        '& code': {
          fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Consolas, monospace',
          fontSize: '0.9em',
          bgcolor: 'action.hover',
          px: 0.5,
          borderRadius: '4px',
        },
        '& pre': {
          m: 0,
          my: 0.75,
          p: 1,
          bgcolor: (theme) => theme.palette.mode === 'dark' ? 'rgba(0,0,0,0.35)' : '#f1f3f5',
          borderRadius: 1,
          overflowX: 'auto',
          '& code': { bgcolor: 'transparent', p: 0 },
        },
        '& strong': { fontWeight: 700 },
        '& h1, & h2, & h3, & h4': {
          fontSize: '1em',
          fontWeight: 700,
          m: 0,
          mb: 0.5,
        },
        '& hr': { border: 0, borderTop: '1px solid', borderColor: 'divider', my: 1 },
      }}
    >
      <Markdown
        remarkPlugins={[remarkGfm, remarkBreaks]}
        rehypePlugins={[[rehypeSanitize, sanitizeSchema]]}
        unwrapDisallowed
        allowedElements={[...ALLOWED_TAGS]}
        components={{
          a: ({ href, children }) => (
            <a href={href} target="_blank" rel="noreferrer noopener">
              {children}
            </a>
          ),
        }}
      >
        {content}
      </Markdown>
    </Box>
  )
}
