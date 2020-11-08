use crate::{
    stores::{
        match_store::{Match, MatchStore},
        player_registration_store::PlayerRegistrationStore,
        player_store::{Player, PlayerStore},
        tournament_store::{Tournament, TournamentStore},
    },
    ServerError,
};
use actix_web::{
    get, post,
    web::Path,
    web::{Data, Json},
    HttpResponse, Responder,
};
use chrono::Local;
use serde::{Deserialize, Serialize};

#[get("/health_check")]
pub async fn health_check() -> HttpResponse {
    HttpResponse::Ok().finish()
}

// Tournament endpoints

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

    let id = db.insert_tournament(tournament.into_inner()).await?;
    Ok(HttpResponse::Ok().body(id.to_string()))
}

#[get("/tournaments")]
pub async fn get_tournaments(db: Data<TournamentStore>) -> Result<impl Responder, ServerError> {
    let tournaments = db.get_tournaments().await?;
    Ok(HttpResponse::Ok().json(tournaments))
}

// Player endpoints

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

// Match endpoints

#[post("/matches")]
pub async fn insert_match(match_data: Json<Match>, db: Data<MatchStore>) -> Result<impl Responder, ServerError> {
    if match_data.start_time < Local::now().naive_local() {
        println!("invalid");
        Err(ServerError::InvalidStartTime) 
    } else {
        let id = db.insert_match(match_data.into_inner()).await?;
        Ok(HttpResponse::Ok().body(id.to_string()))
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PlayerMatchRegistrationRequest {
    pub player_id: i64,
    pub registered_by: String,
}
// TODO: This should probably take a form instead
#[post("/matches/{match_id}/register/player")]
pub async fn register_player(
    match_id: Path<i64>,
    mut payload: Json<PlayerMatchRegistrationRequest>,
    store: Data<PlayerRegistrationStore>,
) -> Result<impl Responder, ServerError> {
    let registered_by = std::mem::take(&mut payload.registered_by);
    let match_registration = store
        .insert_player_registration(payload.player_id, *match_id, registered_by)
        .await?;
    Ok(HttpResponse::Ok().json(match_registration))
}
