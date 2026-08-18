use std::path::{Path, PathBuf};

use axum::extract::Multipart;
use uuid::Uuid;

use crate::error::{AppError, AppResult};

const MAX_AVATAR_BYTES: usize = 5 * 1024 * 1024;
const MAX_AVATAR_URL_LEN: usize = 2048;
const ALLOWED_EXT: &[&str] = &["jpg", "jpeg", "png", "gif", "webp"];
const LETTER_PALETTE: [&str; 8] = [
    "#5c6bc0", "#26a69a", "#ef5350", "#ab47bc", "#42a5f5", "#66bb6a", "#ffa726", "#8d6e63",
];

pub type AvatarFile = (Option<String>, Vec<u8>);

pub struct PersonMultipart {
    pub name: String,
    pub avatar: Option<AvatarFile>,
    pub avatar_url: Option<String>,
}

pub struct ApproveMultipart {
    pub person_id: Option<i64>,
    pub create_person_name: Option<String>,
    pub avatar: Option<AvatarFile>,
    pub avatar_url: Option<String>,
}

pub async fn save_avatar_from_multipart_field(
    uploads_dir: &Path,
    field_name: &str,
    field_filename: Option<&str>,
    data: &[u8],
) -> AppResult<String> {
    if data.is_empty() {
        return Err(AppError::bad_request(format!("空的文件字段: {field_name}")));
    }
    if data.len() > MAX_AVATAR_BYTES {
        return Err(AppError::bad_request("头像文件过大（最大 5MB）"));
    }

    let ext = field_filename
        .and_then(|name| Path::new(name).extension())
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .filter(|e| ALLOWED_EXT.contains(&e.as_str()))
        .ok_or_else(|| AppError::bad_request("仅支持 jpg/png/gif/webp 头像"))?;

    std::fs::create_dir_all(uploads_dir)?;
    let filename = format!("{}.{}", Uuid::new_v4(), ext);
    let full_path = uploads_dir.join(&filename);
    tokio::fs::write(&full_path, data).await?;
    Ok(filename)
}

pub fn name_initial(name: &str) -> char {
    name.chars().find(|c| !c.is_whitespace()).unwrap_or('?')
}

pub fn is_letter_avatar(path: &str) -> bool {
    Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.starts_with("letter-") && n.ends_with(".svg"))
        .unwrap_or(false)
}

pub fn parse_avatar_url(raw: &str) -> AppResult<Option<String>> {
    let s = raw.trim();
    if s.is_empty() {
        return Ok(None);
    }
    if s.len() > MAX_AVATAR_URL_LEN {
        return Err(AppError::bad_request("头像 URL 过长"));
    }
    if s.contains("..") {
        return Err(AppError::bad_request("头像 URL 无效"));
    }
    let lower = s.to_ascii_lowercase();
    if !(lower.starts_with("http://") || lower.starts_with("https://")) {
        return Err(AppError::bad_request(
            "头像 URL 须以 http:// 或 https:// 开头",
        ));
    }
    Ok(Some(s.to_string()))
}

/// File upload wins over URL; otherwise generate a first-character SVG.
pub async fn resolve_new_avatar(
    uploads_dir: &Path,
    name: &str,
    avatar: Option<AvatarFile>,
    avatar_url: Option<String>,
) -> AppResult<String> {
    if let Some((filename, data)) = avatar {
        return save_avatar_from_multipart_field(uploads_dir, "avatar", filename.as_deref(), &data)
            .await;
    }
    if let Some(url) = avatar_url {
        return Ok(url);
    }
    generate_letter_avatar(uploads_dir, name).await
}

pub async fn generate_letter_avatar(uploads_dir: &Path, name: &str) -> AppResult<String> {
    std::fs::create_dir_all(uploads_dir)?;
    let ch = name_initial(name);
    let fill = palette_color(name);
    let glyph = xml_escape_char(ch);
    let svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="128" height="128" viewBox="0 0 128 128">
  <rect width="128" height="128" rx="64" fill="{fill}"/>
  <text x="64" y="64" text-anchor="middle" dominant-baseline="central" fill="#ffffff" font-size="58" font-family="system-ui, 'Noto Sans SC', 'PingFang SC', 'Microsoft YaHei', sans-serif">{glyph}</text>
</svg>
"##
    );
    let filename = format!("letter-{}.svg", Uuid::new_v4());
    let full_path = uploads_dir.join(&filename);
    tokio::fs::write(&full_path, svg).await?;
    Ok(filename)
}

