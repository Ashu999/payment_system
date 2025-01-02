use crate::utils::response::{json_response, ApiResponse, MessageData};
use actix_web::{post, web, Responder};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct WebhookPayload {
    transaction_id: uuid::Uuid,
    status: String,
    amount: rust_decimal::Decimal,
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
    let mut conn = sqlx::postgres::PgListener::connect(env!("DATABASE_URL"))
        .await
        .expect("Failed to connect");

    conn.listen("transaction_insert")
        .await
        .expect("Failed to listen to notifications");

    loop {
        let notification = conn.recv().await.expect("Failed to receive notification");
        if let Ok(payload) = serde_json::from_str::<WebhookPayload>(&notification.payload()) {
            // Make request to webhook endpoint
            let client = awc::Client::default();
            let _ = client
                .post("http://localhost:8080/merchant/webhook") //this link will be dynamic
                .send_json(&payload)
                .await;
        }
    }
}
