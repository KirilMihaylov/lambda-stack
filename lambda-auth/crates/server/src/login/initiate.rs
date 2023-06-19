use actix_web::{
    http::header::ContentType,
    post,
    web::{self, Data},
    HttpResponse,
};
use opaque_ke::{
    errors::ProtocolError, ServerLogin, ServerLoginStartResult, ServerRegistration, ServerSetup,
};
use rand_core::OsRng;
use sqlx::PgPool;
use time::OffsetDateTime;
use zeroize::Zeroizing;

use lambda_auth::{
    login::{InitiationRequest, InitiationResponse, PersistedServerState},
    state::{EncryptedServerState, EncryptionError, TEN_MINUTES},
    AuthCipherSuite,
};

use crate::OpaqueKeyPair;

#[post("/initiate")]
pub async fn handle_initiate(
    auth_private_key: Data<Zeroizing<Vec<u8>>>,
    _opaque_auth_keypair: Data<OpaqueKeyPair>,
    database: Data<PgPool>,
    payload: web::Bytes,
) -> Result<HttpResponse, actix_web::Error> {
    let Ok(InitiationRequest {
               username,
               request: credential_request,
           }): Result<InitiationRequest, postcard::Error> =
        postcard::from_bytes(&payload) else {
        return Ok(HttpResponse::BadRequest().finish());
    };

    let Ok(maybe_result): Result<Option<(Vec<u8>, Vec<u8>)>, sqlx::Error> = sqlx::query_as(
        include_str!("../../../../sql/fetch_password_file.sql")
    )
        .bind(username.as_str())
        .fetch_optional(database.get_ref())
        .await else {
        return Ok(HttpResponse::InternalServerError().body("1"));
    };

    let (opaque_setup, password_file): (
        ServerSetup<AuthCipherSuite>,
        Option<ServerRegistration<AuthCipherSuite>>,
    ) = if let Some((opaque_setup, password_file)) = maybe_result {
        let Ok(opaque_setup): Result<ServerSetup<AuthCipherSuite>, postcard::Error> = postcard::from_bytes(&opaque_setup) else {
            return Ok(HttpResponse::InternalServerError().body("1.3"));
        };

        let Ok(password_file): Result<ServerRegistration<AuthCipherSuite>, postcard::Error> = postcard::from_bytes(&password_file) else {
            return Ok(HttpResponse::InternalServerError().body("1.7"));
        };

        (opaque_setup, Some(password_file))
    } else {
        (ServerSetup::new(&mut OsRng), None)
    };

    let Ok(start_result): Result<ServerLoginStartResult<AuthCipherSuite>, ProtocolError> = ServerLogin::start(
        &mut OsRng,
        &opaque_setup,
        password_file,
        credential_request,
        username.as_bytes(),
        Default::default(),
    ) else {
        return Ok(HttpResponse::BadRequest().body("2"));
    };

    let Ok(state): Result<EncryptedServerState<PersistedServerState>, EncryptionError> = EncryptedServerState::encrypt(&PersistedServerState {
        server_setup: opaque_setup,
        server_login: start_result.state,
        user: username.clone(),
        valid_until: OffsetDateTime::now_utc() + TEN_MINUTES,
    }, &auth_private_key, &username) else {
        return Ok(HttpResponse::InternalServerError().body("3"));
    };

    let Ok(response_payload) = postcard::to_allocvec(&InitiationResponse {
        response: start_result.message,
        state,
    }) else {
        return Ok(HttpResponse::InternalServerError().body("4"));
    };

    Ok(HttpResponse::Ok()
        .content_type(ContentType::octet_stream())
        .body(response_payload))
}
