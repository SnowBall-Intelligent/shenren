use hmac::{Hmac, Mac};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::Sha256;

use crate::entities::site_settings;
use crate::error::{AppError, AppResult};

#[derive(Clone, Debug, Default, Deserialize)]
pub struct CaptchaPayload {
    pub provider: Option<String>,
    pub token: Option<String>,
    pub lot_number: Option<String>,
    pub captcha_output: Option<String>,
    pub pass_token: Option<String>,
    pub gen_time: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CaptchaProviderConfig {
    pub provider: String,
    pub site_key: String,
    pub secret: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct PublicProvider {
    pub provider: String,
    pub site_key: String,
}

#[derive(Deserialize)]
struct SuccessBody {
    success: Option<bool>,
}

#[derive(Deserialize)]
struct GeetestBody {
    result: Option<String>,
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn http_client() -> AppResult<Client> {
    Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| AppError::internal(format!("http client: {e}")))
}

pub fn is_vendor(raw: &str) -> bool {
    matches!(raw, "turnstile" | "recaptcha" | "geetest")
}

pub fn normalize_vendor(raw: &str) -> AppResult<String> {
    match raw.trim() {
        "turnstile" => Ok("turnstile".to_string()),
        "recaptcha" => Ok("recaptcha".to_string()),
        "geetest" => Ok("geetest".to_string()),
        other => Err(AppError::bad_request(format!(
            "未知的人机验证类型: {other}"
        ))),
    }
}

fn config_complete(item: &CaptchaProviderConfig) -> bool {
    is_vendor(&item.provider) && !item.site_key.trim().is_empty() && !item.secret.trim().is_empty()
}

fn legacy_providers(settings: &site_settings::Model) -> Vec<CaptchaProviderConfig> {
    let provider = settings.captcha_provider.trim();
    if !is_vendor(provider) {
        return Vec::new();
    }
    let site_key = settings
        .captcha_site_key
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let secret = settings
        .captcha_secret
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    match (site_key, secret) {
        (Some(site_key), Some(secret)) => vec![CaptchaProviderConfig {
            provider: provider.to_string(),
            site_key: site_key.to_string(),
            secret: secret.to_string(),
        }],
        _ => Vec::new(),
    }
}

pub fn parse_providers(settings: &site_settings::Model) -> Vec<CaptchaProviderConfig> {
    match settings.captcha_providers.as_deref().map(str::trim) {
        None | Some("") => legacy_providers(settings),
        Some(raw) => match serde_json::from_str::<Vec<CaptchaProviderConfig>>(raw) {
            Ok(items) => items.into_iter().filter(config_complete).collect(),
            Err(_) => legacy_providers(settings),
        },
    }
}

pub fn serialize_providers(items: &[CaptchaProviderConfig]) -> AppResult<String> {
    serde_json::to_string(items).map_err(|e| AppError::internal(format!("captcha json: {e}")))
}

pub fn first_as_legacy(
    items: &[CaptchaProviderConfig],
) -> (String, Option<String>, Option<String>) {
    match items.first() {
        Some(item) => (
            item.provider.clone(),
            Some(item.site_key.clone()),
            Some(item.secret.clone()),
        ),
        None => ("none".to_string(), None, None),
    }
}

pub fn public_provider_list(settings: &site_settings::Model) -> Vec<PublicProvider> {
    parse_providers(settings)
        .into_iter()
        .map(|item| PublicProvider {
            provider: item.provider,
            site_key: item.site_key,
        })
        .collect()
}

pub fn normalize_provider_list(
    items: Vec<(String, Option<String>, Option<String>)>,
) -> AppResult<Vec<CaptchaProviderConfig>> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::with_capacity(items.len());
    for (raw_provider, site_key, secret) in items {
        let provider = normalize_vendor(&raw_provider)?;
        if !seen.insert(provider.clone()) {
            return Err(AppError::bad_request("同一验证厂商不能重复添加"));
        }
        let site_key = site_key
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let secret = secret
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let (Some(site_key), Some(secret)) = (site_key, secret) else {
            return Err(AppError::bad_request("请填写每个厂商的站点密钥和私钥"));
        };
        out.push(CaptchaProviderConfig {
            provider,
            site_key,
            secret,
        });
    }
    if out.len() > 3 {
        return Err(AppError::bad_request("最多配置 3 个人机验证厂商"));
    }
    Ok(out)
}

pub async fn verify_submission_captcha(
    settings: &site_settings::Model,
    payload: Option<&CaptchaPayload>,
    remote_ip: Option<std::net::IpAddr>,
) -> AppResult<()> {
    let chain = parse_providers(settings);
    if chain.is_empty() {
        return Ok(());
    }

    let payload = payload.ok_or_else(|| AppError::bad_request("请先完成人机验证"))?;
    let claimed = payload
        .provider
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());

