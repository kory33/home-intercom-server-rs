use actix_web::dev::ServiceRequest;
use actix_web::{error, post, web, App, HttpResponse, HttpServer};
use actix_web_httpauth::extractors::bearer::BearerAuth;
use actix_web_httpauth::middleware::HttpAuthentication;
use std::env;
use webhook::Webhook;

struct GlobalState {
    webhook_url: String,
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

#[post("/ping")]
async fn handle_ping(data: web::Data<GlobalState>) -> Result<HttpResponse, actix_web::Error> {
    // TODO increment counter
    Ok(HttpResponse::Ok().finish())
}

#[post("/notify")]
async fn handle_notify(data: web::Data<GlobalState>) -> Result<HttpResponse, actix_web::Error> {
    // we admit that the request is valid
    match send_webhook(data.webhook_url.clone()).await {
        Ok(_) => Ok(HttpResponse::Ok().finish()),
        Err(_) => Ok(HttpResponse::BadGateway().finish()),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let webhook_url = env::var("INTERCOM_DISCORD_WEBHOOK_URL").expect("webhook url");
    let request_secret = env::var("INTERCOM_REQUEST_SECRET").expect("request secret");

    HttpServer::new(move || {
        let request_secret = request_secret.clone();

        let state = GlobalState {
            webhook_url: webhook_url.clone(),
        };

        let validate_credentials = move |req: ServiceRequest, credentials: BearerAuth| {
            let valid = credentials.token() == request_secret;
            async move {
                if valid {
                    Ok(req)
                } else {
                    Err(error::ErrorUnauthorized(""))
                }
            }
        };

        App::new()
            .wrap(HttpAuthentication::bearer(validate_credentials))
            .data(state)
            .service(handle_ping)
            .service(handle_notify)
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}
