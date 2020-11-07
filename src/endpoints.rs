use actix_web::{
    get, post,
    web::{Data, Json},
    Error, HttpResponse, Responder,
};
use chrono::Local;

use crate::stores::tournament_store::{Tournament, TournamentStore, TournamentStoreError};

#[get("/health_check")]
pub async fn health_check() -> HttpResponse {
    HttpResponse::Ok().finish()
}

#[post("/tournaments")]
pub async fn insert_tournament(
    tournament: Json<Tournament>,
    db: Data<TournamentStore>,
) -> Result<impl Responder, TournamentStoreError> {
    if tournament.start_date > tournament.end_date
        || tournament.start_date < Local::today().naive_local()
    {
        return Err(TournamentStoreError::InvalidDate);
    }

    db.insert_tournament(tournament.into_inner()).await?;
    Ok(HttpResponse::Ok())
}

#[get("/tournaments")]
pub async fn get_tournaments(db: Data<TournamentStore>) -> Result<impl Responder, Error> {
    let tournaments = db.get_tournaments().await?;
    Ok(HttpResponse::Ok().json(tournaments))
}
