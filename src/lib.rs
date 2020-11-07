use actix_web::{dev::Server, web::Data, HttpServer};
use actix_web::{get, middleware::Logger, App, HttpResponse};
use sqlx::PgPool;
use std::io;
use std::net::TcpListener;

pub mod configuration;

#[get("/health_check")]
async fn health_check() -> HttpResponse {
    HttpResponse::Ok().finish()
}

pub fn run(listener: TcpListener, db_pool: PgPool) -> io::Result<Server> {
    let db_pool = Data::new(db_pool);
    let server = HttpServer::new(move || {
        App::new()
            .app_data(db_pool.clone())
            .wrap(Logger::default())
            .service(health_check)
    })
    .listen(listener)?
    .run();
    Ok(server)
}
