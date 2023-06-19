use actix_web::{
    post,
    web::{self, Data},
    HttpResponse,
};
use opaque_ke::ServerRegistration;
use sqlx::PgPool;
use time::OffsetDateTime;
use zeroize::Zeroizing;

use lambda_auth::{
    middleware::AuthenticatedUser,
    password_change::{ConclusionRequest, PersistedServerState},
    state::DecryptionError,
    AuthCipherSuite,
};

#[post("/conclude")]
pub async fn handle_conclude(
    auth_private_key: Data<Zeroizing<Vec<u8>>>,
    database: Data<PgPool>,
    user: AuthenticatedUser,
    payload: web::Bytes,
) -> Result<HttpResponse, actix_web::Error> {
    let Ok(request): Result<ConclusionRequest, postcard::Error> =
        postcard::from_bytes(&payload) else {
        return Ok(HttpResponse::BadRequest().body("1"));
    };

    let Ok(state): Result<PersistedServerState, DecryptionError> =
        request.state.decrypt(&auth_private_key, &user.username()) else {
        return Ok(HttpResponse::BadRequest().body("2"));
    };

    if state.valid_until <= OffsetDateTime::now_utc() {
        return Ok(HttpResponse::BadRequest().body("5"));
    }

    let password_file: ServerRegistration<AuthCipherSuite> =
        ServerRegistration::finish(request.upload);

    let Ok(result) = sqlx::query(include_str!("../../../../sql/upload_password_file.sql"))
        .bind(user.username())
        .bind(state.opaque_setup)
        .bind(&*password_file.serialize())
        .execute(database.get_ref())
        .await else {
        return Ok(HttpResponse::InternalServerError().body("7"));
    };

    debug_assert_eq!(result.rows_affected(), 1);

    Ok(HttpResponse::Ok().finish())
}
