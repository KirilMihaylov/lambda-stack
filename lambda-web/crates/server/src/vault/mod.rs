use std::{future::Future, pin::Pin, sync::Arc};

use anyhow::Result as AnyResult;
use sqlx::{query_scalar, PgPool};
use zeroize::Zeroizing;

use lambda_rt::VaultProvider;

#[derive(Debug, Clone)]
pub struct Vault {
    pool: Arc<PgPool>,
}

impl Vault {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool: Arc::new(pool),
        }
    }
}

impl VaultProvider for Vault {
    type Result<'r> =
        Pin<Box<dyn Future<Output = AnyResult<Option<Zeroizing<Vec<u8>>>>> + Send + 'r>>;

    fn fetch_secret(&mut self, identifier: String) -> Self::Result<'_> {
        Box::pin(async {
            query_scalar(include_str!("sql/fetch_secret.sql"))
                .bind(identifier)
                .fetch_optional(&*self.pool)
                .await
                .map(|secret: Option<Vec<u8>>| secret.map(Zeroizing::new))
                .map_err(Into::into)
        })
    }
}
