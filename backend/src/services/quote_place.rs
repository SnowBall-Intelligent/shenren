use std::collections::{HashMap, HashSet};

use chrono::{DateTime, FixedOffset};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, ConnectionTrait, EntityTrait, QueryFilter, Set,
};
use uuid::Uuid;

use crate::entities::{persons, quotes};
use crate::error::{AppError, AppResult};

pub fn new_quote_id() -> String {
    Uuid::new_v4().to_string()
}

pub async fn quote_search_condition<C: ConnectionTrait>(db: &C, q: &str) -> AppResult<Condition> {
    let q = q.trim();
    let mut cond = Condition::any().add(quotes::Column::Content.contains(q));
    let people = persons::Entity::find()
        .filter(persons::Column::Name.contains(q))
        .all(db)
        .await?;
    let ids: Vec<i64> = people.into_iter().map(|p| p.id).collect();
    if !ids.is_empty() {
        cond = cond.add(quotes::Column::PersonId.is_in(ids));
    }
    Ok(cond)
}

async fn load_bucket<C: ConnectionTrait>(db: &C, pinned: bool) -> AppResult<Vec<quotes::Model>> {
    Ok(quotes::Entity::find()
        .filter(quotes::Column::Status.eq(quotes::status::APPROVED))
        .filter(quotes::Column::Pinned.eq(pinned))
        .all(db)
        .await?)
}

pub fn build_chain_order(rows: &[quotes::Model]) -> AppResult<Vec<String>> {
    if rows.is_empty() {
        return Ok(Vec::new());
    }
    let by_id: HashMap<&str, &quotes::Model> = rows.iter().map(|q| (q.id.as_str(), q)).collect();
    let heads: Vec<&quotes::Model> = rows.iter().filter(|q| q.place_after_id.is_none()).collect();
    if heads.is_empty() {
        return Err(AppError::internal("语录链缺少链头"));
    }
    let start = heads[0];
    let mut order = Vec::with_capacity(rows.len());
    let mut seen = HashSet::new();
    let mut current = Some(start);
    while let Some(node) = current {
        if !seen.insert(node.id.as_str()) {
            return Err(AppError::internal("语录链存在环"));
        }
        order.push(node.id.clone());
        let next_id = node.place_before_id.as_deref();
        current = next_id.and_then(|id| by_id.get(id).copied());
    }
    if order.len() != rows.len() {
        return Err(AppError::internal("语录链断裂或存在多个链头"));
    }
    Ok(order)
}

pub async fn chain_order_for_bucket<C: ConnectionTrait>(
    db: &C,
    pinned: bool,
) -> AppResult<Vec<String>> {
    let rows = load_bucket(db, pinned).await?;
    match build_chain_order(&rows) {
        Ok(order) => Ok(order),
        Err(_) => repair_bucket_chain(db, pinned, &rows).await,
    }
}

async fn repair_bucket_chain<C: ConnectionTrait>(
    db: &C,
    pinned: bool,
    rows: &[quotes::Model],
) -> AppResult<Vec<String>> {
    let mut sorted = rows.to_vec();
    sorted.sort_by(|a, b| {
        b.published_at
            .cmp(&a.published_at)
            .then_with(|| b.id.cmp(&a.id))
    });
    let ids: Vec<String> = sorted.iter().map(|q| q.id.clone()).collect();
    apply_chain(db, pinned, &ids).await?;
    Ok(ids)
}

pub async fn full_feed_order<C: ConnectionTrait>(db: &C) -> AppResult<Vec<String>> {
    let mut ids = chain_order_for_bucket(db, true).await?;
    ids.extend(chain_order_for_bucket(db, false).await?);
    Ok(ids)
}

