use actix_web::{web::Bytes, HttpResponse};
use anyhow::Result as AnyResult;
use tokio::sync::{
    mpsc::{Receiver as MpscReceiver, Sender as MpscSender},
    oneshot::{
        channel as oneshot_channel, error::RecvError, Receiver as OneshotReceiver,
        Sender as OneshotSender,
    },
};

use lambda_rt::{Response as LambdaResponse, User as LambdaUser};

pub mod modules;
pub mod workers;

type ResponseSender = OneshotSender<AnyResult<LambdaResponse>>;
type ResponseReceiver = OneshotReceiver<AnyResult<LambdaResponse>>;

pub struct Request<User>
where
    User: LambdaUser,
{
    externally_sourced: bool,
    user: User,
    data: Vec<u8>,
    response_sender: ResponseSender,
}

pub type RequestSender<User> = MpscSender<Request<User>>;
pub type RequestReceiver<User> = MpscReceiver<Request<User>>;

pub async fn request_handler<User>(
    user: User,
    body: Bytes,
    sender: RequestSender<User>,
) -> HttpResponse
where
    User: LambdaUser,
{
    let (response_sender, response_receiver): (ResponseSender, ResponseReceiver) =
        oneshot_channel();

    if sender
        .send(Request {
            externally_sourced: true,
            user,
            data: body.to_vec(),
            response_sender,
        })
        .await
        .is_ok()
    {
        let Ok(response): Result<AnyResult<LambdaResponse>, RecvError> = response_receiver.await else {
            return HttpResponse::InternalServerError()
                .body("Failed to receive response from handler!");
        };

        match response {
            Ok(response) => match response {
                LambdaResponse::Success(response) => HttpResponse::Ok().body(response),
                LambdaResponse::Error(response) => {
                    HttpResponse::UnprocessableEntity().body(response)
                }
            },
            Err(error) => HttpResponse::InternalServerError().body(
                format!(
                    "Error occurred!\nContext: {}\nRoot cause: {}\nDebug version: {:?}",
                    error,
                    error.root_cause(),
                    error,
                )
                .into_bytes(),
            ),
        }
    } else {
        HttpResponse::InternalServerError()
            .body("Failed to send request to handler! Channel closed!")
    }
}
