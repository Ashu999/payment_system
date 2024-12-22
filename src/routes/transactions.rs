use crate::utils::auth;
use actix_web::{get, post, web, HttpResponse, Responder};
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
        Err(msg) => return HttpResponse::Unauthorized().json(msg),
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
        Ok(transactions) => HttpResponse::Ok().json(transactions),
        Err(_) => HttpResponse::InternalServerError().json("Failed to fetch transactions"),
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
        Err(msg) => return HttpResponse::Unauthorized().json(msg),
    };

    // Validate amount is positive
    if send_request.amount <= Decimal::new(0, 0) {
        return HttpResponse::BadRequest().json(SendTransactionResponse {
            amount: send_request.amount,
            receiver_email: send_request.email.clone(),
            balance: Decimal::new(0, 0),
            message: "Amount must be positive".to_string(),
        });
    }

    // Start a transaction
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(_) => return HttpResponse::InternalServerError().json("Failed to start transaction"),
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
        Ok(None) => return HttpResponse::NotFound().json("Sender not found"),
        Err(_) => return HttpResponse::InternalServerError().json("Database error"),
    };

    // Check if sender has sufficient balance
    if sender.balance < send_request.amount {
        return HttpResponse::BadRequest().json(SendTransactionResponse {
            amount: send_request.amount,
            receiver_email: send_request.email.clone(),
            balance: sender.balance,
            message: "Insufficient balance".to_string(),
        });
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
            return HttpResponse::NotFound().json(SendTransactionResponse {
                amount: send_request.amount,
                receiver_email: send_request.email.clone(),
                balance: sender.balance,
                message: "Receiver not found".to_string(),
            })
        }
        Err(_) => return HttpResponse::InternalServerError().json("Database error"),
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
            return HttpResponse::InternalServerError().json("Failed to update sender balance")
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
        return HttpResponse::InternalServerError().json("Failed to update receiver balance");
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
        return HttpResponse::InternalServerError().json("Failed to create transaction records");
    }

    // Commit the transaction
    if let Err(_) = tx.commit().await {
        return HttpResponse::InternalServerError().json("Failed to commit transaction");
    }

    HttpResponse::Ok().json(SendTransactionResponse {
        amount: send_request.amount,
        receiver_email: send_request.email.clone(),
        balance: updated_sender.balance,
        message: "Transaction successful".to_string(),
    })
}
