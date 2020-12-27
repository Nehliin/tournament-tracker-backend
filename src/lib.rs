use actix_web::{dev::Server, http, web, HttpServer, ResponseError};
use actix_web::{web::Data, App};
use actix_web_httpauth::{extractors::bearer::BearerAuth, middleware::HttpAuthentication};
use authentication::authenticate_request;
use endpoints::*;
use sqlx::PgPool;
use std::io;
use std::net::TcpListener;
use thiserror::Error;
use tracing::{subscriber::set_global_default, Subscriber};
use tracing_actix_web::TracingLogger;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::{fmt::MakeWriter, prelude::*, EnvFilter, Registry};

mod authentication;
pub mod configuration;
pub mod endpoints;
pub mod match_operations;
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
    #[error("Invalid winner, player not part of match")]
    InvalidWinner,
    #[error("Invalid result, the result string isn't a valid score")]
    InvalidResult,
    #[error("Match already completed")]
    MatchAlreadyCompleted,
    #[error("Can't finish match, it hasn't started yet")]
    MatchNotStarted,
    #[error("Player already registered to match")]
    PlayerAlreadyReigstered,
    #[error("Can't start match, player is missing")]
    PlayerMissing,
    #[error("Player can't be found")]
    PlayerNotFound,
    #[error("Match can't be found")]
    MatchNotFound,
    #[error("Match already started")]
    MatchAlreadyStarted,
    #[error("Invalid email")]
    InvalidEmail,
    #[error("Invalid password")]
    InvalidPassword,
    #[error("Invalid auth token")]
    InvalidToken,
    #[error("Login failed")]
    LoginFailed,
    #[error("Internal Database error")]
    InternalDataBaseError(#[from] sqlx::Error),
}

impl ResponseError for ServerError {
    fn status_code(&self) -> http::StatusCode {
        match &self {
            ServerError::InvalidDate
            | ServerError::PlayerMissing
            | ServerError::InvalidRooster
            | ServerError::InvalidStartTime
            | ServerError::InvalidPlayerRegistration
            | ServerError::InvalidWinner
            | ServerError::InvalidResult
            | ServerError::MatchAlreadyStarted
            | ServerError::InvalidPassword
            | ServerError::InvalidEmail
            | ServerError::PlayerAlreadyReigstered => http::StatusCode::BAD_REQUEST,
            ServerError::MatchNotFound | ServerError::PlayerNotFound => http::StatusCode::NOT_FOUND,
            ServerError::InternalDataBaseError(_) | ServerError::LoginFailed => {
                http::StatusCode::INTERNAL_SERVER_ERROR
            }
            ServerError::InvalidToken => http::StatusCode::UNAUTHORIZED,
            ServerError::MatchNotStarted | ServerError::MatchAlreadyCompleted => {
                http::StatusCode::CONFLICT
            }
        }
    }
}

pub fn get_trace_subscriber(
    name: String,
    fallback_filter: String,
    writer: impl MakeWriter + Sync + Send + 'static,
) -> impl Subscriber + Send + Sync {
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(fallback_filter));
    let formatting_layer = BunyanFormattingLayer::new(name, writer);

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
    let server = HttpServer::new(move || {
        let pool_clone = db_pool.clone();
        let auth = HttpAuthentication::bearer(move |req, credentials: BearerAuth| {
            let clone = pool_clone.clone();
            authenticate_request(clone, req, credentials)
        });
        App::new()
            .app_data(Data::new(db_pool.clone()))
            .wrap(TracingLogger)
            // authenticated scope
            .service(
                web::scope("/")
                    .wrap(auth)
                    .service(insert_tournament)
                    .service(insert_match)
                    .service(insert_player)
                    .service(register_player)
                    .service(add_court_to_tournament)
                    .service(finish_match_endpoint),
            )
            .service(get_tournaments)
            .service(health_check)
            .service(get_player)
            .service(get_tournament_matches)
    })
    .listen(listener)?
    .run();
    Ok(server)
}
