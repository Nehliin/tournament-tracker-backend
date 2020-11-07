use std::net::TcpListener;

use sqlx::PgPool;
use tokio::runtime::Runtime;
use tournament_tracker_backend::configuration::get_configuration;

async fn spawn_server() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind address");
    let port = listener.local_addr().unwrap().port();

    let configuration = get_configuration().expect("Failed to read configuration.");
    let connection_pool = PgPool::new(&configuration.database.connection_string())
        .await
        .expect("Failed to connect to database");

    let server = tournament_tracker_backend::run(listener, connection_pool)
        .expect("Failed to create server");
    let rt = Runtime::new().expect("Failed to start tokio runtime");
    // tokio, unlike smol detaches when task handle is droppped
    rt.block_on(async {
        let _ = tokio::spawn(server);
    });
    // return the server address
    format!("http://127.0.0.1:{}", port)
}

#[actix_rt::test]
async fn health_check_test() {
    let server_addr = spawn_server().await;

    let client = reqwest::Client::new();

    let response = client
        .get(&format!("{}/health_check", &server_addr))
        .send()
        .await
        .expect("Request failed");

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}
