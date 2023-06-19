use std::{marker::PhantomData, time::Duration};

use aes_gcm::{
    aead::{AeadInPlace as _, KeyInit as _, Nonce},
    Aes256Gcm, Key, Tag,
};
use hkdf::Hkdf;
use rand_core::{OsRng, RngCore as _};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha2::Sha512;
use thiserror::Error;
use zeroize::{Zeroize, Zeroizing};

pub type StateSalt = [u8; 32];

const OUTPUT_KEY_MATERIAL_LENGTH: usize = 32;
const SYMMETRIC_KEY_LENGTH: usize = 32;
const _: () = if SYMMETRIC_KEY_LENGTH != OUTPUT_KEY_MATERIAL_LENGTH {
    panic!("Symmetric key's length is different that KDF's output key material's length!");
};

#[derive(Serialize, Deserialize)]
pub struct EncryptedServerState<State> {
    ciphertext: Vec<u8>,
    nonce: Nonce<Aes256Gcm>,
    tag: Tag,
    salt: StateSalt,
    #[serde(skip, default)]
    _state: PhantomData<State>,
}

impl<State> EncryptedServerState<State> {
    pub fn encrypt<K>(
        state: &State,
        key: &Zeroizing<K>,
        username: &str,
    ) -> Result<Self, EncryptionError>
    where
        State: Serialize,
        K: AsRef<[u8]> + Zeroize,
    {
        let mut state: Vec<u8> = postcard::to_allocvec(&state)?;

        let salt: StateSalt = new_salt()?;

        let mut cipher: Option<Aes256Gcm> = None;
        let cipher: &mut Aes256Gcm = new_cipher_in_place(key, &salt, username, &mut cipher)?;

        let nonce: Nonce<Aes256Gcm> = new_nonce()?;

        let tag: Tag = cipher.encrypt_in_place_detached(&nonce, username.as_bytes(), &mut state)?;

        Ok(Self {
            ciphertext: state,
            nonce,
            tag,
            salt,
            _state: PhantomData,
        })
    }

    pub fn decrypt<K>(
        mut self,
        key: &Zeroizing<K>,
        username: &str,
    ) -> Result<State, DecryptionError>
    where
        K: AsRef<[u8]> + Zeroize,
        State: DeserializeOwned,
    {
        {
            let mut cipher: Option<Aes256Gcm> = None;

            new_cipher_in_place(key, &self.salt, username, &mut cipher)?
                .decrypt_in_place_detached(
                    &self.nonce,
                    username.as_bytes(),
                    &mut self.ciphertext,
                    &self.tag,
                )?;
        }

        postcard::from_bytes(&self.ciphertext).map_err(Into::into)
    }
}

fn new_cipher_in_place<'r, K>(
    key: &Zeroizing<K>,
    salt: &StateSalt,
    username: &str,
    cipher: &'r mut Option<Aes256Gcm>,
) -> Result<&'r mut Aes256Gcm, hkdf::InvalidLength>
where
    K: AsRef<[u8]> + Zeroize,
{
    let kdf: Hkdf<Sha512> = Hkdf::new(Some(salt.as_slice()), key.as_ref());

    let mut key_material: Zeroizing<[u8; OUTPUT_KEY_MATERIAL_LENGTH]> =
        Zeroizing::new([0; OUTPUT_KEY_MATERIAL_LENGTH]);

    kdf.expand(username.as_bytes(), key_material.as_mut_slice())?;

    Ok(cipher.insert(Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(
        &key_material[..SYMMETRIC_KEY_LENGTH],
    ))))
}

fn new_salt() -> Result<StateSalt, rand_core::Error> {
    let mut salt: StateSalt = StateSalt::default();

    OsRng.try_fill_bytes(salt.as_mut_slice())?;

    Ok(salt)
}

fn new_nonce() -> Result<Nonce<Aes256Gcm>, rand_core::Error> {
    let mut nonce: Nonce<Aes256Gcm> = Nonce::<Aes256Gcm>::default();

    OsRng.try_fill_bytes(nonce.as_mut_slice())?;

    Ok(nonce)
}

#[derive(Debug, Error)]
pub enum EncryptionError {
    #[error("Serialization of state failed!")]
    Serialization(#[from] postcard::Error),
    #[error("Used PRNG has low entropy!")]
    LowEntropy(#[from] rand_core::Error),
    #[error("Key derivation failed because of internal error!")]
    KeyDerivation(#[from] hkdf::InvalidLength),
    #[error("Encryption of serialized state failed!")]
    Encryption(#[from] aes_gcm::Error),
}

#[derive(Debug, Error)]
pub enum DecryptionError {
    #[error("Key derivation failed because of internal error!")]
    KeyDerivation(#[from] hkdf::InvalidLength),
    #[error("Encryption of serialized state failed!")]
    Encryption(#[from] aes_gcm::Error),
    #[error("Deserialization of state failed!")]
    Deserialization(#[from] postcard::Error),
}

pub const TEN_MINUTES: Duration = Duration::from_secs(/* 10 Minutes */ 600);
