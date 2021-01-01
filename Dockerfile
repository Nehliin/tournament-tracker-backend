FROM rust:1.49 AS planner
WORKDIR /app
RUN cargo install cargo-chef
COPY . . 
# Generate a list of cargo dependencies used
RUN cargo chef prepare --recipe-path recipe.json

FROM rust:1.49 AS cacher
WORKDIR /app
RUN cargo install cargo-chef
COPY --from=planner /app/recipe.json recipe.json
# Build all dependencies so they can be cached
RUN cargo chef cook --release --recipe-path recipe.json

FROM rust:1.49 AS builder
WORKDIR /app
# Copy over cached dependencies
COPY --from=cacher /app/target target
COPY --from=cacher /usr/local/cargo /usr/local/cargo
COPY . .
# Uses the sqlx-data.json to do compile time checking of queries
ENV SQLX_OFFLINE true
RUN cargo build --release --bin app 

# Runtime stage
FROM debian:buster-slim AS runtime
WORKDIR /app

# Copy the compiled binary from the builder environment 
# to our runtime environment
COPY --from=builder /app/target/release/app app
COPY configuration configuration
# Set log level
ENV RUST_LOG sqlx=warn,info
ENV ENVIROMENT prod

# DB config (done via qovery cli):
#ENV APP_DATABASE__USERNAME ${QOVERY_DATABASE_TT_PSQL_USERNAME}
#ENV APP_DATABASE__PASSWORD ${QOVERY_DATABASE_TT_PSQL_PASSWORD}
#ENV APP_DATABASE__HOST ${QOVERY_DATABASE_TT_PSQL_HOST}
#ENV APP_DATABASE__PORT ${QOVERY_DATABASE_TT_PSQL_PORT}
#ENV APP_DATABASE__DATABASE_NAME ${QOVERY_DATABASE_TT_PSQL_NAME}

# TODO: make it match config?
EXPOSE 8080

ENTRYPOINT [ "./app"]