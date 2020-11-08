use actix_web::{dev::Server, http, web::Data, HttpServer, ResponseError};
use actix_web::{middleware::Logger, App};
use endpoints::*;
use sqlx::PgPool;
use std::io;
use std::net::TcpListener;
use stores::{player_store::PlayerStore, tournament_store::TournamentStore};
use thiserror::Error;

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
    #[error("Internal Database error")]
    InternalDataBaseError(#[from] sqlx::Error),
}

impl ResponseError for ServerError {
    fn status_code(&self) -> http::StatusCode {
        match &self {
            ServerError::InvalidDate => http::StatusCode::BAD_REQUEST,
            // Todo: log the actual error
            ServerError::InternalDataBaseError(_) => http::StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

pub fn run(listener: TcpListener, db_pool: PgPool) -> io::Result<Server> {
    let tournament_store = Data::new(TournamentStore {
        pool: db_pool.clone(),
    });
    let player_store = Data::new(PlayerStore { pool: db_pool });

    let server = HttpServer::new(move || {
        App::new()
            .app_data(tournament_store.clone())
            .app_data(player_store.clone())
            .wrap(Logger::default())
            .service(insert_tournament)
            .service(get_tournaments)
            .service(health_check)
            .service(insert_player)
            .service(get_player)
    })
    .listen(listener)?
    .run();
    Ok(server)
}
