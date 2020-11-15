use crate::{
    stores::{
        court_store::CourtStore,
        match_store::{Match, MatchStore},
        player_registration_store::PlayerRegistrationStore,
        player_store::{Player, PlayerStore},
        tournament_store::{Tournament, TournamentStore},
    },
    ServerError,
};
use actix_web::{
    get, post, put,
    web::Path,
    web::{Data, Json},
    HttpResponse, Responder,
};
use chrono::{Local, NaiveDateTime};
use futures::future;
use serde::{Deserialize, Serialize};

#[get("/health_check")]
pub async fn health_check() -> HttpResponse {
    HttpResponse::Ok().finish()
}

// Tournament endpoints
#[tracing::instrument(name = "Insert tournament", skip(db))]
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

#[tracing::instrument(name = "Get tournaments", skip(db))]
#[get("/tournaments")]
pub async fn get_tournaments(db: Data<TournamentStore>) -> Result<impl Responder, ServerError> {
    let tournaments = db.get_tournaments().await?;
    Ok(HttpResponse::Ok().json(tournaments))
}

// Player endpoints
#[tracing::instrument(name = "Insert player", skip(db))]
#[post("/players")]
pub async fn insert_player(
    player: Json<Player>,
    db: Data<PlayerStore>,
) -> Result<impl Responder, ServerError> {
    db.insert_player(&player).await?;
    Ok(HttpResponse::Ok())
}

#[tracing::instrument(name = "Get player", skip(db))]
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
#[tracing::instrument(name = "Insert match", skip(db))]
#[post("/matches")]
pub async fn insert_match(
    match_data: Json<Match>,
    db: Data<MatchStore>,
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

#[derive(Debug, Serialize, Deserialize)]
pub struct PlayerMatchRegistrationRequest {
    pub player_id: i64,
    pub registered_by: String,
}

// TODO: This should probably take a form instead
#[tracing::instrument(
    name = "Register player to match",
    skip(match_store, player_registration_store)
)]
#[post("/matches/{match_id}/register/player")]
pub async fn register_player(
    match_id: Path<i64>,
    mut payload: Json<PlayerMatchRegistrationRequest>,
    player_registration_store: Data<PlayerRegistrationStore>,
    match_store: Data<MatchStore>,
) -> Result<impl Responder, ServerError> {
    let match_data = match_store.get_match(*match_id).await?;

    if match_data.player_one != payload.player_id && match_data.player_two != payload.player_id {
        return Err(ServerError::InvalidPlayerRegistration);
    }

    let previous_registration = player_registration_store
        .get_player_registration(payload.player_id, *match_id)
        .await?;
    if previous_registration.is_some() {
        return Err(ServerError::PlayerAlreadyReigstered);
    }

    let registered_by = std::mem::take(&mut payload.registered_by);
    let match_registration = player_registration_store
        .insert_player_registration(payload.player_id, *match_id, registered_by)
        .await?;
    Ok(HttpResponse::Ok().json(match_registration))
}

#[derive(Debug, Serialize, PartialEq)]
pub struct MatchInfo {
    id: i64,
    class: String,
    player_one: Player,
    player_two: Player,
    player_one_arrived: bool,
    player_two_arrived: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    court: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    winner: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<String>,
    start_time: NaiveDateTime,
}

impl MatchInfo {
    fn from_components(match_data: Match, player_one: Player, player_two: Player) -> Self {
        MatchInfo {
            id: match_data.id,
            class: match_data.class,
            start_time: match_data.start_time,
            player_one_arrived: false,
            player_two_arrived: false,
            player_one,
            player_two,
            winner: None,
            result: None,
            court: None,
        }
    }
}

#[tracing::instrument(
    name = "Start match",
    skip(match_store, player_registration_store, court_store)
)]
#[put("/matches/{match_id}/start")]
pub async fn start_match(
    match_id: Path<i64>,
    player_registration_store: Data<PlayerRegistrationStore>,
    match_store: Data<MatchStore>,
    court_store: Data<CourtStore>,
    player_storeage: Data<PlayerStore>,
) -> Result<impl Responder, ServerError> {
    let match_data = match_store.get_match(*match_id).await?;

    if match_data.is_none() {
        return Err(ServerError::MatchNotFound);
    }
    let match_data = match_data.unwrap();

    if let Some(current_court) = court_store
        .get_match_court(match_data.tournament_id, match_data.id)
        .await?
    {
        return Err(ServerError::MatchAlreadyStarted);
    }

    // if court already assigned -> return err
    // check player registration
    // if no court assigned and players are present
    // try to assign free court
    // if no free court add to court queue

    // in finish match
    // free up court
    // pop court queue

    if let (Ok(Some(registration_one)), Ok(Some(registration_two))) = future::join(
        player_registration_store.get_player_registration(match_data.player_one, match_data.id),
        player_registration_store.get_player_registration(match_data.player_two, match_data.id),
    )
    .await
    {
        if let Some(assigned_court) = court_store
            .try_assign_free_court(match_data.tournament_id, match_data.id)
            .await?
        {
            if let (Ok(Some(player_one)), Ok(Some(player_two))) = future::join(
                player_storage.get_player(match_data.player_one),
                player_storeage.get_player(match_data.player_two),
            )
            .await
            {
                let match_info = MatchInfo {
                    court: assigned_court,
                    player_one_arrived: true,
                    player_two_arrived: true,
                    start_time: Local::now().naive_local(),
                    ..MatchInfo::from_components(match_data, player_one, player_two)
                };
                Ok(HttpResponse::Ok().json(match_info))
            } else {
                Err(ServerError::FailedToFindPlayer)
            }
        } else {
            let court = court_store
                .append_court_queue(match_data.tournament_id, match_data.id)
                .await?;

            let match_info = MatchInfo {
                court,
                player_one_arrived: true,
                player_two_arrived: true,
                ..MatchInfo::from_components(match_data, player_one, player_two)
            };
            Ok(HttpResponse::Ok().json(match_info))
        }
    } else {
        Err(ServerError::PlayerMissing)
    }
}
