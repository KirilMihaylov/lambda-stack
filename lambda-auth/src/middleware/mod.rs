use std::{
    borrow::Borrow,
    future::{ready, Ready},
    ops::Deref,
    rc::Rc,
};

use actix_web::{
    dev::{Payload, Service, ServiceRequest, ServiceResponse, Transform},
    Error as ActixError, FromRequest, HttpMessage, HttpRequest, ResponseError,
};
use ed25519_dalek::VerifyingKey;
use thiserror::Error;

use self::internals::AuthService;

mod internals;

#[derive(Clone)]
pub struct Auth {
    token_verifying_key: Rc<VerifyingKey>,
}

impl Auth {
    pub fn new(verifying_key: VerifyingKey) -> Self {
        Self {
            token_verifying_key: Rc::new(verifying_key),
        }
    }
}

impl<S> Transform<S, ServiceRequest> for Auth
where
    S: Service<ServiceRequest, Response = ServiceResponse>,
    S::Error: Into<ActixError>,
    S::Future: Unpin,
{
    type Response = ServiceResponse;
    type Error = ActixError;
    type Transform = AuthService<S>;
    type InitError = ();
    type Future = Ready<Result<AuthService<S>, ()>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(AuthService::new(
            self.token_verifying_key.clone(),
            Rc::new(service),
        )))
    }
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct User {
    username: String,
}

impl User {
    const fn new(username: String) -> Self {
        Self { username }
    }

    pub fn username(&self) -> &str {
        &self.username
    }
}

#[derive(Clone)]
pub struct AuthenticatedUser(Rc<User>);

impl Deref for AuthenticatedUser {
    type Target = User;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl AsRef<User> for AuthenticatedUser {
    fn as_ref(&self) -> &User {
        self.0.deref()
    }
}

impl Borrow<User> for AuthenticatedUser {
    fn borrow(&self) -> &User {
        self.0.deref()
    }
}

impl FromRequest for AuthenticatedUser {
    type Error = NoUserSetError;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        ready(
            req.extensions()
                .get::<Rc<User>>()
                .cloned()
                .map(Self)
                .ok_or(NoUserSetError),
        )
    }
}

#[derive(Debug, Error)]
#[error("Authenticated user not set! Authentication middleware not attached!")]
pub struct NoUserSetError;

impl ResponseError for NoUserSetError {}