fn palette_color(name: &str) -> &'static str {
    let mut h: u32 = 0;
    for b in name.as_bytes() {
        h = h.wrapping_mul(31).wrapping_add(u32::from(*b));
    }
    LETTER_PALETTE[(h as usize) % LETTER_PALETTE.len()]
}

fn xml_escape_char(c: char) -> String {
    match c {
        '&' => "&amp;".into(),
        '<' => "&lt;".into(),
        '>' => "&gt;".into(),
        '"' => "&quot;".into(),
        '\'' => "&apos;".into(),
        _ => c.to_string(),
    }
}

/// Collect all text fields and optional avatar file from multipart.
pub async fn parse_person_multipart(mut multipart: Multipart) -> AppResult<PersonMultipart> {
    let mut name: Option<String> = None;
    let mut avatar: Option<AvatarFile> = None;
    let mut avatar_url: Option<String> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::bad_request(format!("multipart 解析失败: {e}")))?
    {
        let field_name = field.name().unwrap_or("").to_string();
        match field_name.as_str() {
            "name" => {
                let text = field
                    .text()
                    .await
                    .map_err(|e| AppError::bad_request(format!("读取 name 失败: {e}")))?;
                name = Some(text.trim().to_string());
            }
            "avatar_url" => {
                let text = field
                    .text()
                    .await
                    .map_err(|e| AppError::bad_request(format!("读取头像 URL 失败: {e}")))?;
                avatar_url = parse_avatar_url(&text)?;
            }
            "avatar" => {
                let filename = field.file_name().map(|s| s.to_string());
                let data = field
                    .bytes()
                    .await
                    .map_err(|e| AppError::bad_request(format!("读取头像失败: {e}")))?;
                if !data.is_empty() {
                    avatar = Some((filename, data.to_vec()));
                }
            }
            _ => {}
        }
    }

    let name = name
        .filter(|s| !s.is_empty())
        .ok_or_else(|| AppError::bad_request("名称不能为空"))?;
    Ok(PersonMultipart {
        name,
        avatar,
        avatar_url,
    })
}

pub async fn parse_approve_multipart(mut multipart: Multipart) -> AppResult<ApproveMultipart> {
    let mut person_id: Option<i64> = None;
    let mut create_person_name: Option<String> = None;
    let mut avatar: Option<AvatarFile> = None;
    let mut avatar_url: Option<String> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::bad_request(format!("multipart 解析失败: {e}")))?
    {
        let field_name = field.name().unwrap_or("").to_string();
        match field_name.as_str() {
            "person_id" => {
                let text = field
                    .text()
                    .await
                    .map_err(|e| AppError::bad_request(format!("读取 person_id 失败: {e}")))?;
                let text = text.trim();
                if !text.is_empty() {
                    person_id = Some(
                        text.parse()
                            .map_err(|_| AppError::bad_request("person_id 无效"))?,
                    );
                }
            }
            "create_person_name" | "name" => {
                let text = field
                    .text()
                    .await
                    .map_err(|e| AppError::bad_request(format!("读取神人名称失败: {e}")))?;
                let text = text.trim().to_string();
                if !text.is_empty() {
                    create_person_name = Some(text);
                }
            }
            "avatar_url" => {
                let text = field
                    .text()
                    .await
                    .map_err(|e| AppError::bad_request(format!("读取头像 URL 失败: {e}")))?;
                avatar_url = parse_avatar_url(&text)?;
            }
            "avatar" => {
                let filename = field.file_name().map(|s| s.to_string());
                let data = field
                    .bytes()
                    .await
                    .map_err(|e| AppError::bad_request(format!("读取头像失败: {e}")))?;
                if !data.is_empty() {
                    avatar = Some((filename, data.to_vec()));
                }
            }
            _ => {}
        }
    }

    Ok(ApproveMultipart {
        person_id,
        create_person_name,
        avatar,
        avatar_url,
    })
}

pub fn delete_avatar_file(uploads_dir: &Path, avatar_path: &str) {
    if avatar_path.is_empty()
        || avatar_path.starts_with("http://")
        || avatar_path.starts_with("https://")
        || avatar_path.contains("..")
        || Path::new(avatar_path).is_absolute()
    {
        return;
    }
    let full: PathBuf = uploads_dir.join(avatar_path);
    let _ = std::fs::remove_file(full);
}
