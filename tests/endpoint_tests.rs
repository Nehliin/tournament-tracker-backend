use std::net::TcpListener;

use chrono::{Duration, Local};
use sqlx::{Connection, Executor};
use sqlx::{PgConnection, PgPool};
use tokio::runtime::Runtime;
use tournament_tracker_backend::{
    configuration::{get_configuration, DatabaseSettings},
    stores::tournament_store::Tournament,
};
use uuid::Uuid;

async fn spawn_server() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind address");
    let port = listener.local_addr().unwrap().port();

    let mut configuration = get_configuration().expect("Failed to read configuration.");

    // Randomise database name:
    configuration.database.database_name = Uuid::new_v4().to_string();

    let connection_pool = configure_database(&configuration.database).await;

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

pub async fn configure_database(config: &DatabaseSettings) -> PgPool {
    // Create database
    let mut connection = PgConnection::connect(&config.connection_string_without_db())
        .await
        .expect("Failed to connect to Postgres");
    connection
        .execute(&*format!(r#"CREATE DATABASE "{}";"#, config.database_name))
        .await
        .expect("Failed to create database.");

    // Migrate database
    let connection_pool = PgPool::connect(&config.connection_string())
        .await
        .expect("Failed to connect to Postgres.");
    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to migrate the database");

    connection_pool
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

#[actix_rt::test]
async fn insert_tournament_test() {
    let server_addr = spawn_server().await;

    let client = reqwest::Client::new();
    let start_date = Local::today().naive_local();

    let tournament = Tournament {
        id: 0, // doesn't matter
        name: "Södertälje open".into(),
        start_date,
        end_date: start_date + Duration::days(1),
    };

    let response = client
        .post(&format!("{}/tournaments", &server_addr))
        .json(&tournament)
        .send()
        .await
        .expect("Request failed");

    assert!(response.status().is_success());
}

#[actix_rt::test]
async fn get_tournament_test() {
    let server_addr = spawn_server().await;

    let client = reqwest::Client::new();
    let start_date = Local::today().naive_local();

    let tournament = Tournament {
        id: 0, // doesn't matter
        name: "Södertälje open".into(),
        start_date,
        end_date: start_date + Duration::days(1),
    };

    let response = client
        .post(&format!("{}/tournaments", &server_addr))
        .json(&tournament)
        .send()
        .await
        .expect("Request failed");

    assert!(response.status().is_success());

    // get tournament:

    let response = client
        .get(&format!("{}/tournaments", &server_addr))
        .send()
        .await
        .expect("Request failed");

    assert!(response.status().is_success());
    let tournament_list = response
        .json::<Vec<Tournament>>()
        .await
        .expect("Response body");
    assert_eq!(tournament_list.len(), 1);

    let tournament = Tournament {
        id: tournament_list[0].id,
        ..tournament
    };

    assert_eq!(tournament_list[0], tournament);
}
