#![forbid(rust_2018_compatibility, deprecated_in_future)]
#![deny(rust_2021_compatibility, warnings)]

use actix_web::{
    dev::{ServiceFactory, ServiceRequest},
    guard,
    web::{self, Data},
    App, HttpServer,
};
use anyhow::{Error as AnyError, Result as AnyResult};
use clap::Parser as _;
use ed25519_dalek::{SecretKey, SigningKey};
use opaque_ke::keypair::KeyPair;
use sqlx::PgPool;
use tokio::fs;
use zeroize::Zeroizing;

use lambda_auth::{middleware::Auth as AuthMiddleware, OpaqueKeyPair};

use self::args::Args;

#[cfg(not(target_has_atomic = "ptr"))]
compile_error!("Only supported on targets with atomic operations over pointer-width integers!");

mod args;
mod login;
mod password_change;
mod registration;

#[actix_web::main]
async fn main() -> AnyResult<()> {
    let args: Args = Args::parse();

    let auth_private_key: Data<Zeroizing<Vec<u8>>> = fs::read(args.auth_key)
        .await
        .map(Zeroizing::new)
        .map(Data::new)?;

    let opaque_auth_keypair: Data<OpaqueKeyPair> =
        KeyPair::from_private_key_slice(&auth_private_key).map(Data::new)?;

    let signing_keypair: Data<SigningKey> = fs::read(args.signing_key)
        .await
        .map(Zeroizing::new)
        .map_err(AnyError::from)
        .and_then(|secret_key: Zeroizing<Vec<u8>>| {
            SecretKey::try_from(secret_key.as_slice())
                .map(Zeroizing::new)
                .map_err(Into::into)
        })
        .map(|ref secret_key: Zeroizing<SecretKey>| SigningKey::from_bytes(secret_key))
        .map(Data::new)?;

    let database_pool: Data<PgPool> = Data::new(
        PgPool::connect_with(
            sqlx::postgres::PgConnectOptions::new()
                .host(&{ args.db_host })
                .port(args.db_port)
                .database(&{ args.db_name })
                .username(&{ args.db_user })
                .password(&{ args.db_pass }),
        )
        .await?,
    );

    let server: HttpServer<_, App<_>, _, _> = HttpServer::new(move || {
        populate_with_services(
            App::new(),
            &auth_private_key,
            &opaque_auth_keypair,
            &signing_keypair,
            &database_pool,
        )
    })
    .bind("127.0.0.1:7777")?;

    println!("Starting server...");

    server.run().await?;

    Ok(())
}

fn populate_with_services<T>(
    app: App<T>,
    auth_private_key: &Data<Zeroizing<Vec<u8>>>,
    opaque_auth_keypair: &Data<OpaqueKeyPair>,
    signing_keypair: &Data<SigningKey>,
    database_pool: &Data<PgPool>,
) -> App<T>
where
    T: ServiceFactory<ServiceRequest, Config = (), Error = actix_web::Error, InitError = ()>,
{
    let auth_middleware: AuthMiddleware = AuthMiddleware::new(signing_keypair.verifying_key());

    app.app_data(auth_private_key.clone())
        .app_data(opaque_auth_keypair.clone())
        .app_data(signing_keypair.clone())
        .app_data(database_pool.clone())
        .service(
            web::scope("/login")
                .guard(guard::Post())
                .service(login::initiate::handle_initiate)
                .service(login::conclude::handle_conclude),
        )
        .service(
            web::scope("/registration")
                .wrap(auth_middleware.clone())
                .guard(guard::Post())
                .service(registration::initiate::handle_initiate)
                .service(registration::conclude::handle_conclude),
        )
        .service(
            web::scope("/password")
                .wrap(auth_middleware.clone())
                .guard(guard::Get())
                .service(password_change::initiate::handle_initiate)
                .service(password_change::conclude::handle_conclude),
        )
}
