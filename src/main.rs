use actix_governor::{Governor, GovernorConfigBuilder};
use actix_rt::spawn;
use actix_web::{middleware, web, App, HttpServer};
use env_logger::Env;
use routes::balance::{add_amount, get_balance};
use routes::health::health;
use routes::merchant::{listen_to_notifications, webhook_listener};
use routes::transactions::{get_transactions, send_transaction};
use routes::user::{get_user, login, register};
use sqlx::postgres::PgPoolOptions;
use std::env;

mod routes;
mod utils;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();

    // Initialize logger
    env_logger::init_from_env(Env::default().default_filter_or("info"));

    // Create connection pool
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to create pool");

    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to migrate the database");

    spawn(async move {
        listen_to_notifications().await;
    });

    // Configure rate limiting
    let governor_conf = GovernorConfigBuilder::default()
        .seconds_per_request(1) // Allow 1 requests per second
        .burst_size(5)
        .finish()
        .unwrap();

    println!("Starting server at http://0.0.0.0:8080");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            // Add security middleware
            .wrap(Governor::new(&governor_conf))
            .wrap(middleware::Compress::default())
            // Enable max body size of ~1mb
            .app_data(web::JsonConfig::default().limit(1000000))
            .service(health)
            .service(register)
            .service(login)
            .service(get_user)
            .service(get_balance)
            .service(add_amount)
            .service(get_transactions)
            .service(send_transaction)
            .service(webhook_listener)
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
