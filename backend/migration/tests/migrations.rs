use migration::{Migrator, MigratorTrait};
use sea_orm_migration::prelude::SchemaManager;
use sea_orm_migration::sea_orm::{
    ConnectionTrait, Database, DatabaseBackend, FromQueryResult, Statement,
};

const LEGACY_MIGRATION_COUNT: u32 = 6;
const FIRST_CONTENT: &str = "迁移测试：引号 ' 与反斜杠 \\ 必须原样保留";
const SECOND_CONTENT: &str = "迁移测试：接龙第二条";

#[derive(Debug, FromQueryResult)]
struct MigratedQuote {
    id: String,
    content: String,
    place_before_id: Option<String>,
    place_after_id: Option<String>,
}

#[derive(Debug, FromQueryResult)]
struct MigratedAdmin {
    username: String,
    role: String,
}

#[tokio::test]
async fn migrates_legacy_quotes_on_configured_database() {
    let Ok(database_url) = std::env::var("TEST_DATABASE_URL") else {
        eprintln!("TEST_DATABASE_URL is not set; skipping external migration test");
        return;
    };
    let db = Database::connect(&database_url)
        .await
        .expect("connect migration test database");
    let backend = db.get_database_backend();
    assert!(matches!(
        backend,
        DatabaseBackend::Sqlite | DatabaseBackend::MySql
    ));
    let initial_schema = SchemaManager::new(&db);
    assert!(
        !initial_schema
            .has_table("seaql_migrations")
            .await
            .expect("inspect migration history table"),
        "migration integration tests require a dedicated empty database"
    );

    Migrator::up(&db, Some(LEGACY_MIGRATION_COUNT))
        .await
        .expect("apply legacy migrations");
    let schema = SchemaManager::new(&db);
    assert!(schema
        .has_column("quotes", "sort_order")
        .await
        .expect("inspect legacy quote schema"));

    let now = chrono::Utc::now().fixed_offset();
    db.execute(Statement::from_sql_and_values(
        backend,
        "INSERT INTO admins (username, password_hash, created_at) VALUES (?, ?, ?)",
        vec![
            "legacy-admin".into(),
            "legacy-password-hash".into(),
            now.into(),
        ],
    ))
    .await
    .expect("insert legacy admin");

    for (content, sort_order) in [(FIRST_CONTENT, 20_i32), (SECOND_CONTENT, 10_i32)] {
        db.execute(Statement::from_sql_and_values(
            backend,
            "INSERT INTO quotes (
                content, status, created_at, pinned, sort_order, published_at
             ) VALUES (?, ?, ?, ?, ?, ?)",
            vec![
                content.into(),
                "approved".into(),
                now.into(),
                false.into(),
                sort_order.into(),
                now.into(),
            ],
        ))
        .await
        .expect("insert legacy quote");
    }

    Migrator::up(&db, None)
        .await
        .expect("migrate legacy quotes to UUID schema");
    Migrator::up(&db, None)
        .await
        .expect("migration rerun should be idempotent");

    let schema = SchemaManager::new(&db);
    assert!(!schema
        .has_column("quotes", "sort_order")
        .await
        .expect("inspect migrated quote schema"));
    assert!(schema
        .has_column("quotes", "proposed_person_avatar_url")
        .await
        .expect("inspect proposed avatar column"));
    assert!(schema
        .has_table("api_keys")
        .await
        .expect("inspect API key table"));
    assert!(schema
        .has_column("admins", "role")
        .await
        .expect("inspect admin role column"));

    let admins = MigratedAdmin::find_by_statement(Statement::from_string(
        backend,
        "SELECT username, role FROM admins ORDER BY id".to_string(),
    ))
    .all(&db)
    .await
    .expect("load migrated admins");
    assert_eq!(admins.len(), 1);
    assert_eq!(admins[0].username, "legacy-admin");
    assert_eq!(admins[0].role, "super_admin");

    let rows = MigratedQuote::find_by_statement(Statement::from_string(
        backend,
        "SELECT id, content, place_before_id, place_after_id FROM quotes ORDER BY content"
            .to_string(),
    ))
    .all(&db)
    .await
    .expect("load migrated quotes");
    assert_eq!(rows.len(), 2);
    assert!(rows
        .iter()
        .all(|row| uuid::Uuid::parse_str(&row.id).is_ok()));
    assert!(rows.iter().any(|row| row.content == FIRST_CONTENT));
    assert!(rows.iter().any(|row| row.content == SECOND_CONTENT));

    let head = rows
        .iter()
        .find(|row| row.place_after_id.is_none())
        .expect("chain head");
    let tail = rows
        .iter()
        .find(|row| row.place_before_id.is_none())
        .expect("chain tail");
    assert_ne!(head.id, tail.id);
    assert_eq!(head.place_before_id.as_deref(), Some(tail.id.as_str()));
    assert_eq!(tail.place_after_id.as_deref(), Some(head.id.as_str()));
}