pub async fn ordered_approved_quotes<C: ConnectionTrait>(
    db: &C,
    person_id: Option<i64>,
    q: Option<&str>,
    pinned: Option<bool>,
) -> AppResult<Vec<quotes::Model>> {
    let mut finder = quotes::Entity::find()
        .filter(quotes::Column::Status.eq(quotes::status::APPROVED))
        .filter(quotes::Column::PersonId.is_not_null());
    if let Some(pid) = person_id {
        finder = finder.filter(quotes::Column::PersonId.eq(pid));
    }
    if let Some(pinned) = pinned {
        finder = finder.filter(quotes::Column::Pinned.eq(pinned));
    }
    if let Some(q) = q.filter(|s| !s.is_empty()) {
        finder = finder.filter(quote_search_condition(db, q).await?);
    }
    let rows = finder.all(db).await?;
    let allowed: HashSet<String> = rows.iter().map(|r| r.id.clone()).collect();

    let mut ordered = Vec::new();
    let buckets: Vec<bool> = match pinned {
        Some(p) => vec![p],
        None => vec![true, false],
    };
    for bucket_pinned in buckets {
        let order = chain_order_for_bucket(db, bucket_pinned).await?;
        for id in order {
            if allowed.contains(&id) {
                if let Some(row) = rows.iter().find(|r| r.id == id) {
                    ordered.push(row.clone());
                }
            }
        }
    }
    Ok(ordered)
}

pub async fn apply_chain<C: ConnectionTrait>(
    db: &C,
    pinned: bool,
    ids: &[String],
) -> AppResult<()> {
    for (i, id) in ids.iter().enumerate() {
        let after = if i == 0 {
            None
        } else {
            Some(ids[i - 1].clone())
        };
        let before = if i + 1 < ids.len() {
            Some(ids[i + 1].clone())
        } else {
            None
        };
        let Some(row) = quotes::Entity::find_by_id(id.clone()).one(db).await? else {
            continue;
        };
        if row.pinned != pinned {
            return Err(AppError::bad_request("重排列表与置顶状态不一致"));
        }
        let mut am: quotes::ActiveModel = row.into();
        am.place_after_id = Set(after);
        am.place_before_id = Set(before);
        am.update(db).await?;
    }
    Ok(())
}

pub async fn remove_from_chain<C: ConnectionTrait>(db: &C, quote: &quotes::Model) -> AppResult<()> {
    if quote.status != quotes::status::APPROVED {
        return Ok(());
    }
    let prev = quote.place_after_id.clone();
    let next = quote.place_before_id.clone();
    if let Some(ref pid) = prev {
        if let Some(row) = quotes::Entity::find_by_id(pid.clone()).one(db).await? {
            let mut am: quotes::ActiveModel = row.into();
            am.place_before_id = Set(next.clone());
            am.update(db).await?;
        }
    }
    if let Some(ref nid) = next {
        if let Some(row) = quotes::Entity::find_by_id(nid.clone()).one(db).await? {
            let mut am: quotes::ActiveModel = row.into();
            am.place_after_id = Set(prev.clone());
            am.update(db).await?;
        }
    }
    let mut am: quotes::ActiveModel = quote.clone().into();
    am.place_after_id = Set(None);
    am.place_before_id = Set(None);
    am.update(db).await?;
    Ok(())
}

async fn set_neighbors<C: ConnectionTrait>(
    db: &C,
    id: &str,
    after: Option<String>,
    before: Option<String>,
) -> AppResult<()> {
    let Some(row) = quotes::Entity::find_by_id(id.to_string()).one(db).await? else {
        return Err(AppError::not_found("言论不存在"));
    };
    let mut am: quotes::ActiveModel = row.into();
    am.place_after_id = Set(after);
    am.place_before_id = Set(before);
    am.update(db).await?;
    Ok(())
}

