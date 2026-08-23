use crate::error::{AppError, AppResult};

pub const MAX_QUOTE_CHARS: usize = 2000;
pub const MAX_SOURCE_CHARS: usize = 500;
pub const MAX_PERSON_NAME_CHARS: usize = 128;
pub const MAX_PROPOSED_NAME_CHARS: usize = 64;
pub const MAX_SITE_NAME_CHARS: usize = 128;
pub const MAX_SITE_TEXT_CHARS: usize = 2000;

pub fn normalize_quote_content(raw: &str) -> AppResult<String> {
    let stripped = strip_html(raw.trim());
    let content = sanitize_markdown_urls(&stripped);
    if content.trim().is_empty() {
        return Err(AppError::bad_request("言论内容不能为空"));
    }
    if content.chars().count() > MAX_QUOTE_CHARS {
        return Err(AppError::bad_request("言论内容过长"));
    }
    let lower = content.to_ascii_lowercase();
    if lower.contains("<script") {
        return Err(AppError::bad_request("言论内容包含不允许的标签"));
    }
    Ok(content)
}

pub fn normalize_source(raw: Option<String>) -> AppResult<Option<String>> {
    let Some(s) = raw.map(|s| s.trim().to_string()).filter(|s| !s.is_empty()) else {
        return Ok(None);
    };
    if s.chars().count() > MAX_SOURCE_CHARS {
        return Err(AppError::bad_request("来源过长"));
    }
    Ok(Some(s))
}

pub fn normalize_person_name(raw: &str) -> AppResult<String> {
    let name = raw.trim().to_string();
    if name.is_empty() {
        return Err(AppError::bad_request("名称不能为空"));
    }
    if name.chars().count() > MAX_PERSON_NAME_CHARS {
        return Err(AppError::bad_request("神人名称过长"));
    }
    Ok(name)
}

pub fn normalize_proposed_name(raw: &str) -> AppResult<String> {
    let name = raw.trim().to_string();
    if name.is_empty() {
        return Err(AppError::bad_request("神人名称不能为空"));
    }
    if name.chars().count() > MAX_PROPOSED_NAME_CHARS {
        return Err(AppError::bad_request("神人名称过长"));
    }
    Ok(name)
}

pub fn normalize_site_name(raw: &str) -> AppResult<String> {
    let name = raw.trim().to_string();
    if name.is_empty() {
        return Err(AppError::bad_request("站点名称不能为空"));
    }
    if name.chars().count() > MAX_SITE_NAME_CHARS {
        return Err(AppError::bad_request("站点名称过长"));
    }
    Ok(name)
}

pub fn normalize_site_text(raw: Option<String>) -> AppResult<Option<String>> {
    let Some(s) = raw.map(|s| s.trim().to_string()).filter(|s| !s.is_empty()) else {
        return Ok(None);
    };
    if s.chars().count() > MAX_SITE_TEXT_CHARS {
        return Err(AppError::bad_request("文本过长"));
    }
    Ok(Some(s))
}

fn strip_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' if in_tag => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out
}

fn url_allowed(url: &str) -> bool {
    let u = url.trim();
    let lower = u.to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://") || lower.starts_with("mailto:")
}

/// Rewrite markdown `](url)` whose protocol is not http/https/mailto.
fn sanitize_markdown_urls(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(idx) = rest.find("](") {
        out.push_str(&rest[..idx + 2]);
        rest = &rest[idx + 2..];
        let Some(end) = rest.find(')') else {
            out.push_str(rest);
            return out;
        };
        let url = &rest[..end];
        if url_allowed(url) {
            out.push_str(url);
        } else {
            out.push('#');
        }
        out.push(')');
        rest = &rest[end + 1..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_tags_and_bad_links() {
        let s =
            normalize_quote_content("hello <script>x</script> [a](javascript:alert(1))").unwrap();
        assert!(!s.to_ascii_lowercase().contains("<script"));
        assert!(s.contains("[a](#)"));
    }
}
