use crate::utils::response::{json_response, ApiResponse, MessageData};
use actix_web::{post, web, Responder};
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{Consumer as KafkaConsumer, StreamConsumer};
use rdkafka::message::Message;
use rdkafka::producer::{FutureProducer, FutureRecord};
use serde::{Deserialize, Serialize};
use std::env;
use std::time::Duration;

#[derive(Deserialize, Serialize)]
pub struct WebhookPayload {
    transaction_id: uuid::Uuid,
    status: String,
    amount: rust_decimal::Decimal,
}

#[derive(Deserialize, Serialize)]
pub struct WebhookMessage {
    webhook_url: String,
    payload: WebhookPayload,
}

// assume this is running on a differnt server
#[post("/merchant/webhook")]
pub async fn webhook_listener(payload: web::Json<WebhookPayload>) -> impl Responder {
    let response = MessageData {
        message: format!(
            "Webhook received - Transaction ID: {}, Status: {}, Amount: {}",
            payload.transaction_id, payload.status, payload.amount
        ),
    };

    println!("webhook message: {}", response.message);

    json_response(ApiResponse::success(response))
}

// this should be separate worker
pub async fn listen_to_notifications() {
    let kafka_bootstrap_servers =
        env::var("KAFKA_BOOTSTRAP_SERVERS").expect("KAFKA_BOOTSTRAP_SERVERS must be set");
    let kafka_topic = env::var("KAFKA_WEBHOOK_TOPIC").expect("KAFKA_WEBHOOK_TOPIC must be set");

    let producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", &kafka_bootstrap_servers)
        .set("message.timeout.ms", "5000")
        .create()
        .expect("Failed to create Kafka producer");

    let mut conn = sqlx::postgres::PgListener::connect(env!("DATABASE_URL"))
        .await
        .expect("Failed to connect");

    conn.listen("transaction_insert")
        .await
        .expect("Failed to listen to notifications");

    loop {
        let notification = conn.recv().await.expect("Failed to receive notification");
        if let Ok(payload) = serde_json::from_str::<WebhookPayload>(&notification.payload()) {
            let webhook_message = WebhookMessage {
                webhook_url: "http://localhost:8080/merchant/webhook".to_string(),
                payload,
            };

            let message = serde_json::to_string(&webhook_message)
                .expect("Failed to serialize webhook message");

            producer
                .send(
                    FutureRecord::to(&kafka_topic).payload(&message).key(""),
                    Duration::from_secs(5),
                )
                .await
                .expect("Failed to send message to Kafka");
        }
    }
}

pub async fn process_webhooks() {
    let kafka_bootstrap_servers =
        env::var("KAFKA_BOOTSTRAP_SERVERS").expect("KAFKA_BOOTSTRAP_SERVERS must be set");
    let kafka_topic = env::var("KAFKA_WEBHOOK_TOPIC").expect("KAFKA_WEBHOOK_TOPIC must be set");

    let consumer: StreamConsumer = loop {
        match ClientConfig::new()
            .set("group.id", "webhook_processor")
            .set("bootstrap.servers", &kafka_bootstrap_servers)
            .set("enable.auto.commit", "true")
            .set("auto.offset.reset", "earliest")
            .create()
        {
            Ok(consumer) => break consumer,
            Err(e) => {
                eprintln!(
                    "Failed to create Kafka consumer: {}. Retrying in 5 seconds...",
                    e
                );
                actix_rt::time::sleep(Duration::from_secs(5)).await;
            }
        }
    };

    // Add retry logic for topic subscription
    let max_retries = 5;
    let mut retry_count = 0;

    loop {
        match consumer.subscribe(&[&kafka_topic]) {
            Ok(_) => {
                println!("Successfully subscribed to topic: {}", kafka_topic);
                break;
            }
            Err(e) => {
                retry_count += 1;
                eprintln!(
                    "Failed to subscribe to topic (attempt {}/{}): {}. Retrying in 5 seconds...",
                    retry_count, max_retries, e
                );

                if retry_count >= max_retries {
                    panic!(
                        "Failed to subscribe to Kafka topic after {} attempts",
                        max_retries
                    );
                }

                actix_rt::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }

    loop {
        match consumer.recv().await {
            Ok(message) => {
                if let Some(payload) = message.payload() {
                    if let Ok(message_str) = std::str::from_utf8(payload) {
                        if let Ok(webhook_message) =
                            serde_json::from_str::<WebhookMessage>(message_str)
                        {
                            let client = awc::Client::default();
                            // Using spawn to handle the async HTTP request
                            actix_rt::spawn(async move {
                                let _ = client
                                    .post(&webhook_message.webhook_url)
                                    .send_json(&webhook_message.payload)
                                    .await;
                            });
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Error while receiving message: {}", e);
                // Using actix_rt::spawn instead of tokio
                actix_rt::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }
}
