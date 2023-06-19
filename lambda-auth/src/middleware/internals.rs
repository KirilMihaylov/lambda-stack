use std::{
    future::Future,
    pin::Pin,
    rc::Rc,
    task::{Context, Poll},
};

use actix_web::{
    dev::{Service, ServiceRequest, ServiceResponse},
    http::header::AUTHORIZATION,
    Error as ActixError, HttpMessage, HttpResponse,
};
use ed25519_dalek::VerifyingKey;
use time::OffsetDateTime;

use crate::{SignedToken, Token};

use super::User;

pub struct AuthService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse>,
    S::Error: Into<ActixError>,
    S::Future: Unpin,
{
    token_verifying_key: Rc<VerifyingKey>,
    service: Rc<S>,
}

impl<S> AuthService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse>,
    S::Error: Into<ActixError>,
    S::Future: Unpin,
{
    pub(crate) fn new(token_verifying_key: Rc<VerifyingKey>, service: Rc<S>) -> Self {
        Self {
            token_verifying_key,
            service,
        }
    }
}

impl<S> Service<ServiceRequest> for AuthService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse>,
    S::Error: Into<ActixError>,
    S::Future: Unpin,
{
    type Response = ServiceResponse;
    type Error = ActixError;
    type Future = AuthFuture<S>;

    fn poll_ready(&self, ctx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(ctx).map_err(Into::into)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        AuthFuture('response: {
            'validation: {
                let Some(authorization) = req.headers().get(AUTHORIZATION) else { break 'validation; };

                let Ok(authorization) = authorization.to_str() else { break 'validation; };

                if !authorization.is_ascii() {
                    break 'validation;
                };

                let Ok(token) = data_encoding::BASE64.decode(authorization.as_bytes()) else { break 'validation; };

                let Ok(token): Result<SignedToken, postcard::Error> = postcard::from_bytes(&token) else { break 'validation; };

                if self
                    .token_verifying_key
                    .verify_strict(&token.token_bytes, &token.signature)
                    .is_err()
                {
                    break 'validation;
                }

                let Ok(token) = postcard::from_bytes::<Token>(&token.token_bytes) else { break 'validation; };

                if token.expires() <= OffsetDateTime::now_utc() {
                    break 'validation;
                }

                {
                    let insertion_result: Option<Rc<User>> =
                        req.extensions_mut().insert(Rc::new(User::new(token.user)));

                    debug_assert!(insertion_result.is_none());
                }

                break 'response FutureVariant::ServiceFuture(self.service.call(req));
            }

            FutureVariant::Response(Some(
                req.into_response(HttpResponse::Unauthorized().finish()),
            ))
        })
    }
}

pub struct AuthFuture<S>(FutureVariant<S>)
where
    S: Service<ServiceRequest, Response = ServiceResponse>,
    S::Error: Into<ActixError>,
    S::Future: Unpin;

impl<S> Future for AuthFuture<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse>,
    S::Error: Into<ActixError>,
    S::Future: Unpin,
{
    type Output = Result<ServiceResponse, ActixError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.get_mut().0).poll(cx)
    }
}

enum FutureVariant<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse>,
    S::Error: Into<ActixError>,
    S::Future: Unpin,
{
    ServiceFuture(S::Future),
    Response(Option<ServiceResponse>),
}

impl<S> Future for FutureVariant<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse>,
    S::Error: Into<ActixError>,
    S::Future: Unpin,
{
    type Output = Result<ServiceResponse, ActixError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.get_mut() {
            FutureVariant::ServiceFuture(future) => Pin::new(future)
                .poll(cx)
                .map(|output| Result::map_err(output, Into::into)),
            FutureVariant::Response(response) => {
                if let Some(response) = response.take() {
                    Poll::Ready(Ok(response))
                } else {
                    unreachable!()
                }
            }
        }
    }
}
