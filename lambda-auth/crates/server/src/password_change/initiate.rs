use actix_web::{
    http::header::ContentType,
    post,
    web::{self, Data},
    HttpResponse,
};
use opaque_ke::{
    errors::ProtocolError, ServerRegistration, ServerRegistrationStartResult, ServerSetup,
};
use rand_core::OsRng;
use time::OffsetDateTime;
use zeroize::Zeroizing;

use lambda_auth::{
    middleware::AuthenticatedUser,
    password_change::{InitiationRequest, InitiationResponse, PersistedServerState},
    state::{EncryptedServerState, EncryptionError, TEN_MINUTES},
    AuthCipherSuite,
};

use crate::OpaqueKeyPair;

#[post("/initiate")]
pub async fn handle_initiate(
    auth_private_key: Data<Zeroizing<Vec<u8>>>,
    opaque_auth_keypair: Data<OpaqueKeyPair>,
    user: AuthenticatedUser,
    payload: web::Bytes,
) -> Result<HttpResponse, actix_web::Error> {
    let Ok(InitiationRequest {
               request,
           }): Result<InitiationRequest, postcard::Error> =
        postcard::from_bytes(&payload) else {
        return Ok(HttpResponse::BadRequest().finish());
    };

    let opaque_setup: ServerSetup<AuthCipherSuite> =
        ServerSetup::new_with_key(&mut OsRng, OpaqueKeyPair::clone(&opaque_auth_keypair));

    let Ok(start_result): Result<ServerRegistrationStartResult<AuthCipherSuite>, ProtocolError> =
        ServerRegistration::start(
            &opaque_setup,
            request,
            user.username().as_bytes(),
        ) else {
        return Ok(HttpResponse::BadRequest().body("2"));
    };

    let Ok(state): Result<EncryptedServerState<PersistedServerState>, EncryptionError> =
        EncryptedServerState::encrypt(&PersistedServerState {
            opaque_setup: if let Ok(opaque_setup) = postcard::to_allocvec(&opaque_setup) {
                opaque_setup
            } else {
                return Ok(HttpResponse::InternalServerError().body("2.5"));
            },
            valid_until: OffsetDateTime::now_utc() + TEN_MINUTES,
        }, &auth_private_key, user.username()) else {
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
