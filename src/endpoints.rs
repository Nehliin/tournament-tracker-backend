use crate::authentication::{create_user, login_user};
use crate::match_operations::finish_match;
use crate::stores::match_store::MatchResult;
use crate::{
    match_operations::register_player_to_match,
    stores::{
        court_store::{CourtStore, TournamentCourtAllocation},
        match_store::{Match, MatchStore},
        player_store::{Player, PlayerStore},
        tournament_store::{Tournament, TournamentStore},
    },
    ServerError,
};
use actix_web::{
    get, post,
    web::Path,
    web::{Data, Form, Json},
    HttpResponse, Responder,
};
use chrono::Local;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tracing::info;

#[get("/health_check")]
pub async fn health_check() -> HttpResponse {
    HttpResponse::Ok().finish()
}

// Auth endpoints
#[derive(Debug, Serialize, Deserialize)]
pub struct CreadentialsPayload {
    pub email: String,
    pub password: String,
}

#[tracing::instrument(name = "User login", skip(db, payload))]
#[post("/login")]
pub async fn login(
    payload: Json<CreadentialsPayload>,
    db: Data<PgPool>,
) -> Result<impl Responder, ServerError> {
    if payload.email.is_empty() {
        return Err(ServerError::InvalidEmail);
    }
    if payload.password.is_empty() {
        return Err(ServerError::InvalidPassword);
    }
    info!("Attempting login for user: {}", payload.email);
    let token = login_user(&db, &payload.email, &payload.password).await?;
    Ok(HttpResponse::Ok().body(token))
}

#[tracing::instrument(name = "Create new user", skip(db, payload))]
#[post("/user")]
pub async fn create_new_user(
    payload: Json<CreadentialsPayload>,
    db: Data<PgPool>,
) -> Result<impl Responder, ServerError> {
    if payload.email.is_empty() {
        return Err(ServerError::InvalidEmail);
    }
    if payload.password.is_empty() {
        return Err(ServerError::InvalidPassword);
    }
    info!("Attempting user registration for email: {}", payload.email);
    let _ = create_user(&db, &payload.email, &payload.password).await?;
    Ok(HttpResponse::Ok())
}

// Tournament endpoints
#[tracing::instrument(name = "Insert tournament", skip(db))]
#[post("/tournaments")]
pub async fn insert_tournament(
    tournament: Json<Tournament>,
    db: Data<PgPool>,
) -> Result<impl Responder, ServerError> {
    if tournament.start_date > tournament.end_date
        || tournament.start_date < Local::today().naive_local()
    {
        return Err(ServerError::InvalidDate);
    }

    let id = db.insert_tournament(tournament.into_inner()).await?;
    Ok(HttpResponse::Ok().body(id.to_string()))
}

#[tracing::instrument(name = "Get tournaments", skip(db))]
#[get("/tournaments")]
pub async fn get_tournaments(db: Data<PgPool>) -> Result<impl Responder, ServerError> {
    let tournaments = db.get_tournaments().await?;
    Ok(HttpResponse::Ok().json(tournaments))
}

#[tracing::instrument(name = "Get tournament matches", skip(db))]
#[get("/tournaments/{id}/matches")]
pub async fn get_tournament_matches(
    id: Path<i32>,
    db: Data<PgPool>,
) -> Result<impl Responder, ServerError> {
    let tournaments =
        crate::match_operations::get_tournament_matches(*id, &*db.into_inner()).await?;
    Ok(HttpResponse::Ok().json(tournaments))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CourtForm {
    pub name: String,
}

#[tracing::instrument(name = "Add court to tournament", skip(db))]
#[post("/tournaments/{id}/courts")]
pub async fn add_court_to_tournament(
    id: Path<i32>,
    court_form: Form<CourtForm>,
    db: Data<PgPool>,
) -> Result<impl Responder, ServerError> {
    let court_allocation = TournamentCourtAllocation {
        court_name: court_form.into_inner().name,
        tournament_id: *id,
        match_id: None,
    };
    db.insert_tournament_court_allocation(court_allocation)
        .await?;
    Ok(HttpResponse::Ok())
}

// Player endpoints
#[tracing::instrument(name = "Insert player", skip(db))]
#[post("/players")]
pub async fn insert_player(
    player: Json<Player>,
    db: Data<PgPool>,
) -> Result<impl Responder, ServerError> {
    db.insert_player(&player).await?;
    Ok(HttpResponse::Ok())
}

#[tracing::instrument(name = "Get player", skip(db))]
#[get("/players/{id}")]
pub async fn get_player(id: Path<i64>, db: Data<PgPool>) -> Result<impl Responder, ServerError> {
    if let Some(player) = db.get_player(*id).await? {
        Ok(HttpResponse::Ok().json(player))
    } else {
        Ok(HttpResponse::NotFound().finish())
    }
}

// Match endpoints
#[tracing::instrument(name = "Insert match", skip(db))]
#[post("/matches")]
pub async fn insert_match(
    match_data: Json<Match>,
    db: Data<PgPool>,
) -> Result<impl Responder, ServerError> {
    if match_data.start_time < Local::now().naive_local() {
        Err(ServerError::InvalidStartTime)
    } else if match_data.player_one == match_data.player_two {
        Err(ServerError::InvalidRooster)
    } else {
        let id = db.insert_match(match_data.into_inner()).await?;
        Ok(HttpResponse::Ok().body(id.to_string()))
    }
}

#[tracing::instrument(name = "Finish match", skip(db))]
#[post("/matches/{match_id}/finish")]
pub async fn finish_match_endpoint(
    id: Path<i64>,
    result: Json<MatchResult>,
    db: Data<PgPool>,
) -> Result<impl Responder, ServerError> {
    let match_info = finish_match(*id, result.into_inner(), &db).await?;
    Ok(HttpResponse::Ok().json(match_info))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PlayerMatchRegistrationPayload {
    pub player_id: i64,
    pub registered_by: String,
}

// TODO: This should probably take a form instead
#[tracing::instrument(name = "Register player to match", skip(storage))]
#[post("/matches/{match_id}/register/player")]
pub async fn register_player(
    match_id: Path<i64>,
    payload: Json<PlayerMatchRegistrationPayload>,
    storage: Data<PgPool>,
) -> Result<impl Responder, ServerError> {
    let match_registration =
        register_player_to_match(&*storage.into_inner(), *match_id, payload.into_inner()).await?;
    Ok(HttpResponse::Ok().json(match_registration))
}
