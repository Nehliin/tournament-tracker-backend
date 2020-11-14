FROM rust:1.47 AS planner
WORKDIR /app
RUN cargo install cargo-chef
COPY . . 
# Generate a list of cargo dependencies used
RUN cargo chef prepare --recipe-path recipe.json

FROM rust:1.47 AS cacher
WORKDIR /app
RUN cargo install cargo-chef
COPY --from=planner /app/recipe.json recipe.json
# Build all dependencies so they can be cached
RUN cargo chef cook --release --recipe-path recipe.json

FROM rust:1.47 AS builder
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
RUN apt-get update -y \
    && apt-get install -y --no-install-recommends openssl \
    # Clean up
    && apt-get autoremove -y \
    && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*

# Copy the compiled binary from the builder environment 
# to our runtime environment
COPY --from=builder /app/target/release/app app
COPY configuration configuration
# Set log level
ENV RUST_LOG sqlx=warn,info
ENV ENVIROMENT prod

ENTRYPOINT [ "./app"]