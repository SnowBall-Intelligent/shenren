pub use sea_orm_migration::prelude::*;

mod m20240818_000001_init;
mod m20260821_000001_captcha;
mod m20260821_000002_captcha_providers;
mod m20260821_000003_quote_indexes;
mod m20260822_000001_quote_placement;
mod m20260822_000002_quote_place_anchor;
mod m20260823_000001_quote_uuid_chain;
mod m20260823_000002_api_keys_and_proposed_avatar;
mod m20260823_000003_admin_roles;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20240818_000001_init::Migration),
            Box::new(m20260821_000001_captcha::Migration),
            Box::new(m20260821_000002_captcha_providers::Migration),
            Box::new(m20260821_000003_quote_indexes::Migration),
            Box::new(m20260822_000001_quote_placement::Migration),
            Box::new(m20260822_000002_quote_place_anchor::Migration),
            Box::new(m20260823_000001_quote_uuid_chain::Migration),
            Box::new(m20260823_000002_api_keys_and_proposed_avatar::Migration),
            Box::new(m20260823_000003_admin_roles::Migration),
        ]
    }
}
