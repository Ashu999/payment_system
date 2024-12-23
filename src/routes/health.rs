use crate::utils::response::{json_response, ApiResponse, MessageData};
use actix_web::{get, Responder};

#[get("/")]
async fn health() -> impl Responder {
    json_response(ApiResponse::success(MessageData {
        message: "Welcome to the payment system API!".to_string(),
    }))
}
