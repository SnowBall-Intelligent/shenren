pub use sea_orm_migration::prelude::*;

mod m20240818_000001_init;
mod m20260821_000001_captcha;
mod m20260821_000002_captcha_providers;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20240818_000001_init::Migration),
            Box::new(m20260821_000001_captcha::Migration),
            Box::new(m20260821_000002_captcha_providers::Migration),
        ]
    }
}
