use std::{io, net::TcpListener};

use sqlx::PgPool;
use tournament_tracker_backend::{configuration::get_configuration, run};

#[actix_web::main]
async fn main() -> io::Result<()> {
    let config = get_configuration().expect("Failed to read configuration");

    let connection_pool = PgPool::connect(&config.database.connection_string())
        .await
        .expect("Failed to connect to database");

    let listener = TcpListener::bind(format!("127.0.0.1:{}", config.application_port))
        .expect("Failed to bind address");

    run(listener, connection_pool)?.await
}
