use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use routes::user::{get_user, login, register};
use sqlx::postgres::PgPoolOptions;
use std::env;

mod routes;
mod utils;

#[get("/")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Welcome to the payment system API!")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();

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

    println!("Starting server at http://0.0.0.0:8080");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .service(hello)
            .service(register)
            .service(login)
            .service(get_user)
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
