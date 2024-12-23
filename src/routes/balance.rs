use crate::utils::{
    auth,
    response::{json_response, ApiResponse, MessageData},
};
use actix_web::{get, post, web, Responder};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::types::Uuid;

#[derive(Deserialize)]
pub struct AddBalanceRequest {
    amount: Decimal,
}

#[derive(Serialize, Deserialize)]
pub struct BalanceResponse {
    balance: Decimal,
}

#[get("/balance")]
pub async fn get_balance(
    req: actix_web::HttpRequest,
    pool: web::Data<sqlx::PgPool>,
) -> impl Responder {
    // Verify token and get claims
    let claims = match auth::verify_request_token(&req) {
        Ok(claims) => claims,
        Err(msg) => return json_response(ApiResponse::<MessageData>::error(401, msg.to_string())),
    };

    let user = sqlx::query!("SELECT balance FROM users WHERE id = $1", claims.sub)
        .fetch_optional(&**pool)
        .await;

    match user {
        Ok(Some(user)) => json_response(ApiResponse::success(BalanceResponse {
            balance: user.balance,
        })),
        _ => json_response(ApiResponse::<MessageData>::error(
            404,
            "User not found".to_string(),
        )),
    }
}

#[post("/balance/add")]
pub async fn add_amount(
    req: actix_web::HttpRequest,
    add_request: web::Json<AddBalanceRequest>,
    pool: web::Data<sqlx::PgPool>,
) -> impl Responder {
    // Verify token and get claims
    let claims = match auth::verify_request_token(&req) {
        Ok(claims) => claims,
        Err(msg) => return json_response(ApiResponse::<MessageData>::error(401, msg.to_string())),
    };

    // Validate amount is positive
    if add_request.amount <= Decimal::new(0, 0) {
        return json_response(ApiResponse::<MessageData>::error(
            400,
            "Amount must be positive".to_string(),
        ));
    }

    // Update balance and create transaction record in a transaction
    let result = sqlx::query!(
        r#"
        WITH updated_user AS (
            UPDATE users 
            SET balance = balance + $1 
            WHERE id = $2 
            RETURNING balance
        )
        INSERT INTO transactions (id, user_id, transaction_type, amount, status)
        VALUES ($3, $2, 'RECEIVED', $1, 'SUCCESS')
        RETURNING (SELECT balance FROM updated_user)
        "#,
        add_request.amount,
        claims.sub,
        Uuid::new_v4(),
    )
    .fetch_one(&**pool)
    .await;

    match result {
        Ok(record) => json_response(ApiResponse::success(BalanceResponse {
            balance: record.balance.unwrap_or_default(),
        })),
        Err(_) => json_response(ApiResponse::<MessageData>::error(
            500,
            "Failed to update balance".to_string(),
        )),
    }
}
