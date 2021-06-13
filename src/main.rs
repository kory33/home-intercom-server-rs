use actix_web::{error, post, web, App, Error, HttpResponse, HttpServer};
use futures_util::StreamExt;
use std::env;
use webhook::Webhook;

struct GlobalState {
    request_secret: String,
    webhook_url: String,
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

async fn send_webhook(webhook_url: String) -> Result<(), Box<dyn std::error::Error>> {
    Webhook::from_url(webhook_url.as_str())
        .send(|message| {
            message
                .username("Intercom notifier")
                .avatar_url("https://github.com/kory33/home-intercom-server-rs/raw/master/assets/240px-Speaker_Icon.jpg")
                .content("@everyone")
                .embed(|embed| {
                    embed.title("The intercom just rang!")
                })
        })
        .await
}

#[post("/")]
async fn handle_request(
    data: web::Data<GlobalState>,
    payload: web::Payload,
) -> Result<HttpResponse, Error> {
    if data.request_secret == extract_capped_body_string(payload).await? {
        // we admit that the request is valid
        match send_webhook(data.webhook_url.clone()).await {
            Ok(_) => Ok(HttpResponse::Ok().finish()),
            Err(_) => Ok(HttpResponse::BadGateway().finish()),
        }
    } else {
        // request body did not equal the request secret
        Ok(HttpResponse::Unauthorized().finish())
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let webhook_url = env::var("INTERCOM_DISCORD_WEBHOOK_URL").expect("webhook url");
    let request_secret = env::var("INTERCOM_REQUEST_SECRET").expect("request secret");

    HttpServer::new(move || {
        App::new()
            .data(GlobalState {
                request_secret: request_secret.clone(),
                webhook_url: webhook_url.clone(),
            })
            .service(handle_request)
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
