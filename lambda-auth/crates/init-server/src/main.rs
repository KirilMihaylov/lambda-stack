#![forbid(rust_2018_compatibility, deprecated_in_future)]
#![deny(rust_2021_compatibility, warnings)]

use clap::Parser as _;
use opaque_ke::{
    ClientRegistration, ClientRegistrationFinishParameters, ClientRegistrationFinishResult,
    ClientRegistrationStartResult, Identifiers, RegistrationResponse, ServerRegistration,
    ServerSetup,
};
use rand_core::OsRng;
use sqlx::{
    pool::PoolConnection, query, Connection as _, Executor as _, PgPool, Postgres, Transaction,
};
use tokio::{
    fs,
    io::{
        stdin, stdout, AsyncBufReadExt as _, AsyncWriteExt as _, BufReader, Result as IoResult,
        Stdin, Stdout,
    },
};
use zeroize::Zeroizing;

use lambda_auth::{AuthCipherSuite, OpaqueKeyPair};

use self::args::Args;

mod args;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Args = Args::parse();

    let opaque_server_setup: ServerSetup<AuthCipherSuite> = ServerSetup::new_with_key(
        &mut OsRng,
        OpaqueKeyPair::from_private_key_slice(
            Zeroizing::new(fs::read(args.auth_key).await?).as_ref(),
        )?,
    );

    let database_pool: PgPool = PgPool::connect_with(
        sqlx::postgres::PgConnectOptions::new()
            .host(&{ args.db_host })
            .port(args.db_port)
            .database(&{ args.db_name })
            .username(&{ args.db_user })
            .password(&{ args.db_pass }),
    )
    .await?;

    let (username, password_file): (String, Vec<u8>) =
        user_registration(&opaque_server_setup).await?;

    initialize_database(
        &database_pool,
        username,
        postcard::to_allocvec(&opaque_server_setup)?,
        password_file,
    )
    .await?;

    Ok(())
}

async fn initialize_database(
    database_pool: &PgPool,
    username: String,
    opaque_setup: Vec<u8>,
    password_file: Vec<u8>,
) -> anyhow::Result<()> {
    let mut connection: PoolConnection<Postgres> = database_pool.acquire().await?;
    let mut transaction: Transaction<'_, Postgres> = connection.begin().await?;
    let mut nested_transaction: Transaction<'_, Postgres> = transaction.begin().await?;

    if let Err(error) = nested_transaction
        .execute(include_str!("sql/initialize.sql"))
        .await
    {
        nested_transaction.rollback().await?;

        return Err(error.into());
    }

    nested_transaction.commit().await?;

    if let Err(error) = transaction
        .execute(
            query(include_str!("../../../sql/upload_password_file.sql"))
                .bind(username)
                .bind(opaque_setup)
                .bind(password_file),
        )
        .await
    {
        transaction.rollback().await?;

        return Err(error.into());
    }

    transaction.commit().await.map_err(Into::into)
}

async fn user_registration(
    opaque_server_setup: &ServerSetup<AuthCipherSuite>,
) -> anyhow::Result<(String, Vec<u8>)> {
    let username: String;

    let password_file: Vec<u8> = {
        let password: Zeroizing<String>;

        {
            let mut stdout: Stdout = stdout();
            let mut stdin: BufReader<Stdin> = BufReader::new(stdin());

            username = enter_username(&mut stdout, &mut stdin).await?;

            password = enter_password(&mut stdout, &mut stdin).await?;
        }

        let ClientRegistrationStartResult { message, state }: ClientRegistrationStartResult<
            AuthCipherSuite,
        > = ClientRegistration::start(&mut OsRng, password.as_bytes())?;

        let registration_response: RegistrationResponse<AuthCipherSuite> =
            ServerRegistration::start(&opaque_server_setup, message, username.as_bytes())?.message;

        let ClientRegistrationFinishResult { message, .. }: ClientRegistrationFinishResult<
            AuthCipherSuite,
        > = state.finish(
            &mut OsRng,
            password.as_bytes(),
            registration_response,
            ClientRegistrationFinishParameters::new(Identifiers::default(), None),
        )?;

        postcard::to_allocvec(&ServerRegistration::finish(message))?
    };

    Ok((username, password_file))
}

async fn enter_username(stdout: &mut Stdout, stdin: &mut BufReader<Stdin>) -> IoResult<String> {
    loop {
        stdout
            .write_all(b"Enter administrator's username: ")
            .await?;
        stdout.flush().await?;

        let mut username: String = String::new();
        stdin.read_line(&mut username).await?;

        username = String::from(username.trim());

        if username.is_empty() {
            stdout.write_all(b"Username must not be empty!\n").await?;

            continue;
        }

        break Ok(username);
    }
}

async fn enter_password(
    stdout: &mut Stdout,
    stdin: &mut BufReader<Stdin>,
) -> IoResult<Zeroizing<String>> {
    loop {
        stdout
            .write_all(b"Enter administrator's password: ")
            .await?;
        stdout.flush().await?;

        let mut password: Zeroizing<String> = Zeroizing::new(String::new());
        stdin.read_line(&mut password).await?;

        password = Zeroizing::new(String::from(password.trim()));

        if password.is_empty() {
            stdout.write_all(b"Password must not be empty!\n").await?;

            continue;
        }

        break Ok(password);
    }
}
