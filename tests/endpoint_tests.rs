use std::net::TcpListener;

use chrono::{Duration, Local};
use reqwest::StatusCode;
use sqlx::{Connection, Executor};
use sqlx::{PgConnection, PgPool};
use tokio::runtime::Runtime;
use tournament_tracker_backend::{
    configuration::{get_configuration, DatabaseSettings},
    endpoints::PlayerMatchRegistrationRequest,
    stores::match_store::Match,
    stores::{
        player_registration_store::PlayerMatchRegistration, player_store::Player,
        tournament_store::Tournament,
    },
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

#[actix_rt::test]
async fn insert_and_get_player_test() {
    let server_addr = spawn_server().await;

    let client = reqwest::Client::new();

    let player = Player {
        id: 3,
        name: "Göte svensson".into(),
    };

    let response = client
        .post(&format!("{}/players", &server_addr))
        .json(&player)
        .send()
        .await
        .expect("Request failed");

    assert!(response.status().is_success());

    let response = client
        .get(&format!("{}/players/3", &server_addr))
        .send()
        .await
        .expect("Request failed");

    assert!(response.status().is_success());
    assert_eq!(
        player,
        response.json::<Player>().await.expect("Response body")
    );
}

#[actix_rt::test]
async fn should_404_on_missing_player() {
    let server_addr = spawn_server().await;

    let client = reqwest::Client::new();

    let response = client
        .get(&format!("{}/players/3", &server_addr))
        .send()
        .await
        .expect("Request failed");

    assert_eq!(response.status(), StatusCode::from_u16(404).unwrap());
}

#[actix_rt::test]
async fn should_fail_invalid_player_registration() {
    let server_addr = spawn_server().await;

    let client = reqwest::Client::new();

    let player_registration_request = PlayerMatchRegistrationRequest {
        player_id: 0,
        registered_by: "Svante".to_string(),
    };

    let response = client
        .post(&format!("{}/matches/0/register/player", &server_addr))
        .json(&player_registration_request)
        .send()
        .await
        .expect("Request failed");

    assert_eq!(response.status(), StatusCode::from_u16(500).unwrap());
}

#[actix_rt::test]
async fn should_register_player() {
    let server_addr = spawn_server().await;

    let client = reqwest::Client::new();

    let start_date = Local::today().naive_local();

    let tournament = Tournament {
        id: 0, // doesn't matter
        name: "Södertälje open".into(),
        start_date,
        end_date: start_date + Duration::days(1),
    };
    // insert tournament
    let response = client
        .post(&format!("{}/tournaments", &server_addr))
        .json(&tournament)
        .send()
        .await
        .expect("Request failed");

    assert!(response.status().is_success());
    let tournament_id = response.text().await.unwrap();

    let player = Player {
        id: 0,
        name: "Göte svensson".into(),
    };

    // insert player 1
    let response = client
        .post(&format!("{}/players", &server_addr))
        .json(&player)
        .send()
        .await
        .expect("Request failed");

    assert!(response.status().is_success());

    let player = Player {
        id: 1,
        name: "Sture svensson".into(),
    };

    // insert player 2
    let response = client
        .post(&format!("{}/players", &server_addr))
        .json(&player)
        .send()
        .await
        .expect("Request failed");

    assert!(response.status().is_success());

    // insert match
    let match_data = Match {
        id: 0, // not important
        player_one: 0,
        player_two: 1,
        tournament_id: tournament_id.parse::<i32>().unwrap(),
        class: "p96".to_string(),
        start_time: Local::now().naive_local() + Duration::hours(2),
    };

    let response = client
        .post(&format!("{}/matches", &server_addr))
        .json(&match_data)
        .send()
        .await
        .expect("Request failed");

    assert!(response.status().is_success());
    let match_id = response.text().await.unwrap();

    let player_registration_request = PlayerMatchRegistrationRequest {
        player_id: 1,
        registered_by: "Svante".to_string(),
    };

    // register player 2
    let response = client
        .post(&format!(
            "{}/matches/{}/register/player",
            &server_addr, match_id
        ))
        .json(&player_registration_request)
        .send()
        .await
        .expect("Request failed");

    assert!(response.status().is_success());

    let actual = response.json::<PlayerMatchRegistration>().await.unwrap();

    assert_eq!(1, actual.player_id);
    assert_eq!(match_id.parse::<i64>().unwrap(), actual.match_id);
    assert_eq!("Svante".to_string(), actual.registerd_by);
}

// TODO: add more match insertion tests
