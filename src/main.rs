use actix_governor::{Governor, GovernorConfigBuilder};
use actix_web::{middleware, web, App, HttpServer};
use env_logger::Env;
use rdkafka::admin::{AdminClient, AdminOptions, NewTopic, TopicReplication};
use rdkafka::client::DefaultClientContext;
use rdkafka::ClientConfig;
use routes::balance::{add_amount, get_balance};
use routes::health::health;
use routes::merchant::{listen_to_notifications, process_webhooks, webhook_listener};
use routes::transactions::{get_transactions, send_transaction};
use routes::user::{get_user, login, register};
use sqlx::postgres::PgPoolOptions;
use std::env;

mod routes;
mod utils;

// Create Kafka topic
async fn create_kafka_topic() {
    let kafka_bootstrap_servers =
        env::var("KAFKA_BOOTSTRAP_SERVERS").expect("KAFKA_BOOTSTRAP_SERVERS must be set");
    let kafka_topic = env::var("KAFKA_WEBHOOK_TOPIC").expect("KAFKA_WEBHOOK_TOPIC must be set");

    let admin_client: AdminClient<DefaultClientContext> = ClientConfig::new()
        .set("bootstrap.servers", &kafka_bootstrap_servers)
        .create()
        .expect("Failed to create Kafka admin client");

    let topic = NewTopic::new(
        &kafka_topic,
        1,                          // num_partitions
        TopicReplication::Fixed(1), // replication_factor
    );

    admin_client
        .create_topics(&[topic], &AdminOptions::new())
        .await
        .expect("Failed to create Kafka topic");
}

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

    // Create Kafka topic
    create_kafka_topic().await;

    //seprate worker, db transection Insert tracking and kafka producer
    std::thread::spawn(|| {
        actix_rt::System::new().block_on(async {
            listen_to_notifications().await;
        });
    });

    //seprate worker, kafka consumer and webhook processor
    std::thread::spawn(|| {
        actix_rt::System::new().block_on(async {
            process_webhooks().await;
        });
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