pub async fn insert_by_anchor<C: ConnectionTrait>(
    db: &C,
    quote_id: &str,
    pinned: bool,
    before_id: Option<&str>,
    after_id: Option<&str>,
) -> AppResult<()> {
    if before_id.is_some() && after_id.is_some() {
        return Err(AppError::bad_request("只能指定排在某条前面或后面其中之一"));
    }
    let anchor_id = before_id.or(after_id).unwrap();
    if anchor_id == quote_id {
        return Err(AppError::bad_request("不能相对自己排序"));
    }

    let quote = quotes::Entity::find_by_id(quote_id.to_string())
        .one(db)
        .await?
        .ok_or_else(|| AppError::not_found("言论不存在"))?;

    if quote.status != quotes::status::APPROVED {
        return Err(AppError::bad_request("只能调整已通过的言论"));
    }
    if quote.pinned != pinned {
        return Err(AppError::bad_request("置顶状态与参考言论不一致"));
    }

    remove_from_chain(db, &quote).await?;

    let anchor = quotes::Entity::find_by_id(anchor_id.to_string())
        .one(db)
        .await?
        .ok_or_else(|| AppError::bad_request("参考言论不存在，或与当前置顶状态不一致"))?;
    if anchor.status != quotes::status::APPROVED || anchor.pinned != pinned {
        return Err(AppError::bad_request(
            "参考言论不存在，或与当前置顶状态不一致",
        ));
    }

    if before_id.is_some() {
        let prev = anchor.place_after_id.clone();
        set_neighbors(db, quote_id, prev.clone(), Some(anchor.id.clone())).await?;
        if let Some(ref pid) = prev {
            set_neighbors(
                db,
                pid,
                quotes::Entity::find_by_id(pid.clone())
                    .one(db)
                    .await?
                    .map(|r| r.place_after_id)
                    .flatten(),
                Some(quote_id.to_string()),
            )
            .await?;
        }
        set_neighbors(
            db,
            &anchor.id,
            Some(quote_id.to_string()),
            anchor.place_before_id.clone(),
        )
        .await?;
    } else {
        let next = anchor.place_before_id.clone();
        set_neighbors(db, quote_id, Some(anchor.id.clone()), next.clone()).await?;
        set_neighbors(
            db,
            &anchor.id,
            anchor.place_after_id.clone(),
            Some(quote_id.to_string()),
        )
        .await?;
        if let Some(ref nid) = next {
            set_neighbors(
                db,
                nid,
                Some(quote_id.to_string()),
                quotes::Entity::find_by_id(nid.clone())
                    .one(db)
                    .await?
                    .map(|r| r.place_before_id)
                    .flatten(),
            )
            .await?;
        }
    }
    Ok(())
}

pub async fn insert_by_time<C: ConnectionTrait>(db: &C, quote_id: &str) -> AppResult<()> {
    let quote = quotes::Entity::find_by_id(quote_id.to_string())
        .one(db)
        .await?
        .ok_or_else(|| AppError::not_found("言论不存在"))?;
    if quote.status != quotes::status::APPROVED {
        return Err(AppError::bad_request("只能调整已通过的言论"));
    }

    remove_from_chain(db, &quote).await?;

    let bucket = load_bucket(db, quote.pinned).await?;
    let anchor = bucket
        .iter()
        .filter(|q| q.id != quote.id)
        .filter(|q| {
            q.published_at > quote.published_at
                || (q.published_at == quote.published_at && q.id > quote.id)
        })
        .min_by(|a, b| {
            a.published_at
                .cmp(&b.published_at)
                .then_with(|| a.id.cmp(&b.id))
        });

    if let Some(target) = anchor {
        insert_by_anchor(db, quote_id, quote.pinned, None, Some(&target.id)).await
    } else {
        insert_at_head(db, quote_id, quote.pinned).await
    }
}

async fn insert_at_head<C: ConnectionTrait>(db: &C, quote_id: &str, pinned: bool) -> AppResult<()> {
    let bucket = load_bucket(db, pinned).await?;
    let old_head = bucket
        .iter()
        .find(|q| q.id != quote_id && q.place_after_id.is_none());
    if let Some(head) = old_head {
        set_neighbors(db, quote_id, None, Some(head.id.clone())).await?;
        set_neighbors(
            db,
            &head.id,
            Some(quote_id.to_string()),
            head.place_before_id.clone(),
        )
        .await?;
    } else {
        set_neighbors(db, quote_id, None, None).await?;
    }
    Ok(())
}

pub async fn place_quote<C: ConnectionTrait>(
    db: &C,
    quote_id: &str,
    pinned: bool,
    before_id: Option<String>,
    after_id: Option<String>,
) -> AppResult<()> {
    match (before_id.as_deref(), after_id.as_deref()) {
        (None, None) => insert_by_time(db, quote_id).await,
        (b, a) => insert_by_anchor(db, quote_id, pinned, b, a).await,
    }
}

