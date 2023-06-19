use actix_web::{
    post,
    web::{self, Data},
    HttpResponse,
};
use opaque_ke::ServerRegistration;
use sqlx::PgPool;
use subtle::ConstantTimeEq as _;
use time::OffsetDateTime;
use zeroize::Zeroizing;

use lambda_auth::{
    registration::{ConclusionRequest, PersistedServerState},
    state::DecryptionError,
    AuthCipherSuite,
};

#[post("/conclude")]
pub async fn handle_conclude(
    auth_private_key: Data<Zeroizing<Vec<u8>>>,
    database: Data<PgPool>,
    payload: web::Bytes,
) -> Result<HttpResponse, actix_web::Error> {
    let Ok(request): Result<ConclusionRequest, postcard::Error> =
        postcard::from_bytes(&payload) else {
        return Ok(HttpResponse::BadRequest().body("1"));
    };

    let Ok(state): Result<PersistedServerState, DecryptionError> =
        request.state.decrypt(&auth_private_key, &request.username) else {
        return Ok(HttpResponse::BadRequest().body("2"));
    };

    if request.username.len() != state.user.len() {
        return Ok(HttpResponse::BadRequest().body("3"));
    }

    if (!request.username.as_bytes().ct_eq(state.user.as_bytes())).into() {
        return Ok(HttpResponse::BadRequest().body("4"));
    }

    if state.valid_until <= OffsetDateTime::now_utc() {
        return Ok(HttpResponse::BadRequest().body("5"));
    }

    let password_file: ServerRegistration<AuthCipherSuite> =
        ServerRegistration::finish(request.upload);

    let Ok(result) = sqlx::query(include_str!("../../../../sql/upload_password_file.sql"))
        .bind(request.username)
        .bind(state.opaque_setup)
        .bind(&*password_file.serialize())
        .execute(database.get_ref())
        .await else {
        return Ok(HttpResponse::InternalServerError().body("7"));
    };

    debug_assert_eq!(result.rows_affected(), 1);

    Ok(HttpResponse::Ok().finish())
}
