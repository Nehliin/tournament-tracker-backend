#![allow(dead_code)]

use std::net::TcpListener;

use reqwest::{Client, RequestBuilder, Response};
use sqlx::{Connection, Executor};
use sqlx::{PgConnection, PgPool};
use tokio::runtime::Runtime;
use tournament_tracker_backend::{
    configuration::{get_configuration, DatabaseSettings},
    endpoints::{CourtForm, PlayerMatchRegistrationPayload},
    get_trace_subscriber, init_subscriber,
    stores::match_store::Match,
    stores::{player_store::Player, tournament_store::Tournament},
};
use tournament_tracker_backend::{endpoints::CredentialsPayload, stores::match_store::MatchResult};
use uuid::Uuid;

pub struct UnauthenticatedClient {
    pub client: Client,
    pub server_addr: String,
}

pub struct AuthenticatedClient {
    pub unauthenticated_client: UnauthenticatedClient,
    pub token: String,
}

pub fn insert_tournament(
    client: &Client,
    server_addr: &str,
    tournament: &Tournament,
) -> RequestBuilder {
    client
        .post(&format!("{}/authenticated/tournaments", server_addr))
        .json(&tournament)
}

pub fn get_tournaments(client: &Client, server_addr: &str) -> RequestBuilder {
    client.get(&format!("{}/tournaments", server_addr))
}

pub fn add_court_to_tournament(
    client: &Client,
    server_addr: &str,
    tournament_id: i32,
    court_name: String,
) -> RequestBuilder {
    client
        .post(&format!(
            "{}/authenticated/tournaments/{}/courts",
            server_addr, tournament_id
        ))
        .form(&CourtForm { name: court_name })
}

pub fn get_tournaments_matches(
    client: &Client,
    server_addr: &str,
    tournament_id: i32,
) -> RequestBuilder {
    client.get(&format!(
        "{}/tournaments/{}/matches",
        server_addr, tournament_id,
    ))
}

pub fn insert_player(client: &Client, server_addr: &str, player: &Player) -> RequestBuilder {
    client
        .post(&format!("{}/authenticated/players", server_addr))
        .json(&player)
}

pub fn get_player(client: &Client, server_addr: &str, player_id: i64) -> RequestBuilder {
    client.get(&format!("{}/players/{}", server_addr, player_id))
}

pub fn insert_match(client: &Client, server_addr: &str, match_data: &Match) -> RequestBuilder {
    client
        .post(&format!("{}/authenticated/matches", server_addr))
        .json(&match_data)
}

pub fn finish_match(
    client: &Client,
    server_addr: &str,
    match_id: i64,
    match_result: &MatchResult,
) -> RequestBuilder {
    client
        .post(&format!(
            "{}/authenticated/matches/{}/finish",
            server_addr, match_id
        ))
        .json(&match_result)
}

pub fn register_player(
    client: &Client,
    server_addr: &str,
    match_id: i64,
    player_registration_req: &PlayerMatchRegistrationPayload,
) -> RequestBuilder {
    client
        .post(&format!(
            "{}/authenticated/matches/{}/register/player",
            server_addr, match_id
        ))
        .json(&player_registration_req)
}

impl UnauthenticatedClient {
    pub async fn insert_tournament(&self, tournament: &Tournament) -> Response {
        insert_tournament(&self.client, &self.server_addr, tournament)
            .send()
            .await
            .expect("Request failed")
    }

    pub async fn get_tournaments(&self) -> Response {
        get_tournaments(&self.client, &self.server_addr)
            .send()
            .await
            .expect("Request failed")
    }

    pub async fn add_court_to_tournament(
        &self,
        tournament_id: i32,
        court_name: String,
    ) -> Response {
        add_court_to_tournament(&self.client, &self.server_addr, tournament_id, court_name)
            .send()
            .await
            .expect("Request failed")
    }

    pub async fn get_tournaments_matches(&self, tournament_id: i32) -> Response {
        get_tournaments_matches(&self.client, &self.server_addr, tournament_id)
            .send()
            .await
            .expect("Request failed")
    }

    pub async fn insert_player(&self, player: &Player) -> Response {
        insert_player(&self.client, &self.server_addr, player)
            .send()
            .await
            .expect("Request failed")
    }

    pub async fn get_player(&self, player_id: i64) -> Response {
        get_player(&self.client, &self.server_addr, player_id)
            .send()
            .await
            .expect("Request failed")
    }

    pub async fn insert_match(&self, match_data: &Match) -> Response {
        insert_match(&self.client, &self.server_addr, match_data)
            .send()
            .await
            .expect("Request failed")
    }

    pub async fn finish_match(&self, match_id: i64, match_result: &MatchResult) -> Response {
        finish_match(&self.client, &self.server_addr, match_id, match_result)
            .send()
            .await
            .expect("Request failed")
    }

    pub async fn register_player(
        &self,
        match_id: i64,
        player_registration_req: &PlayerMatchRegistrationPayload,
    ) -> Response {
        register_player(
            &self.client,
            &self.server_addr,
            match_id,
            player_registration_req,
        )
        .send()
        .await
        .expect("Request failed")
    }

