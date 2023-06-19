#![forbid(rust_2018_compatibility, deprecated_in_future)]
#![deny(rust_2021_compatibility, warnings)]

use aes_gcm::{AeadInPlace as _, Aes256Gcm, Key, KeyInit};
use anyhow::Context;
use clap::Parser as _;
use hkdf::Hkdf;
use opaque_ke::{
    rand::rngs::OsRng, ClientLogin, ClientLoginFinishParameters, ClientLoginFinishResult,
    ClientLoginStartResult, Identifiers,
};
use reqwest::{Client, Method, Request};
use sha2::Sha512_256;
use zeroize::Zeroizing;

use lambda_auth::{login, AuthCipherSuite, EncryptedToken};

use self::args::Args;

mod args;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Args = Args::parse();

    let client: Client = Client::builder()
        .brotli(true)
        .deflate(true)
        .gzip(true)
        .https_only(false)
        .use_rustls_tls()
        .build()?;

    let ClientLoginStartResult {
        message: request,
        state,
    }: ClientLoginStartResult<AuthCipherSuite> =
        ClientLogin::start(&mut OsRng, args.pass.as_bytes())?;

    let request: login::InitiationRequest = login::InitiationRequest {
        username: args.user.clone(),
        request,
    };

    println!("Preparing initiation request...");

    let request: Request = client
        .request(Method::POST, &format!("{}login/initiate", args.auth_uri))
        .body(postcard::to_allocvec(&request)?)
        .build()?;

    println!("Sending initiation request...");

    let login::InitiationResponse {
        state: server_state,
        response,
    }: login::InitiationResponse = postcard::from_bytes(&Vec::from(
        client
            .execute(request)
            .await
            .context("Failed to execute initiation request!")?
            .bytes()
            .await
            .context("Failed to retrieve response from initiation request!")?,
    ))
    .context("Failed to deserialize initiation response!")?;

    println!("Unpacking initiation response...");

    let ClientLoginFinishResult {
        message: finalization,
        session_key,
        ..
    } = state.finish(
        args.pass.as_bytes(),
        response,
        ClientLoginFinishParameters::new(None, Identifiers::default(), None),
    )?;

    let request: login::ConclusionRequest = login::ConclusionRequest {
        username: args.user.clone(),
        finalization,
        state: server_state,
    };

    println!("Preparing conclusion request...");

    let request: Request = client
        .request(Method::POST, &format!("{}login/conclude", args.auth_uri))
        .body(postcard::to_allocvec(&request)?)
        .build()?;

    println!("Sending conclusion request...");

    let mut token: EncryptedToken =
        postcard::from_bytes(&client.execute(request).await?.bytes().await?)?;

    println!("Unpacking conclusion response...");

    let mut aes_gcm: Option<Aes256Gcm> = None;
    let aes_gcm: &Aes256Gcm = {
        let hkdf: Hkdf<Sha512_256> = Hkdf::new(None, &session_key);

        let mut key: Zeroizing<Key<Aes256Gcm>> = Zeroizing::new(Key::<Aes256Gcm>::from([0; 32]));

        hkdf.expand(args.user.as_bytes(), key.as_mut_slice())?;

        aes_gcm.insert(Aes256Gcm::new(&*key))
    };

    aes_gcm.decrypt_in_place_detached(&token.nonce, &[], &mut token.token_bytes, &token.tag)?;

    println!("Token: {}", String::from_utf8(token.token_bytes)?);

    Ok(())
}
