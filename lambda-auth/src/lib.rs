#![forbid(
    rust_2018_compatibility,
    deprecated_in_future,
    unsafe_code,
    clippy::pedantic
)]
#![deny(rust_2021_compatibility, warnings)]

use aes_gcm::{AeadCore, Aes256Gcm, Nonce, Tag};
use argon2::Argon2;
use ed25519_dalek::Signature;
use opaque_ke::{
    key_exchange::tripledh::TripleDh, keypair::KeyPair, ksf::Ksf, CipherSuite, Ristretto255,
};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

pub mod login;
pub mod middleware;
pub mod password_change;
pub mod registration;
pub mod state;

#[derive(Copy, Clone)]
pub struct AuthCipherSuite;

impl CipherSuite for AuthCipherSuite
where
    Argon2<'static>: Ksf,
{
    type OprfCs = Ristretto255;
    type KeGroup = Ristretto255;
    type KeyExchange = TripleDh;
    type Ksf = Argon2<'static>;
}

pub type OpaqueKeyPair = KeyPair<<AuthCipherSuite as CipherSuite>::KeGroup>;

#[must_use]
#[derive(Debug, Serialize, Deserialize)]
pub struct Token {
    user: String,
    expires: OffsetDateTime,
}

impl Token {
    pub fn new(user: String, expires: OffsetDateTime) -> Self {
        Self { user, expires }
    }

    #[must_use]
    pub fn user(&self) -> &str {
        &self.user
    }

    #[must_use]
    pub fn expires(&self) -> OffsetDateTime {
        self.expires
    }
}

#[must_use]
#[derive(Serialize, Deserialize)]
pub struct SignedToken {
    token_bytes: Vec<u8>,
    signature: Signature,
}

impl SignedToken {
    pub fn new(token_bytes: Vec<u8>, signature: Signature) -> Self {
        Self {
            token_bytes,
            signature,
        }
    }

    #[must_use]
    pub fn token_bytes(&self) -> &[u8] {
        &self.token_bytes
    }

    #[must_use]
    pub fn signature(&self) -> &Signature {
        &self.signature
    }
}

#[must_use]
#[derive(Serialize, Deserialize)]
pub struct EncryptedToken {
    pub nonce: Nonce<<Aes256Gcm as AeadCore>::NonceSize>,
    pub tag: Tag,
    pub token_bytes: Vec<u8>,
}
