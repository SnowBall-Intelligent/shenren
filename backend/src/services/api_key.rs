use std::collections::{HashMap, HashSet, VecDeque};
use std::net::IpAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::http::{header, HeaderMap};
use ipnet::IpNet;
use rand_core::{OsRng, RngCore};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

use crate::error::{AppError, AppResult};

const KEY_PREFIX_LEN: usize = 16;
const IDLE_ENTRY_TTL: Duration = Duration::from_secs(60 * 60);

pub fn generate_api_key() -> (String, String, String) {
    let mut bytes = [0_u8; 32];
    OsRng.fill_bytes(&mut bytes);
    let raw = format!("srk_{}", hex(&bytes));
    let prefix = raw[..KEY_PREFIX_LEN].to_string();
    let hash = hash_api_key(&raw);
    (raw, prefix, hash)
}

pub fn api_key_prefix(raw: &str) -> Option<&str> {
    let raw = raw.trim();
    if !raw.starts_with("srk_") || raw.len() < KEY_PREFIX_LEN {
        return None;
    }
    raw.get(..KEY_PREFIX_LEN)
}

pub fn hash_api_key(raw: &str) -> String {
    hex(&Sha256::digest(raw.as_bytes()))
}

pub fn api_key_hash_matches(expected_hex: &str, raw: &str) -> bool {
    let actual = hash_api_key(raw);
    expected_hex.as_bytes().ct_eq(actual.as_bytes()).into()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn parse_string_list(raw: &str) -> AppResult<Vec<String>> {
    serde_json::from_str(raw).map_err(|e| AppError::internal(format!("api key rules json: {e}")))
}

pub fn encode_string_list(items: &[String]) -> AppResult<String> {
    serde_json::to_string(items).map_err(|e| AppError::internal(format!("api key rules json: {e}")))
}

pub fn normalize_ip_rules(items: Vec<String>) -> AppResult<Vec<String>> {
    let mut seen = HashSet::new();
    let mut normalized = Vec::new();
    for raw in items {
        let value = raw.trim();
        if value.is_empty() {
            continue;
        }
        let canonical = if let Ok(ip) = value.parse::<IpAddr>() {
            ip.to_string()
        } else if let Ok(net) = value.parse::<IpNet>() {
            net.trunc().to_string()
        } else {
            return Err(AppError::bad_request(format!("IP 或 CIDR 无效: {value}")));
        };
        if seen.insert(canonical.clone()) {
            normalized.push(canonical);
        }
    }
    Ok(normalized)
}

pub fn ip_allowed(ip: IpAddr, rules: &[String]) -> bool {
    rules.is_empty()
        || rules.iter().any(|rule| {
            rule.parse::<IpAddr>()
                .map(|allowed| allowed == ip)
                .or_else(|_| rule.parse::<IpNet>().map(|net| net.contains(&ip)))
                .unwrap_or(false)
        })
}

pub fn normalize_domain_rules(items: Vec<String>) -> AppResult<Vec<String>> {
    let mut seen = HashSet::new();
    let mut normalized = Vec::new();
    for raw in items {
        let mut value = raw.trim().to_ascii_lowercase();
        if value.is_empty() {
            continue;
        }
        if value.contains("://")
            || value.contains('/')
            || value.contains('?')
            || value.contains('#')
        {
            return Err(AppError::bad_request(format!(
                "来源域名只填写 hostname: {value}"
            )));
        }
        let wildcard = value.starts_with("*.");
        if wildcard {
            value = value[2..].to_string();
        }
        value = value.trim_end_matches('.').to_string();
        if value.is_empty() || value.contains(':') {
            return Err(AppError::bad_request(format!("来源域名无效: {value}")));
        }
        let host = url::Host::parse(&value)
            .map_err(|_| AppError::bad_request(format!("来源域名无效: {value}")))?;
        if wildcard && !matches!(host, url::Host::Domain(_)) {
            return Err(AppError::bad_request("IP 地址不能使用子域通配"));
        }
        let canonical = format!("{}{}", if wildcard { "*." } else { "" }, host);
        if seen.insert(canonical.clone()) {
            normalized.push(canonical);
        }
    }
    Ok(normalized)
}

pub fn source_domain_allowed(headers: &HeaderMap, rules: &[String]) -> bool {
    if rules.is_empty() {
        return true;
    }
    let source = headers
        .get(header::ORIGIN)
        .and_then(|v| v.to_str().ok())
        .and_then(source_host)
        .or_else(|| {
            headers
                .get(header::REFERER)
                .and_then(|v| v.to_str().ok())
                .and_then(source_host)
        });
    let Some(host) = source else {
        return false;
    };
    rules.iter().any(|rule| match rule.strip_prefix("*.") {
        Some(suffix) => host != suffix && host.ends_with(&format!(".{suffix}")),
        None => host == *rule,
    })
}

fn source_host(raw: &str) -> Option<String> {
    let parsed = url::Url::parse(raw.trim()).ok()?;
    Some(
        parsed
            .host_str()?
            .trim_end_matches('.')
            .to_ascii_lowercase(),
    )
}

#[derive(Clone, Copy, Debug)]
pub struct RateSnapshot {
    pub limit: Option<u64>,
    pub remaining: Option<u64>,
    pub reset_after: Option<u64>,
}

#[derive(Debug)]
pub struct RuntimeLimitError {
    pub message: &'static str,
    pub retry_after: Option<u64>,
}

struct RuntimeEntry {
    requests: VecDeque<Instant>,
    active: u64,
    last_seen: Instant,
}

pub struct ApiKeyLimiters {
    entries: Mutex<HashMap<i64, RuntimeEntry>>,
}

impl ApiKeyLimiters {
    pub fn new() -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
        }
    }

    pub fn check_and_acquire(
        self: &Arc<Self>,
        key_id: i64,
        rate_limit: Option<u64>,
        rate_window_secs: Option<u64>,
        concurrency_limit: Option<u64>,
    ) -> Result<(ApiKeyPermit, RateSnapshot), RuntimeLimitError> {
        let now = Instant::now();
        let mut entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        entries.retain(|_, entry| {
            entry.active > 0
                || !entry.requests.is_empty()
                || now.duration_since(entry.last_seen) < IDLE_ENTRY_TTL
        });
        let entry = entries.entry(key_id).or_insert_with(|| RuntimeEntry {
            requests: VecDeque::new(),
            active: 0,
            last_seen: now,
        });
        entry.last_seen = now;

        if let Some(limit) = concurrency_limit {
            if entry.active >= limit {
                return Err(RuntimeLimitError {
                    message: "该 API Key 已达到并发上限",
                    retry_after: Some(1),
                });
            }
        }

        let snapshot = match (rate_limit, rate_window_secs) {
            (Some(limit), Some(window_secs)) => {
                let window = Duration::from_secs(window_secs.max(1));
                while entry
                    .requests
                    .front()
                    .is_some_and(|t| now.duration_since(*t) >= window)
                {
                    entry.requests.pop_front();
                }
                if entry.requests.len() as u64 >= limit {
                    let retry = entry
                        .requests
                        .front()
                        .map(|t| {
                            window
                                .saturating_sub(now.duration_since(*t))
                                .as_secs()
                                .max(1)
                        })
                        .unwrap_or(1);
                    return Err(RuntimeLimitError {
                        message: "该 API Key 请求频率已达上限",
                        retry_after: Some(retry),
                    });
                }
                entry.requests.push_back(now);
                RateSnapshot {
                    limit: Some(limit),
                    remaining: Some(limit.saturating_sub(entry.requests.len() as u64)),
                    reset_after: entry.requests.front().map(|t| {
                        window
                            .saturating_sub(now.duration_since(*t))
                            .as_secs()
                            .max(1)
                    }),
                }
            }
            _ => {
                entry.requests.clear();
                RateSnapshot {
                    limit: None,
                    remaining: None,
                    reset_after: None,
                }
            }
        };

        entry.active = entry.active.saturating_add(1);
        Ok((
            ApiKeyPermit {
                owner: Arc::clone(self),
                key_id,
            },
            snapshot,
        ))
    }

    fn release(&self, key_id: i64) {
        let mut entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(entry) = entries.get_mut(&key_id) {
            entry.active = entry.active.saturating_sub(1);
            entry.last_seen = Instant::now();
        }
    }

    pub fn clear(&self, key_id: i64) {
        let mut entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        if entries.get(&key_id).is_none_or(|entry| entry.active == 0) {
            entries.remove(&key_id);
        }
    }
}

