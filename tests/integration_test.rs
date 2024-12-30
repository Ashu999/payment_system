use std::env;

use actix_web::{test, web, App};
use payment_system::routes::{
    balance::{add_amount, get_balance},
    transactions::{get_transactions, send_transaction},
    user::{get_user, login, register},
};
use serde_json::json;
use sqlx::PgPool;
use testcontainers::Docker;
use testcontainers::{clients::Cli, images::postgres::Postgres};

#[actix_rt::test]
async fn test_complete_payment_flow() {
    // Set JWT secret for auth
    env::set_var("JWT_SECRET_KEY", "test_secret_key_123");
    // Setup test container
    let docker = Cli::default();
    let postgres_container = docker.run(Postgres::default());
    let db_url = format!(
        "postgres://postgres:postgres@localhost:{}/postgres",
        postgres_container.get_host_port(5432).unwrap()
    );

    // Setup database pool
    let pool = PgPool::connect(&db_url).await.unwrap();

    //migrate database
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    // Setup test app
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .service(register)
            .service(login)
            .service(get_user)
            .service(add_amount)
            .service(get_balance)
            .service(send_transaction)
            .service(get_transactions),
    )
    .await;

    // Test user1 registration
    let user1_register = json!({
        "email": "user1@test.com",
        "password": "password123"
    });
    let req = test::TestRequest::post()
        .uri("/user/register")
        .set_json(&user1_register)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    // Test user1 login
    let req = test::TestRequest::post()
        .uri("/user/login")
        .set_json(&user1_register)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    let user1_token = body["data"]["token"].as_str().unwrap().to_string();

    // Test get user1 info
    let req = test::TestRequest::get()
        .uri("/user")
        .insert_header(("Authorization", format!("Bearer {}", user1_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    // Test add amount for user1
    let amount_to_add = json!({
        "amount": "50"
    });
    let req = test::TestRequest::post()
        .uri("/balance/add")
        .insert_header(("Authorization", format!("Bearer {}", user1_token)))
        .set_json(&amount_to_add)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    // Test user2 registration
    let user2_register = json!({
        "email": "user2@test.com",
        "password": "password123"
    });
    let req = test::TestRequest::post()
        .uri("/user/register")
        .set_json(&user2_register)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    // Test user2 login
    let req = test::TestRequest::post()
        .uri("/user/login")
        .set_json(&user2_register)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    let user2_token = body["data"]["token"].as_str().unwrap().to_string();

    // Test send money from user1 to user2
    let send_money = json!({
        "amount": "10",
        "email": "user2@test.com"
    });
    let req = test::TestRequest::post()
        .uri("/transaction/send")
        .insert_header(("Authorization", format!("Bearer {}", user1_token)))
        .set_json(&send_money)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    // Test check user1 balance
    let req = test::TestRequest::get()
        .uri("/balance")
        .insert_header(("Authorization", format!("Bearer {}", user1_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["balance"].as_str().unwrap(), "40.00");

    // Test check user2 balance
    let req = test::TestRequest::get()
        .uri("/balance")
        .insert_header(("Authorization", format!("Bearer {}", user2_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["balance"].as_str().unwrap(), "10.00");

    // Test check user1 transactions
    let req = test::TestRequest::get()
        .uri("/transactions")
        .insert_header(("Authorization", format!("Bearer {}", user1_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    // Test check user2 transactions
    let req = test::TestRequest::get()
        .uri("/transactions")
        .insert_header(("Authorization", format!("Bearer {}", user2_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
}
