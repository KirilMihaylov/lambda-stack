use opaque_ke::{RegistrationRequest, RegistrationResponse, RegistrationUpload};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::state::EncryptedServerState;

use super::AuthCipherSuite;

#[derive(Serialize, Deserialize)]
pub struct PersistedServerState {
    pub opaque_setup: Vec<u8>,
    pub user: String,
    pub valid_until: OffsetDateTime,
}

#[derive(Serialize, Deserialize)]
pub struct InitiationRequest {
    pub username: String,
    pub request: RegistrationRequest<AuthCipherSuite>,
}

#[derive(Serialize, Deserialize)]
pub struct InitiationResponse {
    pub response: RegistrationResponse<AuthCipherSuite>,
    pub state: EncryptedServerState<PersistedServerState>,
}

#[derive(Serialize, Deserialize)]
pub struct ConclusionRequest {
    pub username: String,
    pub upload: RegistrationUpload<AuthCipherSuite>,
    pub state: EncryptedServerState<PersistedServerState>,
}
