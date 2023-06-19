#![forbid(rust_2018_compatibility, deprecated_in_future)]
#![deny(rust_2021_compatibility, warnings)]

use clap::Parser as _;
use sqlx::{
    postgres::PgConnectOptions, Connection as _, Executor as _, PgConnection, Postgres, Transaction,
};
use tokio::runtime::Builder as RuntimeBuilder;

use self::args::Args;

mod args;

fn main() -> anyhow::Result<()> {
    RuntimeBuilder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async move {
            let args: Args = Args::parse();

            let mut database_connection: PgConnection = PgConnection::connect_with(
                &PgConnectOptions::new()
                    .host(&{ args.db_host })
                    .port(args.db_port)
                    .database(&{ args.db_name })
                    .username(&{ args.db_user })
                    .password(&{ args.db_pass }),
            )
            .await?;

            let mut transaction: Transaction<'_, Postgres> = database_connection.begin().await?;

            if let Err(error) = transaction
                .execute(include_str!("../sql/initialize.sql"))
                .await
            {
                transaction.rollback().await?;

                return Err(error.into());
            }

            transaction.commit().await.map_err(Into::into)
        })
}
