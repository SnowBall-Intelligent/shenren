use std::collections::{HashMap, VecDeque};
use std::sync::RwLock;

use bytes::Bytes;
use sha2::{Digest, Sha256};

const MAX_QUOTE_ENTRIES: usize = 64;
const MAX_PERSON_ENTRIES: usize = 32;
const MAX_TOTAL_BYTES: usize = 16 * 1024 * 1024;
const MAX_ENTRY_BYTES: usize = 512 * 1024;

#[derive(Clone)]
pub struct CachedBody {
    pub bytes: Bytes,
    pub etag: String,
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct QuotesKey {
    page: u64,
    page_size: u64,
    person_id: Option<i64>,
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct PersonsKey {
    q: String,
    limit: u64,
}

struct Inner {
    site: Option<CachedBody>,
    quotes: HashMap<QuotesKey, CachedBody>,
    quotes_lru: VecDeque<QuotesKey>,
    persons: HashMap<PersonsKey, CachedBody>,
    persons_lru: VecDeque<PersonsKey>,
    total_bytes: usize,
}

pub struct PublicReadCache {
    inner: RwLock<Inner>,
}

impl PublicReadCache {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(Inner {
                site: None,
                quotes: HashMap::new(),
                quotes_lru: VecDeque::new(),
                persons: HashMap::new(),
                persons_lru: VecDeque::new(),
                total_bytes: 0,
            }),
        }
    }

    pub fn get_site(&self) -> Option<CachedBody> {
        self.inner.read().ok()?.site.clone()
    }

    pub fn put_site(&self, body: Vec<u8>) -> Option<CachedBody> {
        let cached = make_cached(body)?;
        if let Ok(mut g) = self.inner.write() {
            if let Some(old) = g.site.take() {
                g.total_bytes = g.total_bytes.saturating_sub(old.bytes.len());
            }
            g.total_bytes = g.total_bytes.saturating_add(cached.bytes.len());
            g.site = Some(cached.clone());
        }
        Some(cached)
    }

    pub fn quotes_cacheable(page: u64, page_size: u64) -> bool {
        page_size <= 20 && page >= 1 && page <= 10
    }

    pub fn get_quotes(
        &self,
        page: u64,
        page_size: u64,
        person_id: Option<i64>,
    ) -> Option<CachedBody> {
        if !Self::quotes_cacheable(page, page_size) {
            return None;
        }
        let key = QuotesKey {
            page,
            page_size,
            person_id,
        };
        let mut g = self.inner.write().ok()?;
        let inner = &mut *g;
        let body = inner.quotes.get(&key)?.clone();
        touch_lru(&mut inner.quotes_lru, &key);
        Some(body)
    }

    pub fn put_quotes(
        &self,
        page: u64,
        page_size: u64,
        person_id: Option<i64>,
        body: Vec<u8>,
    ) -> Option<CachedBody> {
        if !Self::quotes_cacheable(page, page_size) {
            return make_cached(body);
        }
        let cached = make_cached(body)?;
        let key = QuotesKey {
            page,
            page_size,
            person_id,
        };
        if let Ok(mut g) = self.inner.write() {
            let inner = &mut *g;
            insert_bounded(
                &mut inner.quotes,
                &mut inner.quotes_lru,
                &mut inner.total_bytes,
                key,
                cached.clone(),
                MAX_QUOTE_ENTRIES,
            );
        }
        Some(cached)
    }

    pub fn get_persons(&self, q: &str, limit: u64) -> Option<CachedBody> {
        let key = PersonsKey {
            q: q.to_string(),
            limit,
        };
        let mut g = self.inner.write().ok()?;
        let inner = &mut *g;
        let body = inner.persons.get(&key)?.clone();
        touch_lru(&mut inner.persons_lru, &key);
        Some(body)
    }

    pub fn put_persons(&self, q: &str, limit: u64, body: Vec<u8>) -> Option<CachedBody> {
        let cached = make_cached(body)?;
        let key = PersonsKey {
            q: q.to_string(),
            limit,
        };
        if let Ok(mut g) = self.inner.write() {
            let inner = &mut *g;
            insert_bounded(
                &mut inner.persons,
                &mut inner.persons_lru,
                &mut inner.total_bytes,
                key,
                cached.clone(),
                MAX_PERSON_ENTRIES,
            );
        }
        Some(cached)
    }

    pub fn invalidate_site(&self) {
        if let Ok(mut g) = self.inner.write() {
            if let Some(old) = g.site.take() {
                g.total_bytes = g.total_bytes.saturating_sub(old.bytes.len());
            }
        }
    }

    pub fn invalidate_quotes(&self) {
        if let Ok(mut g) = self.inner.write() {
            let n: usize = g.quotes.values().map(|c| c.bytes.len()).sum();
            g.quotes.clear();
            g.quotes_lru.clear();
            g.total_bytes = g.total_bytes.saturating_sub(n);
        }
    }

    pub fn invalidate_persons(&self) {
        if let Ok(mut g) = self.inner.write() {
            let n: usize = g.persons.values().map(|c| c.bytes.len()).sum();
            g.persons.clear();
            g.persons_lru.clear();
            g.total_bytes = g.total_bytes.saturating_sub(n);
        }
    }

    pub fn bust_public(&self) {
        self.invalidate_site();
        self.invalidate_quotes();
        self.invalidate_persons();
    }
}

pub fn cached_or_raw(body: Vec<u8>) -> CachedBody {
    from_bytes(body.clone()).unwrap_or_else(|| CachedBody {
        etag: etag_for(&body),
        bytes: Bytes::from(body),
    })
}

pub fn from_bytes(body: Vec<u8>) -> Option<CachedBody> {
    if body.is_empty() || body.len() > MAX_ENTRY_BYTES {
        return None;
    }
    Some(CachedBody {
        etag: etag_for(&body),
        bytes: Bytes::from(body),
    })
}

fn make_cached(body: Vec<u8>) -> Option<CachedBody> {
    from_bytes(body)
}

fn etag_for(body: &[u8]) -> String {
    let digest = Sha256::digest(body);
    format!("\"{}\"", hex_16(&digest))
}

fn hex_16(bytes: &[u8]) -> String {
    bytes[..16]
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn touch_lru<K: Clone + Eq>(lru: &mut VecDeque<K>, key: &K) {
    lru.retain(|k| k != key);
    lru.push_back(key.clone());
}

fn insert_bounded<K: Clone + Eq + std::hash::Hash>(
    map: &mut HashMap<K, CachedBody>,
    lru: &mut VecDeque<K>,
    total_bytes: &mut usize,
    key: K,
    cached: CachedBody,
    max_entries: usize,
) {
    if let Some(old) = map.remove(&key) {
        *total_bytes = total_bytes.saturating_sub(old.bytes.len());
        lru.retain(|k| k != &key);
    }
    while map.len() >= max_entries || *total_bytes + cached.bytes.len() > MAX_TOTAL_BYTES {
        let Some(evict) = lru.pop_front() else {
            break;
        };
        if let Some(old) = map.remove(&evict) {
            *total_bytes = total_bytes.saturating_sub(old.bytes.len());
        }
    }
    *total_bytes = total_bytes.saturating_add(cached.bytes.len());
    map.insert(key.clone(), cached);
    lru.push_back(key);
}
