# Payments Server

Technologies Used: Rust, Actix, PostgreSQL, Sqlx, JWT, Docker  

API Testing (Import collection): [Thunder Client](./docs/thunder-client-collection_payment_system.json) or [Postman](./docs/postman-collection-payment_system.json)
Also Checkout: [OpenAPI](./docs/openapi.yaml)

Setup

Local Setup:
```shell
docker-compose up --build db  <-- (run postgress in docker)
sqlx migrate run <-- (apply mirgation)

cargo run <-- (run payment_system locally)
```
