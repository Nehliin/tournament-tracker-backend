use std::{io, net::TcpListener};

use sqlx::postgres::PgPoolOptions;
use tournament_tracker_backend::{configuration::get_configuration, run};

#[actix_web::main]
async fn main() -> io::Result<()> {
    let config = get_configuration().expect("Failed to read configuration");

    let connection_pool = PgPoolOptions::new()
        .connect_timeout(std::time::Duration::from_secs(5))
        .connect(&config.database.connection_string())
        .await
        .expect("Failed to connect to database");
    println!("config: {:?}", config);
    let listener = TcpListener::bind(format!("{}:{}", config.application.host, config.application.port))
        .expect("Failed to bind address");
    run(listener, connection_pool)?.await
}
