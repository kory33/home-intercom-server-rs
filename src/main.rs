use actix_web::{error, post, web, App, Error, HttpResponse, HttpServer};
use futures_util::StreamExt;
use std::env;

struct GlobalState {
    request_secret: String,
}

async fn extract_capped_body_string(mut payload: web::Payload) -> Result<String, Error> {
    // max payload size is 256
    const MAX_BODY_SIZE: usize = 256;

    let mut body = web::BytesMut::new();
    while let Some(chunk) = payload.next().await {
        let chunk = chunk?;
        if (body.len() + chunk.len()) > MAX_BODY_SIZE {
            return Err(error::ErrorBadRequest("request too large"));
        }
        body.extend_from_slice(&chunk);
    }

    match String::from_utf8(body.to_vec()) {
        Ok(s) => Ok(s),
        Err(_) => Err(error::ErrorBadRequest("body not recognized")),
    }
}

#[post("/")]
async fn handle_request(
    data: web::Data<GlobalState>,
    payload: web::Payload,
) -> Result<HttpResponse, Error> {
    if data.request_secret == extract_capped_body_string(payload).await? {
        // we admit that the request is valid
        Ok(HttpResponse::Ok().finish())
    } else {
        // request body did not equal the request secret
        Ok(HttpResponse::Unauthorized().finish())
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let request_secret = env::var("INTERCOM_REQUEST_SECRET").expect("request secret");

    HttpServer::new(move || {
        App::new()
            .data(GlobalState {
                request_secret: request_secret.clone(),
            })
            .service(handle_request)
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
