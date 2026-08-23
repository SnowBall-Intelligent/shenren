use sea_orm::{ConnectionTrait, DatabaseBackend, FromQueryResult, Statement};
use sea_orm_migration::prelude::*;
use uuid::Uuid;

#[derive(FromQueryResult)]
struct OldQuoteRow {
    id: i64,
    person_id: Option<i64>,
    proposed_person_name: Option<String>,
    content: String,
    source: Option<String>,
    status: String,
    pinned: bool,
    sort_order: i32,
    published_at: chrono::DateTime<chrono::FixedOffset>,
    place_before_id: Option<i64>,
    place_after_id: Option<i64>,
    created_at: chrono::DateTime<chrono::FixedOffset>,
    reviewed_at: Option<chrono::DateTime<chrono::FixedOffset>>,
    reviewed_by: Option<i64>,
}

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if !manager.has_column("quotes", "sort_order").await? {
            return Ok(());
        }

        let db = manager.get_connection();
        let backend = manager.get_database_backend();

        let rows: Vec<OldQuoteRow> = OldQuoteRow::find_by_statement(Statement::from_string(
            backend,
            "SELECT id, person_id, proposed_person_name, content, source, status, pinned, \
             sort_order, published_at, place_before_id, place_after_id, created_at, \
             reviewed_at, reviewed_by FROM quotes"
                .to_string(),
        ))
        .all(db)
        .await?;

        let mut id_map: std::collections::HashMap<i64, String> = std::collections::HashMap::new();
        for row in &rows {
            id_map.insert(row.id, Uuid::new_v4().to_string());
        }

        let mut chain_after: std::collections::HashMap<String, Option<String>> =
            std::collections::HashMap::new();
        let mut chain_before: std::collections::HashMap<String, Option<String>> =
            std::collections::HashMap::new();

        for pinned in [false, true] {
            let mut bucket: Vec<&OldQuoteRow> = rows
                .iter()
                .filter(|r| r.status == "approved" && r.pinned == pinned)
                .collect();
            bucket.sort_by(|a, b| {
                b.sort_order
                    .cmp(&a.sort_order)
                    .then_with(|| b.published_at.cmp(&a.published_at))
                    .then_with(|| b.id.cmp(&a.id))
            });
            for (i, row) in bucket.iter().enumerate() {
                let uid = id_map[&row.id].clone();
                let after = if i == 0 {
                    None
                } else {
                    Some(id_map[&bucket[i - 1].id].clone())
                };
                let before = if i + 1 < bucket.len() {
                    Some(id_map[&bucket[i + 1].id].clone())
                } else {
                    None
                };
                chain_after.insert(uid.clone(), after);
                chain_before.insert(uid, before);
            }
        }

        let timestamp_sql = match backend {
            DatabaseBackend::Sqlite => "TEXT",
            DatabaseBackend::MySql => "TIMESTAMP",
            _ => {
                return Err(DbErr::Custom(
                    "quote uuid migration supports sqlite and mysql only".into(),
                ))
            }
        };
        let uuid_sql = match backend {
            DatabaseBackend::MySql => "VARCHAR(36)",
            DatabaseBackend::Sqlite => "TEXT",
            _ => unreachable!(),
        };

        db.execute_unprepared("PRAGMA foreign_keys = OFF")
            .await
            .ok();
        db.execute_unprepared("DROP TABLE IF EXISTS quotes_new")
            .await?;
        db.execute_unprepared("DROP TABLE IF EXISTS quotes_legacy")
            .await?;

        db.execute_unprepared(&format!(
            "CREATE TABLE quotes_new (
                id {uuid_sql} PRIMARY KEY NOT NULL,
                person_id BIGINT NULL,
                proposed_person_name VARCHAR(128) NULL,
                content TEXT NOT NULL,
                source TEXT NULL,
                status VARCHAR(32) NOT NULL,
                pinned BOOLEAN NOT NULL DEFAULT 0,
                published_at {timestamp_sql} NOT NULL,
                place_before_id {uuid_sql} NULL,
                place_after_id {uuid_sql} NULL,
                created_at {timestamp_sql} NOT NULL,
                reviewed_at {timestamp_sql} NULL,
                reviewed_by BIGINT NULL,
                FOREIGN KEY (person_id) REFERENCES persons(id) ON DELETE SET NULL ON UPDATE CASCADE,
                FOREIGN KEY (reviewed_by) REFERENCES admins(id) ON DELETE SET NULL ON UPDATE CASCADE
            )",
        ))
        .await?;

        for row in &rows {
            let new_id = id_map[&row.id].clone();
            let (place_before, place_after) = if row.status == "approved" {
                (
                    chain_before.get(&new_id).cloned().flatten(),
                    chain_after.get(&new_id).cloned().flatten(),
                )
            } else {
                let intent_before = row
                    .place_before_id
                    .and_then(|old| id_map.get(&old).cloned());
                let intent_after = row.place_after_id.and_then(|old| id_map.get(&old).cloned());
                (intent_before, intent_after)
            };

            db.execute(Statement::from_sql_and_values(
                backend,
                "INSERT INTO quotes_new (
                    id, person_id, proposed_person_name, content, source, status, pinned,
                    published_at, place_before_id, place_after_id, created_at, reviewed_at,
                    reviewed_by
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                vec![
                    new_id.into(),
                    row.person_id.into(),
                    row.proposed_person_name.clone().into(),
                    row.content.clone().into(),
                    row.source.clone().into(),
                    row.status.clone().into(),
                    row.pinned.into(),
                    row.published_at.into(),
                    place_before.into(),
                    place_after.into(),
                    row.created_at.into(),
                    row.reviewed_at.into(),
                    row.reviewed_by.into(),
                ],
            ))
            .await?;
        }

        match backend {
            DatabaseBackend::MySql => {
                db.execute_unprepared("RENAME TABLE quotes TO quotes_legacy, quotes_new TO quotes")
                    .await?;
            }
            DatabaseBackend::Sqlite => {
                db.execute_unprepared("ALTER TABLE quotes RENAME TO quotes_legacy")
                    .await?;
                // SQLite keeps the old table name visible until commit; finalize the swap explicitly.
                db.execute_unprepared("COMMIT").await.ok();
                db.execute_unprepared("ALTER TABLE quotes_new RENAME TO quotes")
                    .await?;
            }
            _ => unreachable!(),
        }
        db.execute_unprepared("DROP TABLE quotes_legacy").await?;

        create_index_if_missing(
            manager,
            "idx_quotes_status_created_at",
            &[Quotes::Status, Quotes::CreatedAt],
        )
        .await?;
        create_index_if_missing(
            manager,
            "idx_quotes_status_person_created_at",
            &[Quotes::Status, Quotes::PersonId, Quotes::CreatedAt],
        )
        .await?;
        create_index_if_missing(
            manager,
            "idx_quotes_status_pinned",
            &[Quotes::Status, Quotes::Pinned],
        )
        .await?;

        db.execute_unprepared("PRAGMA foreign_keys = ON").await.ok();

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "quote uuid chain migration cannot be reversed automatically".into(),
        ))
    }
}

async fn create_index_if_missing(
    manager: &SchemaManager<'_>,
    name: &str,
    columns: &[Quotes],
) -> Result<(), DbErr> {
    if manager.has_index("quotes", name).await? {
        return Ok(());
    }
    let mut index = Index::create();
    index.name(name).table(Quotes::Table);
    for column in columns {
        index.col(*column);
    }
    manager.create_index(index.to_owned()).await
}

#[derive(Copy, Clone, DeriveIden)]
enum Quotes {
    Table,
    Status,
    PersonId,
    Pinned,
    CreatedAt,
}
