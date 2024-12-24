use actix_web::{test, web, App};
use payment_system::routes::user::register;
use serde_json::json;
use sqlx::{Executor, PgPool};
use testcontainers::Docker;
use testcontainers::{clients::Cli, images::postgres::Postgres};

#[actix_rt::test]
async fn test_user_register() {
    // Set up testcontainers
    let docker = Cli::default();
    let postgres_container = docker.run(Postgres::default());
    let db_url = format!(
        "postgres://postgres:postgres@localhost:{}/postgres",
        postgres_container.get_host_port(5432).unwrap()
    );

    // Set up database connection pool
    let pool = PgPool::connect(&db_url).await.unwrap();

    // Create the users table
    pool.execute(
        "CREATE TABLE users (
            id UUID PRIMARY KEY,
            email TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL,
            balance DECIMAL NOT NULL
        )",
    )
    .await
    .unwrap();

    // Set up Actix web app
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .service(register),
    )
    .await;
    // Create test payload
    let payload = json!({
        "email": "test@example.com",
        "password": "password123"
    });

    // Send POST request
    let req = test::TestRequest::post()
        .uri("/user/register")
        .set_json(&payload)
        .to_request();
    let resp = test::call_service(&app, req).await;

    // Assert response
    assert_eq!(resp.status(), 200);
    let body = test::read_body(resp).await;
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    let body_json: serde_json::Value = serde_json::from_str(&body_str).unwrap();
    let expected_json: serde_json::Value = json!({
        "status": "success",
        "status_code": 200,
        "data": {
            "message": "User created successfully"
        },
        "error": null
    });
    assert_eq!(body_json, expected_json);
}
