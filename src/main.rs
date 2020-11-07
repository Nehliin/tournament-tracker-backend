use std::{io, net::TcpListener};

use tournament_tracker_backend::run;

#[actix_web::main]
async fn main() -> io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind address");
    run(listener)?.await
}