    pub async fn create_user(&self, credentials: &CredentialsPayload) -> Response {
        self.client
            .post(&format!("{}/user", &self.server_addr))
            .json(&credentials)
            .send()
            .await
            .expect("Request failed")
    }

    pub async fn login(&self, credentials: &CredentialsPayload) -> Response {
        self.client
            .post(&format!("{}/login", &self.server_addr))
            .json(&credentials)
            .send()
            .await
            .expect("Request failed")
    }

    pub async fn new_authenticated_client(self) -> AuthenticatedClient {
        let dummy_credentials = CredentialsPayload {
            email: "dummy@test.se".to_string(),
            password: "some-secure-password".to_string(),
        };
        self.create_user(&dummy_credentials).await;
        let token = self.login(&dummy_credentials).await.text().await.unwrap();
        AuthenticatedClient {
            unauthenticated_client: self,
            token,
        }
    }
}

const AUTH_HEADER: &str = "Authorization";

impl AuthenticatedClient {
    fn auth_header_value(&self) -> String {
        format!("Bearer {}", self.token)
    }

    pub async fn insert_tournament(&self, tournament: &Tournament) -> Response {
        insert_tournament(
            &self.unauthenticated_client.client,
            &self.unauthenticated_client.server_addr,
            tournament,
        )
        .header(AUTH_HEADER, self.auth_header_value())
        .send()
        .await
        .expect("Request failed")
    }

    pub async fn get_tournaments(&self) -> Response {
        get_tournaments(
            &self.unauthenticated_client.client,
            &self.unauthenticated_client.server_addr,
        )
        .header(AUTH_HEADER, self.auth_header_value())
        .send()
        .await
        .expect("Request failed")
    }

    pub async fn add_court_to_tournament(
        &self,
        tournament_id: i32,
        court_name: String,
    ) -> Response {
        add_court_to_tournament(
            &self.unauthenticated_client.client,
            &self.unauthenticated_client.server_addr,
            tournament_id,
            court_name,
        )
        .header(AUTH_HEADER, self.auth_header_value())
        .send()
        .await
        .expect("Request failed")
    }

    pub async fn get_tournaments_matches(&self, tournament_id: i32) -> Response {
        get_tournaments_matches(
            &self.unauthenticated_client.client,
            &self.unauthenticated_client.server_addr,
            tournament_id,
        )
        .header(AUTH_HEADER, self.auth_header_value())
        .send()
        .await
        .expect("Request failed")
    }

    pub async fn insert_player(&self, player: &Player) -> Response {
        insert_player(
            &self.unauthenticated_client.client,
            &self.unauthenticated_client.server_addr,
            player,
        )
        .header(AUTH_HEADER, self.auth_header_value())
        .send()
        .await
        .expect("Request failed")
    }

    pub async fn get_player(&self, player_id: i64) -> Response {
        get_player(
            &self.unauthenticated_client.client,
            &self.unauthenticated_client.server_addr,
            player_id,
        )
        .header(AUTH_HEADER, self.auth_header_value())
        .send()
        .await
        .expect("Request failed")
    }

    pub async fn insert_match(&self, match_data: &Match) -> Response {
        insert_match(
            &self.unauthenticated_client.client,
            &self.unauthenticated_client.server_addr,
            match_data,
        )
        .header(AUTH_HEADER, self.auth_header_value())
        .send()
        .await
        .expect("Request failed")
    }

    pub async fn finish_match(&self, match_id: i64, match_result: &MatchResult) -> Response {
        finish_match(
            &self.unauthenticated_client.client,
            &self.unauthenticated_client.server_addr,
            match_id,
            match_result,
        )
        .header(AUTH_HEADER, self.auth_header_value())
        .send()
        .await
        .expect("Request failed")
    }

    pub async fn register_player(
        &self,
        match_id: i64,
        player_registration_req: &PlayerMatchRegistrationPayload,
    ) -> Response {
        register_player(
            &self.unauthenticated_client.client,
            &self.unauthenticated_client.server_addr,
            match_id,
            player_registration_req,
        )
        .header(AUTH_HEADER, self.auth_header_value())
        .send()
        .await
        .expect("Request failed")
    }
}

lazy_static::lazy_static! {
    static ref TRACING: () = {
        let subscriber = get_trace_subscriber("Test server".into(), "debug".into(), || std::io::stdout());
        init_subscriber(subscriber);
    };
}

pub async fn spawn_server_and_authenticate() -> AuthenticatedClient {
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

    UnauthenticatedClient {
        server_addr: format!("http://127.0.0.1:{}", port),
        client: reqwest::Client::new(),
    }
    .new_authenticated_client()
    .await
}

pub async fn configure_database(config: &DatabaseSettings) -> PgPool {
    // Create database
    let mut connection = PgConnection::connect_with(&config.without_db())
        .await
        .expect("Failed to connect to Postgres");
    connection
        .execute(&*format!(r#"CREATE DATABASE "{}";"#, config.database_name))
        .await
        .expect("Failed to create database.");

    // Migrate database
    let connection_pool = PgPool::connect_with(config.with_db())
        .await
        .expect("Failed to connect to Postgres.");
    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to migrate the database");

    connection_pool
}
