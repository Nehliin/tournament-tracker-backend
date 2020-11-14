use actix_web::App;
use actix_web::{dev::Server, http, web::Data, HttpServer, ResponseError};
use endpoints::*;
use sqlx::PgPool;
use std::io;
use std::net::TcpListener;
use stores::{
    match_store::MatchStore, player_registration_store::PlayerRegistrationStore,
    player_store::PlayerStore, tournament_store::TournamentStore,
};
use thiserror::Error;
use tracing::{subscriber::set_global_default, Subscriber};
use tracing_actix_web::TracingLogger;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::{prelude::__tracing_subscriber_SubscriberExt, EnvFilter, Registry};

pub mod configuration;
pub mod endpoints;
pub mod stores;

/*
Actix will log these via the Debug trait and not the display string from the error attribute.
This means data in the error variants that's not part of the attribute string won't reach the
caller which means private info useful for debugging can be part of the variants.
*/
#[derive(Debug, Error)]
pub enum ServerError {
    #[error("Invalid start or end date")]
    InvalidDate,
    #[error("Invalid start time")]
    InvalidStartTime,
    #[error("Invalid rooster, two different players are needed")]
    InvalidRooster,
    #[error("Invalid player registration")]
    InvalidPlayerRegistration,
    #[error("Player already registered to match")]
    PlayerAlreadyReigstered,
    #[error("Internal Database error")]
    InternalDataBaseError(#[from] sqlx::Error),
}

impl ResponseError for ServerError {
    fn status_code(&self) -> http::StatusCode {
        match &self {
            ServerError::InvalidDate
            | ServerError::InvalidRooster
            | ServerError::InvalidStartTime
            | ServerError::InvalidPlayerRegistration
            | ServerError::PlayerAlreadyReigstered => http::StatusCode::BAD_REQUEST,
            ServerError::InternalDataBaseError(_) => http::StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

pub fn get_trace_subscriber(
    name: String,
    fallback_filter: String,
) -> impl Subscriber + Send + Sync {
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(fallback_filter));
    let formatting_layer = BunyanFormattingLayer::new(name, std::io::stdout);

    Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer)
}

pub fn init_subscriber(subscriber: impl Subscriber + Send + Sync) {
    LogTracer::init().expect("Failed to set logger");
    set_global_default(subscriber).expect("Failed to set subscriber");
}

pub fn run(listener: TcpListener, db_pool: PgPool) -> io::Result<Server> {
    let tournament_store = Data::new(TournamentStore {
        pool: db_pool.clone(),
    });
    let player_store = Data::new(PlayerStore {
        pool: db_pool.clone(),
    });
    let match_store = Data::new(MatchStore {
        pool: db_pool.clone(),
    });
    let player_registration_store = Data::new(PlayerRegistrationStore { pool: db_pool });

    let server = HttpServer::new(move || {
        App::new()
            .app_data(match_store.clone())
            .app_data(tournament_store.clone())
            .app_data(player_store.clone())
            .app_data(player_registration_store.clone())
            .wrap(TracingLogger)
            .service(insert_tournament)
            .service(get_tournaments)
            .service(health_check)
            .service(insert_player)
            .service(get_player)
            .service(register_player)
            .service(insert_match)
    })
    .listen(listener)?
    .run();
    Ok(server)
}
