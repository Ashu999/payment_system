use crate::utils::{
    auth,
    response::{json_response, ApiResponse, MessageData},
};
use actix_web::{get, post, web, Responder};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize)]
pub struct TransactionResponse {
    id: Uuid,
    transaction_type: String,
    amount: Decimal,
    status: String,
}

#[derive(Deserialize)]
pub struct SendTransactionRequest {
    amount: Decimal,
    email: String,
}

#[derive(Serialize, Deserialize)]
pub struct SendTransactionResponse {
    amount: Decimal,
    receiver_email: String,
    balance: Decimal,
    message: String,
}

#[get("/transactions")]
pub async fn get_transactions(
    req: actix_web::HttpRequest,
    pool: web::Data<sqlx::PgPool>,
) -> impl Responder {
    // Verify token and get claims
    let claims = match auth::verify_request_token(&req) {
        Ok(claims) => claims,
        Err(msg) => return json_response(ApiResponse::<MessageData>::error(401, msg.to_string())),
    };

    // Fetch transactions for the user
    let transactions = sqlx::query_as!(
        TransactionResponse,
        r#"
        SELECT 
            id,
            transaction_type::text as "transaction_type!",
            amount,
            status::text as "status!"
        FROM transactions 
        WHERE user_id = $1 
        ORDER BY created_at DESC
        "#,
        claims.sub
    )
    .fetch_all(&**pool)
    .await;

    match transactions {
        Ok(transactions) => json_response(ApiResponse::success(transactions)),
        Err(_) => json_response(ApiResponse::<MessageData>::error(
            500,
            "Failed to fetch transactions".to_string(),
        )),
    }
}

#[post("/transaction/send")]
pub async fn send_transaction(
    req: actix_web::HttpRequest,
    send_request: web::Json<SendTransactionRequest>,
    pool: web::Data<sqlx::PgPool>,
) -> impl Responder {
    // Verify token and get claims
    let claims = match auth::verify_request_token(&req) {
        Ok(claims) => claims,
        Err(msg) => return json_response(ApiResponse::<MessageData>::error(401, msg.to_string())),
    };

    // Validate amount is positive
    if send_request.amount <= Decimal::new(0, 0) {
        return json_response(ApiResponse::<MessageData>::error(
            400,
            "Amount must be positive".to_string(),
        ));
    }

    // Start a transaction
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(_) => {
            return json_response(ApiResponse::<MessageData>::error(
                500,
                "Failed to start transaction".to_string(),
            ))
        }
    };

    // Get sender's current balance
    let sender = match sqlx::query!(
        "SELECT balance FROM users WHERE id = $1 FOR UPDATE",
        claims.sub
    )
    .fetch_optional(&mut *tx)
    .await
    {
        Ok(Some(user)) => user,
        Ok(None) => {
            return json_response(ApiResponse::<MessageData>::error(
                404,
                "Sender not found".to_string(),
            ))
        }
        Err(_) => {
            return json_response(ApiResponse::<MessageData>::error(
                500,
                "Database error".to_string(),
            ))
        }
    };

    // Check if sender has sufficient balance
    if sender.balance < send_request.amount {
        return json_response(ApiResponse::<MessageData>::error(
            400,
            "Insufficient balance".to_string(),
        ));
    }

    // Get receiver's ID and verify they exist
    let receiver = match sqlx::query!(
        "SELECT id FROM users WHERE email = $1 FOR UPDATE",
        send_request.email
    )
    .fetch_optional(&mut *tx)
    .await
    {
        Ok(Some(user)) => user,
        Ok(None) => {
            return json_response(ApiResponse::<MessageData>::error(
                404,
                "Receiver not found".to_string(),
            ))
        }
        Err(_) => {
            return json_response(ApiResponse::<MessageData>::error(
                500,
                "Database error".to_string(),
            ))
        }
    };

    // Update sender's balance
    let updated_sender = match sqlx::query!(
        "UPDATE users SET balance = balance - $1 WHERE id = $2 RETURNING balance",
        send_request.amount,
        claims.sub
    )
    .fetch_one(&mut *tx)
    .await
    {
        Ok(user) => user,
        Err(_) => {
            return json_response(ApiResponse::<MessageData>::error(
                500,
                "Failed to update sender balance".to_string(),
            ))
        }
    };

    // Update receiver's balance
    if let Err(_) = sqlx::query!(
        "UPDATE users SET balance = balance + $1 WHERE id = $2",
        send_request.amount,
        receiver.id
    )
    .execute(&mut *tx)
    .await
    {
        return json_response(ApiResponse::<MessageData>::error(
            500,
            "Failed to update receiver balance".to_string(),
        ));
    }

    // Create transaction records for both sender and receiver
    let sender_transaction_id = Uuid::new_v4();
    let receiver_transaction_id = Uuid::new_v4();
    if let Err(_) = sqlx::query!(
        r#"
        INSERT INTO transactions (id, user_id, transaction_type, amount, status)
        VALUES ($1, $2, 'SENT', $3, 'SUCCESS'), ($4, $5, 'RECEIVED', $3, 'SUCCESS')
        "#,
        sender_transaction_id,
        claims.sub,
        send_request.amount,
        receiver_transaction_id,
        receiver.id
    )
    .execute(&mut *tx)
    .await
    {
        return json_response(ApiResponse::<MessageData>::error(
            500,
            "Failed to create transaction records".to_string(),
        ));
    }

    // Commit the transaction
    if let Err(_) = tx.commit().await {
        return json_response(ApiResponse::<MessageData>::error(
            500,
            "Failed to commit transaction".to_string(),
        ));
    }

    json_response(ApiResponse::success(SendTransactionResponse {
        amount: send_request.amount,
        receiver_email: send_request.email.clone(),
        balance: updated_sender.balance,
        message: "Transaction successful".to_string(),
    }))
}