pub struct ApiKeyPermit {
    owner: Arc<ApiKeyLimiters>,
    key_id: i64,
}

impl Drop for ApiKeyPermit {
    fn drop(&mut self) {
        self.owner.release(self.key_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_and_matches_ip_rules() {
        let rules =
            normalize_ip_rules(vec![" 127.0.0.1 ".to_string(), "10.2.3.4/8".to_string()]).unwrap();
        assert_eq!(rules, ["127.0.0.1", "10.0.0.0/8"]);
        assert!(ip_allowed("10.9.8.7".parse().unwrap(), &rules));
        assert!(!ip_allowed("192.0.2.1".parse().unwrap(), &rules));
    }

    #[test]
    fn domain_rules_ignore_origin_scheme_port_and_case() {
        let rules = normalize_domain_rules(vec![
            "Example.COM.".to_string(),
            "*.Api.Example.com".to_string(),
        ])
        .unwrap();
        let mut headers = HeaderMap::new();
        headers.insert(header::ORIGIN, "https://EXAMPLE.com:8443".parse().unwrap());
        assert!(source_domain_allowed(&headers, &rules));

        headers.insert(
            header::ORIGIN,
            "http://v1.api.example.com:3000".parse().unwrap(),
        );
        assert!(source_domain_allowed(&headers, &rules));

        headers.insert(header::ORIGIN, "https://api.example.com".parse().unwrap());
        assert!(!source_domain_allowed(
            &headers,
            &["*.api.example.com".to_string()]
        ));
    }

    #[test]
    fn enforces_concurrency_and_rate_limits() {
        let limiters = Arc::new(ApiKeyLimiters::new());
        let (permit, _) = limiters.check_and_acquire(1, None, None, Some(1)).unwrap();
        let concurrency_error = limiters
            .check_and_acquire(1, None, None, Some(1))
            .err()
            .unwrap();
        assert!(concurrency_error.message.contains("并发"));
        drop(permit);
        assert!(limiters.check_and_acquire(1, None, None, Some(1)).is_ok());

        assert!(limiters
            .check_and_acquire(2, Some(1), Some(60), None)
            .is_ok());
        let rate_error = limiters
            .check_and_acquire(2, Some(1), Some(60), None)
            .err()
            .unwrap();
        assert!(rate_error.message.contains("频率"));
    }
}
