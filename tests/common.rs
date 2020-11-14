use std::net::TcpListener;

use reqwest::{Client, Response};
use sqlx::{Connection, Executor};
use sqlx::{PgConnection, PgPool};
use tokio::runtime::Runtime;
use tournament_tracker_backend::{
    configuration::{get_configuration, DatabaseSettings},
    endpoints::PlayerMatchRegistrationRequest,
    get_trace_subscriber, init_subscriber,
    stores::match_store::Match,
    stores::{player_store::Player, tournament_store::Tournament},
};
use uuid::Uuid;

pub struct TournamentTrackerClient {
    pub client: Client,
    pub server_addr: String,
}

impl TournamentTrackerClient {
    pub async fn insert_tournament(&self, tournament: &Tournament) -> Response {
        self.client
            .post(&format!("{}/tournaments", &self.server_addr))
            .json(&tournament)
            .send()
            .await
            .expect("Request failed")
    }

    pub async fn get_tournaments(&self) -> Response {
        self.client
            .get(&format!("{}/tournaments", &self.server_addr))
            .send()
            .await
            .expect("Request failed")
    }

    pub async fn insert_player(&self, player: &Player) -> Response {
        self.client
            .post(&format!("{}/players", &self.server_addr))
            .json(&player)
            .send()
            .await
            .expect("Request failed")
    }

    pub async fn get_player(&self, player_id: i64) -> Response {
        self.client
            .get(&format!("{}/players/{}", &self.server_addr, player_id))
            .send()
            .await
            .expect("Request failed")
    }

    pub async fn insert_match(&self, match_data: &Match) -> Response {
        self.client
            .post(&format!("{}/matches", &self.server_addr))
            .json(&match_data)
            .send()
            .await
            .expect("Request failed")
    }

    pub async fn register_player(
        &self,
        match_id: i64,
        player_registration_req: &PlayerMatchRegistrationRequest,
    ) -> Response {
        self.client
            .post(&format!(
                "{}/matches/{}/register/player",
                &self.server_addr, match_id
            ))
            .json(&player_registration_req)
            .send()
            .await
            .expect("Request failed")
    }
}

lazy_static::lazy_static! {
    static ref TRACING: () = {
        let subscriber = get_trace_subscriber("Test server".into(), "debug".into());
        init_subscriber(subscriber);
    };
}

pub async fn spawn_server() -> TournamentTrackerClient {
    lazy_static::initialize(&TRACING);

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

    TournamentTrackerClient {
        server_addr: format!("http://127.0.0.1:{}", port),
        client: reqwest::Client::new(),
    }
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
