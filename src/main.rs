use actix_web::dev::ServiceRequest;
use actix_web::{error, get, post, web, App, HttpResponse, HttpServer};
use actix_web_httpauth::extractors::bearer::BearerAuth;
use actix_web_httpauth::middleware::HttpAuthentication;
use actix_web_prom::PrometheusMetrics;
use prometheus::{opts, IntCounterVec};
use std::env;
use webhook::Webhook;

// Application-wide configuration for Discord webhook url
struct WebhookURLConfig {
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

#[get("/")]
async fn handle_index() -> HttpResponse {
    HttpResponse::NoContent().finish()
}

#[post("/ping")]
async fn handle_ping(data: web::Data<IntCounterVec>) -> HttpResponse {
    // increment count with dummy_label=""
    data.with_label_values(&[""]).inc();

    HttpResponse::Ok().finish()
}

#[post("/notify")]
async fn handle_notify(
    data: web::Data<WebhookURLConfig>,
) -> Result<HttpResponse, actix_web::Error> {
    // we admit that the request is valid
    match send_webhook(data.webhook_url.clone()).await {
        Ok(_) => Ok(HttpResponse::Ok().finish()),
        Err(_) => Ok(HttpResponse::BadGateway().finish()),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // we do not care the value of the label, so put "dummy_label" as the only label
    let ping_count = IntCounterVec::new(
        opts!("ping_count", "count of valid requests to /ping").namespace("home_intercom_server"),
        &["dummy_label"],
    )
    .unwrap();

    let metrics_server = {
        let ping_count = ping_count.clone();
        let prometheus = PrometheusMetrics::new("home_intercom_server", Some("/metrics"), None);
        prometheus.registry.register(Box::new(ping_count)).unwrap();

        HttpServer::new(move || App::new().wrap(prometheus.clone()))
            .bind("0.0.0.0:8081")?
            .run()
    };

    let app_server = {
        let webhook_url = env::var("INTERCOM_DISCORD_WEBHOOK_URL").expect("webhook url");
        let request_secret = env::var("INTERCOM_REQUEST_SECRET").expect("request secret");

        HttpServer::new(move || {
            let request_secret = request_secret.clone();
            let validate_credentials = move |req: ServiceRequest, credentials: BearerAuth| {
                let valid = credentials.token() == request_secret;
                async move {
                    if valid {
                        Ok(req)
                    } else {
                        Err(error::ErrorUnauthorized("Invalid token."))
                    }
                }
            };

            App::new()
                .wrap(HttpAuthentication::bearer(validate_credentials))
                .data(WebhookURLConfig {
                    webhook_url: webhook_url.clone(),
                })
                .data(ping_count.clone())
                .service(handle_index)
                .service(handle_ping)
                .service(handle_notify)
        })
        .bind("0.0.0.0:8080")?
        .run()
    };

    futures_util::try_join!(app_server, metrics_server)?;
    Ok(())
}