pub async fn reorder_approved<C: ConnectionTrait>(db: &C, page_ids: &[String]) -> AppResult<()> {
    if page_ids.is_empty() {
        return Ok(());
    }
    let unique: HashSet<&str> = page_ids.iter().map(|s| s.as_str()).collect();
    if unique.len() != page_ids.len() {
        return Err(AppError::bad_request("重排列表不能有重复"));
    }

    let feed_ids = full_feed_order(db).await?;
    for id in page_ids {
        if !feed_ids.iter().any(|fid| fid == id) {
            return Err(AppError::bad_request("只能重排已通过的言论"));
        }
    }

    let slots: Vec<usize> = feed_ids
        .iter()
        .enumerate()
        .filter(|(_, id)| unique.contains(id.as_str()))
        .map(|(i, _)| i)
        .collect();
    if slots.len() != page_ids.len() {
        return Err(AppError::bad_request(
            "重排列表与当前顺序不一致，请刷新后重试",
        ));
    }

    let mut ids = feed_ids;
    for (slot, id) in slots.into_iter().zip(page_ids.iter()) {
        ids[slot] = id.clone();
    }

    let rows = quotes::Entity::find()
        .filter(quotes::Column::Status.eq(quotes::status::APPROVED))
        .all(db)
        .await?;
    let pinned_ids: Vec<String> = ids
        .iter()
        .filter(|id| rows.iter().any(|r| r.id == **id && r.pinned))
        .cloned()
        .collect();
    let normal_ids: Vec<String> = ids
        .iter()
        .filter(|id| rows.iter().any(|r| r.id == **id && !r.pinned))
        .cloned()
        .collect();

    apply_chain(db, true, &pinned_ids).await?;
    apply_chain(db, false, &normal_ids).await?;
    Ok(())
}

pub async fn move_in_chain<C: ConnectionTrait>(db: &C, quote_id: &str, up: bool) -> AppResult<()> {
    let quote = quotes::Entity::find_by_id(quote_id.to_string())
        .one(db)
        .await?
        .ok_or_else(|| AppError::not_found("言论不存在"))?;
    if quote.status != quotes::status::APPROVED {
        return Err(AppError::bad_request("只能调整已通过的言论"));
    }

    let mut order = chain_order_for_bucket(db, quote.pinned).await?;
    let pos = order
        .iter()
        .position(|id| id == quote_id)
        .ok_or_else(|| AppError::bad_request("语录不在当前链中"))?;
    let target = if up {
        if pos == 0 {
            return Err(AppError::bad_request("已经在最前"));
        }
        pos - 1
    } else if pos + 1 >= order.len() {
        return Err(AppError::bad_request("已经在最后"));
    } else {
        pos + 1
    };
    order.swap(pos, target);
    apply_chain(db, quote.pinned, &order).await
}

pub async fn on_pinned_changed<C: ConnectionTrait>(
    db: &C,
    quote_id: &str,
    new_pinned: bool,
    before_id: Option<String>,
    after_id: Option<String>,
) -> AppResult<()> {
    let quote = quotes::Entity::find_by_id(quote_id.to_string())
        .one(db)
        .await?
        .ok_or_else(|| AppError::not_found("言论不存在"))?;
    remove_from_chain(db, &quote).await?;
    if before_id.is_some() || after_id.is_some() {
        let mut am: quotes::ActiveModel = quotes::Entity::find_by_id(quote_id.to_string())
            .one(db)
            .await?
            .ok_or_else(|| AppError::not_found("言论不存在"))?
            .into();
        am.pinned = Set(new_pinned);
        am.update(db).await?;
        insert_by_anchor(
            db,
            quote_id,
            new_pinned,
            before_id.as_deref(),
            after_id.as_deref(),
        )
        .await
    } else {
        let mut am: quotes::ActiveModel = quote.into();
        am.pinned = Set(new_pinned);
        am.update(db).await?;
        insert_by_time(db, quote_id).await
    }
}

pub fn parse_published_at(
    raw: Option<&str>,
    now: DateTime<FixedOffset>,
) -> AppResult<DateTime<FixedOffset>> {
    match raw.map(str::trim).filter(|s| !s.is_empty()) {
        Some(raw) => {
            DateTime::parse_from_rfc3339(raw).map_err(|_| AppError::bad_request("发布时间无效"))
        }
        None => Ok(now),
    }
}
