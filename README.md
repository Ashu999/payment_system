# Payments Server

Technologies Used: Rust, Actix, PostgreSQL, Sqlx, JWT, Docker  

API Testing (Import collection): [Postman (free)](./docs/postman-collection-payment_system.json) or [Thunder Client (not free)](./docs/thunder-client-collection_payment_system.json)
Also Checkout: [OpenAPI spec.](./docs/openapi.yaml)

Dev Setup:
```shell
git clone https://github.com/Ashu999/payment_system.git
cd payment_system

docker-compose up --build db    # run postgress in docker

cargo install sqlx-cli          # install sqlx-cli
sqlx database create            # create database at DATABASE_URL
sqlx migrate run                # apply mirgation

cargo run                       # run app locally (OR with watch mode: `cargo watch -x run`)
```

Docker Setup:
```shell
git clone https://github.com/Ashu999/payment_system.git
cd payment_system

docker-compose up --build
```
