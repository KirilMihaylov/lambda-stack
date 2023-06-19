#![forbid(
    rust_2018_compatibility,
    deprecated_in_future,
    unsafe_code,
    clippy::pedantic
)]
#![deny(rust_2021_compatibility, warnings)]

use std::{collections::BTreeMap, fs, sync::Arc};

use actix_web::{
    guard,
    web::{self, Bytes},
    App, HttpServer, Scope,
};
use anyhow::{Context as _, Result as AnyResult};
use clap::Parser;
use ed25519_dalek::VerifyingKey;
use sqlx::{postgres::PgConnectOptions, PgPool};
use wasmtime::{
    Engine as WasmEngine, Linker as WasmLinker, OptLevel as WasmOptLevel, WasmBacktraceDetails,
};

use lambda_auth::middleware::{Auth as AuthMiddleware, AuthenticatedUser};
use lambda_rt::{LinkerWithSdk, SdkContext, SdkUser};

use self::{
    args::Args,
    config::{Bind, Config, RoutePath as ConfigRoutePath},
    service::{modules, workers, RequestSender},
    vault::Vault,
};

mod args;
mod config;
mod service;
mod vault;

#[actix_web::main]
async fn main() -> AnyResult<()> {
    let args: Args = Args::parse();

    let config: Config = fs::read(args.config)
        .context("Failed to read configuration file!")
        .and_then(|content: Vec<u8>| {
            String::from_utf8(content).context("Configuration file uses encoding other than UTF-8!")
        })
        .and_then(|content: String| {
            toml::from_str(&content).context("Failed to parse configuration!")
        })?;

    let database_pool: PgPool = PgPool::connect_with(
        PgConnectOptions::new()
            .host(&config.database.host)
            .port(config.database.port.get())
            .database(&config.database.database)
            .username(&args.db_user)
            .password(&args.db_pass),
    )
    .await?;

    let engine: WasmEngine = new_engine().context("Failed to create WASM engine!")?;

    let modules: modules::Precompiled =
        modules::precompile(&engine, config.modules).context("Failed to precompile modules!")?;

    let linker: Arc<LinkerWithSdk<SdkContext<Vault>>> =
        LinkerWithSdk::new(WasmLinker::new(&engine), Vault::new(database_pool))
            .map(Arc::new)
            .context("Failed to create linker with SDK!")?;

    let make_handler: fn(RequestSender<SdkUser>) -> _ = |sender: RequestSender<SdkUser>| {
        move |user: AuthenticatedUser, body: Bytes| {
            service::request_handler(
                SdkUser::new(String::from(user.username())),
                body,
                sender.clone(),
            )
        }
    };

    let routes_to_handlers: BTreeMap<ConfigRoutePath, RequestSender<SdkUser>> =
        workers::generate_route_handlers(config.global, config.routes, modules, linker)
            .await
            .context("Failed to generate route handlers!")?;

    let verifying_key: VerifyingKey = fs::read(args.verify_key)
        .context("Failed to read verifying key for authentication from file!")
        .and_then(|bytes: Vec<u8>| {
            VerifyingKey::try_from(bytes.as_slice())
                .context("Failed to load verifying key for authentication!")
        })?;

    let server: HttpServer<_, _, _, _> = HttpServer::new(move || {
        App::new()
            .wrap(AuthMiddleware::new(verifying_key))
            .service(routes_to_handlers.iter().fold(
            web::scope("/service").guard(guard::Post()),
            |scope: Scope<_>,
             (route_path, request_sender): (&ConfigRoutePath, &RequestSender<SdkUser>)| {
                scope.route(
                    route_path,
                    web::route().to(make_handler(request_sender.clone())),
                )
            },
        ))
    });

    println!("Preparing to start server...");

    config
        .binds
        .into_iter()
        .try_fold(server, |server: HttpServer<_, _, _, _>, bind: Bind| {
            server.bind((bind.host, bind.port.get()))
        })
        .context("Failed to bind to selected addresses!")?
        .run()
        .await
        .context("Failed to run server!")
}

pub fn new_engine() -> AnyResult<WasmEngine> {
    WasmEngine::new(
        wasmtime::Config::new()
            .async_support(true)
            .cranelift_opt_level(WasmOptLevel::Speed)
            .consume_fuel(false)
            .wasm_backtrace_details(WasmBacktraceDetails::Enable)
            .wasm_multi_value(true)
            .wasm_multi_memory(false)
            .async_support(true)
            .cranelift_nan_canonicalization(true)
            .native_unwind_info(true)
            .parallel_compilation(true)
            .wasm_bulk_memory(true)
            .wasm_threads(false)
            .wasm_simd(true)
            .wasm_memory64(true)
            .wasm_reference_types(true),
    )
}
