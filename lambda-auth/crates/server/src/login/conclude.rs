use actix_web::{
    http::header::ContentType,
    post,
    web::{self, Data},
    HttpResponse,
};
use aes_gcm::{AeadCore, AeadInPlace, Aes256Gcm, Key, KeyInit, Nonce};
use ed25519_dalek::{Signature, Signer as _, SigningKey};
use hkdf::Hkdf;
use opaque_ke::ServerLoginFinishResult;
use rand_core::{OsRng, RngCore};
use sha2::Sha512_256;
use subtle::ConstantTimeEq;
use time::OffsetDateTime;
use zeroize::Zeroizing;

use lambda_auth::{
    login::{ConclusionRequest, PersistedServerState},
    state::{DecryptionError, TEN_MINUTES},
    EncryptedToken, SignedToken, Token,
};

#[post("/conclude")]
pub async fn handle_conclude(
    auth_private_key: Data<Zeroizing<Vec<u8>>>,
    signing_keypair: Data<SigningKey>,
    payload: web::Bytes,
) -> Result<HttpResponse, actix_web::Error> {
    let Ok(request): Result<ConclusionRequest, postcard::Error> = postcard::from_bytes(&payload) else {
        return Ok(HttpResponse::BadRequest().body("1"));
    };

    let Ok(state): Result<PersistedServerState, DecryptionError> = request.state.decrypt(&auth_private_key, &request.username) else {
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

    let mut aes_gcm: Option<Aes256Gcm> = None;
    let aes_gcm: &Aes256Gcm = if let Ok(ServerLoginFinishResult { session_key }) =
        state.server_login.finish(request.finalization)
    {
        let hkdf: Hkdf<Sha512_256> = Hkdf::new(None, &session_key);

        let mut key: Zeroizing<Key<Aes256Gcm>> = Zeroizing::new(Key::<Aes256Gcm>::from([0; 32]));

        if hkdf
            .expand(request.username.as_bytes(), key.as_mut_slice())
            .is_err()
        {
            return Ok(HttpResponse::InternalServerError().body("7"));
        }

        aes_gcm.insert(Aes256Gcm::new(&*key))
    } else {
        return Ok(HttpResponse::BadRequest().body("8"));
    };

    let token: Token = Token::new(state.user, OffsetDateTime::now_utc() + TEN_MINUTES);

    let Ok(token_bytes): Result<Vec<u8>, postcard::Error> = postcard::to_allocvec(&token) else {
        return Ok(HttpResponse::InternalServerError().body("9"));
    };

    let signature: Signature = signing_keypair.sign(&token_bytes);

    let mut token_bytes: Vec<u8> =
        if let Ok(bytes) = postcard::to_allocvec(&SignedToken::new(token_bytes, signature)) {
            data_encoding::BASE64.encode(&bytes).into_bytes()
        } else {
            return Ok(HttpResponse::InternalServerError().body("10"));
        };

    let token_bytes = {
        let mut nonce: Nonce<<Aes256Gcm as AeadCore>::NonceSize> = Nonce::from([0; 12]);

        if OsRng.try_fill_bytes(&mut nonce).is_err() {
            return Ok(HttpResponse::InternalServerError().body("11"));
        }

        let Ok(tag) = aes_gcm.encrypt_in_place_detached(&nonce, &[], &mut token_bytes) else {
            return Ok(HttpResponse::InternalServerError().body("12"));
        };

        if let Ok(token_bytes) = postcard::to_allocvec(&EncryptedToken {
            nonce,
            tag,
            token_bytes,
        }) {
            token_bytes
        } else {
            return Ok(HttpResponse::InternalServerError().body("13"));
        }
    };

    Ok(HttpResponse::Ok()
        .content_type(ContentType::octet_stream())
        .body(token_bytes))
}
