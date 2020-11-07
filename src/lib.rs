use actix_web::{dev::Server, web::Data, HttpServer};
use actix_web::{middleware::Logger, App};
use endpoints::*;
use sqlx::PgPool;
use std::io;
use std::net::TcpListener;
use stores::tournament_store::TournamentStore;

pub mod configuration;
pub mod endpoints;
pub mod stores;

pub fn run(listener: TcpListener, db_pool: PgPool) -> io::Result<Server> {
    let tournament_store = Data::new(TournamentStore { pool: db_pool });
    let server = HttpServer::new(move || {
        App::new()
            .app_data(tournament_store.clone())
            .wrap(Logger::default())
            .service(insert_tournament)
            .service(get_tournaments)
            .service(health_check)
    })
    .listen(listener)?
    .run();
    Ok(server)
}
