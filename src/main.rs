use sqlx::postgres::PgPoolOptions;
use std::{io, net::TcpListener};
use tournament_tracker_backend::{
    configuration::get_configuration, get_trace_subscriber, init_subscriber, run,
};

#[actix_web::main]
async fn main() -> io::Result<()> {
    let subscriber = get_trace_subscriber("tournament-tracker".into(), "info".into());
    init_subscriber(subscriber);

    let config = get_configuration().expect("Failed to read configuration");

    let connection_pool = PgPoolOptions::new()
        .connect_timeout(std::time::Duration::from_secs(5))
        .connect_with(config.database.with_db())
        .await
        .expect("Failed to connect to database");

    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to migrate the database");

    let listener = TcpListener::bind(format!(
        "{}:{}",
        config.application.host, config.application.port
    ))
    .expect("Failed to bind address");
    run(listener, connection_pool)?.await
}