    let cfg = if let Some(id) = claimed {
        chain
            .iter()
            .find(|item| item.provider == id)
            .cloned()
            .ok_or_else(|| AppError::captcha_failed("人机验证未通过"))?
    } else if chain.len() == 1 {
        chain[0].clone()
    } else {
        return Err(AppError::bad_request("请先完成人机验证"));
    };

    verify_one(&cfg, payload, remote_ip).await
}

async fn verify_one(
    cfg: &CaptchaProviderConfig,
    payload: &CaptchaPayload,
    remote_ip: Option<std::net::IpAddr>,
) -> AppResult<()> {
    match cfg.provider.as_str() {
        "turnstile" => verify_turnstile(&cfg.secret, payload, remote_ip).await,
        "recaptcha" => verify_recaptcha(&cfg.secret, payload, remote_ip).await,
        "geetest" => verify_geetest(&cfg.site_key, &cfg.secret, payload).await,
        other => Err(AppError::bad_request(format!(
            "未知的人机验证类型: {other}"
        ))),
    }
}

async fn verify_turnstile(
    secret: &str,
    payload: &CaptchaPayload,
    remote_ip: Option<std::net::IpAddr>,
) -> AppResult<()> {
    let token = payload
        .token
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| AppError::bad_request("请先完成人机验证"))?;

    let client = http_client()?;
    let ip = remote_ip.map(|ip| ip.to_string());
    let mut form = vec![("secret", secret), ("response", token)];
    if let Some(ref ip) = ip {
        form.push(("remoteip", ip.as_str()));
    }
    let res = client
        .post("https://challenges.cloudflare.com/turnstile/v0/siteverify")
        .form(&form)
        .send()
        .await
        .map_err(|e| {
            tracing::error!("turnstile siteverify failed: {e}");
            AppError::captcha_failed("人机验证失败，请重试")
        })?;
    let body: SuccessBody = res
        .json()
        .await
        .map_err(|_| AppError::captcha_failed("人机验证失败，请重试"))?;
    if body.success == Some(true) {
        Ok(())
    } else {
        Err(AppError::captcha_failed("人机验证未通过"))
    }
}

async fn verify_recaptcha(
    secret: &str,
    payload: &CaptchaPayload,
    remote_ip: Option<std::net::IpAddr>,
) -> AppResult<()> {
    let token = payload
        .token
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| AppError::bad_request("请先完成人机验证"))?;

    let client = http_client()?;
    let ip = remote_ip.map(|ip| ip.to_string());
    let mut form = vec![("secret", secret), ("response", token)];
    if let Some(ref ip) = ip {
        form.push(("remoteip", ip.as_str()));
    }
    let res = client
        .post("https://www.recaptcha.net/recaptcha/api/siteverify")
        .form(&form)
        .send()
        .await
        .map_err(|e| {
            tracing::error!("recaptcha siteverify failed: {e}");
            AppError::captcha_failed("人机验证失败，请重试")
        })?;
    let body: SuccessBody = res
        .json()
        .await
        .map_err(|_| AppError::captcha_failed("人机验证失败，请重试"))?;
    if body.success == Some(true) {
        Ok(())
    } else {
        Err(AppError::captcha_failed("人机验证未通过"))
    }
}

async fn verify_geetest(
    captcha_id: &str,
    captcha_key: &str,
    payload: &CaptchaPayload,
) -> AppResult<()> {
    let lot_number = payload
        .lot_number
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| AppError::bad_request("请先完成人机验证"))?;
    let captcha_output = payload
        .captcha_output
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| AppError::bad_request("请先完成人机验证"))?;
    let pass_token = payload
        .pass_token
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| AppError::bad_request("请先完成人机验证"))?;
    let gen_time = payload
        .gen_time
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| AppError::bad_request("请先完成人机验证"))?;

    let mut mac = Hmac::<Sha256>::new_from_slice(captcha_key.as_bytes())
        .map_err(|_| AppError::internal("invalid geetest key"))?;
    mac.update(lot_number.as_bytes());
    let sign_token = hex_encode(&mac.finalize().into_bytes());

    let client = http_client()?;
    let res = client
        .post("https://gcaptcha4.geetest.com/validate")
        .query(&[("captcha_id", captcha_id)])
        .form(&[
            ("lot_number", lot_number),
            ("captcha_output", captcha_output),
            ("pass_token", pass_token),
            ("gen_time", gen_time),
            ("sign_token", sign_token.as_str()),
        ])
        .send()
        .await
        .map_err(|e| {
            tracing::error!("geetest validate failed: {e}");
            AppError::captcha_failed("人机验证失败，请重试")
        })?;
    if !res.status().is_success() {
        return Err(AppError::captcha_failed("人机验证失败，请重试"));
    }
    let body: GeetestBody = res
        .json()
        .await
        .map_err(|_| AppError::captcha_failed("人机验证失败，请重试"))?;
    if body.result.as_deref() == Some("success") {
        Ok(())
    } else {
        Err(AppError::captcha_failed("人机验证未通过"))
    }
}
