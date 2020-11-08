use actix_web::{
    get, post,
    web::Path,
    web::{Data, Json},
    HttpResponse, Responder,
};
use chrono::Local;

use crate::{
    stores::{
        player_store::{Player, PlayerStore},
        tournament_store::{Tournament, TournamentStore},
    },
    ServerError,
};

#[get("/health_check")]
pub async fn health_check() -> HttpResponse {
    HttpResponse::Ok().finish()
}

#[post("/tournaments")]
pub async fn insert_tournament(
    tournament: Json<Tournament>,
    db: Data<TournamentStore>,
) -> Result<impl Responder, ServerError> {
    if tournament.start_date > tournament.end_date
        || tournament.start_date < Local::today().naive_local()
    {
        return Err(ServerError::InvalidDate);
    }

    db.insert_tournament(tournament.into_inner()).await?;
    Ok(HttpResponse::Ok())
}

#[get("/tournaments")]
pub async fn get_tournaments(db: Data<TournamentStore>) -> Result<impl Responder, ServerError> {
    let tournaments = db.get_tournaments().await?;
    Ok(HttpResponse::Ok().json(tournaments))
}

#[post("/players")]
pub async fn insert_player(
    player: Json<Player>,
    db: Data<PlayerStore>,
) -> Result<impl Responder, ServerError> {
    db.insert_player(&player).await?;
    Ok(HttpResponse::Ok())
}

#[get("/players/{id}")]
pub async fn get_player(
    id: Path<i64>,
    db: Data<PlayerStore>,
) -> Result<impl Responder, ServerError> {
    if let Some(player) = db.get_player(*id).await? {
        Ok(HttpResponse::Ok().json(player))
    } else {
        Ok(HttpResponse::NotFound().finish())
    }
}
