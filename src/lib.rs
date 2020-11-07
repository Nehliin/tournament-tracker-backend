use std::net::TcpListener;
use actix_web::{HttpServer, dev::Server};
use actix_web::{middleware::Logger, get, App, HttpResponse};
use std::io;

#[get("/health_check")]
async fn health_check() -> HttpResponse {
    HttpResponse::Ok().finish()
}

pub fn run(listener: TcpListener) -> io::Result<Server> {
    let server = HttpServer::new(|| App::new().wrap(Logger::default()).service(health_check))
        .listen(listener)?
        .run();
    Ok(server)
}
