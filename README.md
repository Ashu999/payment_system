# Payments Server

Technologies Used: Rust, Actix, PostgreSQL, Sqlx, JWT, Docker, Kafka

## Dev Setup:
```shell
git clone https://github.com/Ashu999/payment_system.git
cd payment_system

docker-compose up --build db    # run postgress in docker

cargo test                      # run tests
cargo install sqlx-cli          # install sqlx-cli
sqlx database create            # create database at DATABASE_URL
sqlx migrate run                # apply mirgation

cargo run                       # run app locally (OR with watch mode: `cargo watch -x run`)
```

## Docker Setup:
```shell
git clone https://github.com/Ashu999/payment_system.git
cd payment_system

docker-compose up --build
```

## API Testing:
Import one of the collections (also contains input examples):
[Postman Collection (free)](./docs/postman-collection-payment_system.json) or [Thunder Client (not free)](./docs/thunder-client-collection_payment_system.json)

Also Checkout: [API Documentation](./docs/openapi.yaml)

## This Project includes:
- User APIs
- Transaction APIs
- Balance APIs
- Auth using JWT
- Rate Limiting
- Data validation
- Error handling
- Logging
- Health check
- Dockerization
- async/await
- security middleware
- unit, integration testing 
- CI/CD
- Postgress notification
- Webhook implementation