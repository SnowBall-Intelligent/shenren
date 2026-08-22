use std::collections::HashSet;

use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder,
    Set,
};

use crate::entities::{persons, quotes};
use crate::error::{AppError, AppResult};

pub async fn quote_search_condition<C: ConnectionTrait>(
    db: &C,
    q: &str,
) -> AppResult<Condition> {
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

pub async fn approved_feed<C: ConnectionTrait>(
    db: &C,
    pinned: bool,
) -> AppResult<Vec<quotes::Model>> {
    Ok(quotes::Entity::find()
        .filter(quotes::Column::Status.eq(quotes::status::APPROVED))
        .filter(quotes::Column::Pinned.eq(pinned))
        .order_by_desc(quotes::Column::SortOrder)
        .order_by_desc(quotes::Column::PublishedAt)
        .order_by_desc(quotes::Column::Id)
        .all(db)
        .await?)
}

pub async fn place_relative<C: ConnectionTrait>(
    db: &C,
    quote_id: i64,
    pinned: bool,
    before_id: Option<i64>,
    after_id: Option<i64>,
) -> AppResult<()> {
    if before_id.is_none() && after_id.is_none() {
        return Ok(());
    }
    if before_id.is_some() && after_id.is_some() {
        return Err(AppError::bad_request("只能指定排在某条前面或后面其中之一"));
    }
    let anchor_id = before_id.or(after_id).unwrap();
    if anchor_id == quote_id {
        return Err(AppError::bad_request("不能相对自己排序"));
    }

    let list = approved_feed(db, pinned).await?;
    if !list.iter().any(|q| q.id == anchor_id) {
        return Err(AppError::bad_request("参考言论不存在，或与当前置顶状态不一致"));
    }

    let mut ids: Vec<i64> = list
        .iter()
        .map(|q| q.id)
        .filter(|id| *id != quote_id)
        .collect();
    let anchor_pos = ids
        .iter()
        .position(|id| *id == anchor_id)
        .ok_or_else(|| AppError::bad_request("参考言论不存在"))?;
    let insert_at = if before_id.is_some() {
        anchor_pos
    } else {
        anchor_pos + 1
    };
    ids.insert(insert_at, quote_id);
    write_sort_orders(db, &ids).await
}

pub async fn reorder_approved<C: ConnectionTrait>(db: &C, page_ids: &[i64]) -> AppResult<()> {
    if page_ids.is_empty() {
        return Ok(());
    }
    let mut wanted = page_ids.to_vec();
    let unique: HashSet<i64> = wanted.iter().copied().collect();
    if unique.len() != wanted.len() {
        return Err(AppError::bad_request("重排列表不能有重复"));
    }

    let feed = quotes::Entity::find()
        .filter(quotes::Column::Status.eq(quotes::status::APPROVED))
        .order_by_desc(quotes::Column::Pinned)
        .order_by_desc(quotes::Column::SortOrder)
        .order_by_desc(quotes::Column::PublishedAt)
        .order_by_desc(quotes::Column::Id)
        .all(db)
        .await?;

    for id in &wanted {
        if !feed.iter().any(|q| q.id == *id) {
            return Err(AppError::bad_request("只能重排已通过的言论"));
        }
    }

    let slots: Vec<usize> = feed
        .iter()
        .enumerate()
        .filter(|(_, q)| unique.contains(&q.id))
        .map(|(i, _)| i)
        .collect();
    if slots.len() != wanted.len() {
        return Err(AppError::bad_request("重排列表与当前顺序不一致，请刷新后重试"));
    }

    let mut ids: Vec<i64> = feed.iter().map(|q| q.id).collect();
    for (slot, id) in slots.into_iter().zip(wanted.drain(..)) {
        ids[slot] = id;
    }
    write_sort_orders(db, &ids).await
}

async fn write_sort_orders<C: ConnectionTrait>(db: &C, ids: &[i64]) -> AppResult<()> {
    let n = ids.len() as i32;
    for (i, id) in ids.iter().enumerate() {
        let Some(row) = quotes::Entity::find_by_id(*id).one(db).await? else {
            continue;
        };
        let mut am: quotes::ActiveModel = row.into();
        am.sort_order = Set(n - i as i32);
        am.update(db).await?;
    }
    Ok(())
}
