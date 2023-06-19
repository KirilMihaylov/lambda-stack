use opaque_ke::{
    CredentialFinalization, CredentialRequest, CredentialResponse, ServerLogin, ServerSetup,
};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::state::EncryptedServerState;

use super::AuthCipherSuite;

#[derive(Serialize, Deserialize)]
pub struct PersistedServerState {
    pub server_setup: ServerSetup<AuthCipherSuite>,
    pub server_login: ServerLogin<AuthCipherSuite>,
    pub user: String,
    pub valid_until: OffsetDateTime,
}

#[derive(Serialize, Deserialize)]
pub struct InitiationRequest {
    pub username: String,
    pub request: CredentialRequest<AuthCipherSuite>,
}

#[derive(Serialize, Deserialize)]
pub struct InitiationResponse {
    pub response: CredentialResponse<AuthCipherSuite>,
    pub state: EncryptedServerState<PersistedServerState>,
}

#[derive(Serialize, Deserialize)]
pub struct ConclusionRequest {
    pub username: String,
    pub finalization: CredentialFinalization<AuthCipherSuite>,
    pub state: EncryptedServerState<PersistedServerState>,
}
