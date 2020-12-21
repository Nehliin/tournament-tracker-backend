use crate::stores::court_store::pop_court_queue;
use crate::stores::match_store::MatchResult;
use crate::{
    endpoints::PlayerMatchRegistrationRequest,
    stores::match_store::Match,
    stores::{
        court_store::CourtStore,
        match_store::MatchStore,
        player_registration_store::{PlayerMatchRegistration, PlayerRegistrationStore},
        player_store::Player,
        player_store::PlayerStore,
    },
    ServerError,
};
use chrono::{Local, NaiveDateTime};
use futures::future;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::Deserialize;
use serde::Serialize;
use sqlx::PgPool;
use tracing::{error, info, warn};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct MatchInfo {
    pub id: i64,
    pub class: String,
    pub player_one: Player,
    pub player_two: Player,
    pub player_one_arrived: bool,
    pub player_two_arrived: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub court: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub winner: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
    pub start_time: NaiveDateTime,
}

impl MatchInfo {
    fn without_winner(match_data: Match, player_info: PlayerMatchInfo, court: String) -> Self {
        MatchInfo {
            court: Some(court),
            ..MatchInfo::without_winner_and_court(match_data, player_info)
        }
    }

    fn without_winner_and_court(match_data: Match, player_info: PlayerMatchInfo) -> Self {
        MatchInfo {
            id: match_data.id,
            class: match_data.class,
            start_time: match_data.start_time,
            player_one_arrived: player_info.first_player_arrived,
            player_two_arrived: player_info.second_player_arrived,
            player_one: player_info.first_player,
            player_two: player_info.second_player,
            winner: None,
            court: None,
            result: None,
        }
    }

    fn with_winner(match_data: Match, player_info: PlayerMatchInfo, result: MatchResult) -> Self {
        MatchInfo {
            winner: Some(result.winner),
            result: Some(result.result),
            ..MatchInfo::without_winner_and_court(match_data, player_info)
        }
    }
}

pub async fn register_player_to_match(
    storage: &PgPool,
    match_id: i64,
    mut request: PlayerMatchRegistrationRequest,
) -> Result<PlayerMatchRegistration, ServerError> {
    let match_data = storage.get_match(match_id).await?;

    if match_data.is_none() {
        return Err(ServerError::MatchNotFound);
    }
    let match_data = match_data.unwrap();

    if match_data.player_one != request.player_id && match_data.player_two != request.player_id {
        return Err(ServerError::InvalidPlayerRegistration);
    }

    let previous_registrations = storage.get_registered_players(match_id).await?;

    if previous_registrations
        .iter()
        .any(|registration| registration.player_id == request.player_id)
    {
        return Err(ServerError::PlayerAlreadyReigstered);
    }

    let second_player_registerd = !previous_registrations.is_empty();

    let registered_by = std::mem::take(&mut request.registered_by);
    let match_registration = storage
        .insert_player_registration(request.player_id, match_id, registered_by)
        .await?;

    if second_player_registerd {
        start_match(match_id, storage).await?;
    }
    Ok(match_registration)
}
#[derive(Debug, Serialize, Deserialize)]
pub struct TournamentMatchList {
    pub scheduled: Vec<MatchInfo>,
    pub playing: Vec<MatchInfo>,
    pub finished: Vec<MatchInfo>,
}

#[tracing::instrument(name = "Get tournament match list", skip(storage))]
pub async fn get_tournament_matches(
    tournament_id: i32,
    storage: &PgPool,
) -> Result<TournamentMatchList, ServerError> {
    let query_result = storage.get_tournament_matches(tournament_id).await?;

    let mut finished = Vec::new();
    let mut playing = Vec::new();
    let mut scheduled = Vec::new();

    for match_data in query_result.into_iter() {
        let match_info_future = future::join(
            get_match_player_info(storage, &match_data),
            storage.get_match_result(match_data.id),
        );
        match match_info_future.await {
            (Ok(player_match_info), Some(result)) => {
                // The match is finished
                finished.push(MatchInfo::with_winner(
                    match_data,
                    player_match_info,
                    result,
                ));
            }
            (Ok(player_match_info), None) => {
                let incomplete_match_info =
                    MatchInfo::without_winner_and_court(match_data, player_match_info);
                if let Some(court) = storage
                    .get_match_court(tournament_id, incomplete_match_info.id)
                    .await
                {
                    // If the match has been assigned a court but has no winner
                    // the match is ongoing
                    playing.push(MatchInfo {
                        court: Some(court),
                        ..incomplete_match_info
                    });
                } else {
                    match storage
                        .get_court_queue_placement(tournament_id, incomplete_match_info.id)
                        .await
                    {
                        // If the match has not been assigned a court and doesn't have a winner it hasn't started
                        Ok(queue_placement) => {
                            scheduled.push(MatchInfo {
                                court: Some(get_placement_string(queue_placement)),
                                ..incomplete_match_info
                            });
                        }
                        Err(sqlx::Error::RowNotFound) => {
                            error!(
                                "Match {} should be in the court queue!",
                                incomplete_match_info.id
                            );
                        }
                        _ => {}
                    }
                }
            }
            (Err(err), _) => warn!("Player info not found for match: {}", err),
        }
    }
    Ok(TournamentMatchList {
        finished,
        playing,
        scheduled,
    })
}

#[tracing::instrument(name = "Start match", skip(storage))]
pub async fn start_match(match_id: i64, storage: &PgPool) -> Result<MatchInfo, ServerError> {
    let match_data = storage.get_match(match_id).await?;

    if match_data.is_none() {
        return Err(ServerError::MatchNotFound);
    }
    let match_data = match_data.unwrap();

    // check if court alreay has assigned court
    if storage
        .get_match_court(match_data.tournament_id, match_data.id)
        .await
        .is_some()
    {
        return Err(ServerError::MatchAlreadyStarted);
    }

    // check player registration
    let player_info = get_match_player_info(storage, &match_data).await?;
    // if no court assigned and players are present
    // try to assign free court
    if let Ok(assigned_court) = storage
        .try_assign_free_court(match_data.tournament_id, match_data.id)
        .await
    {
        Ok(MatchInfo {
            start_time: Local::now().naive_local(),
            ..MatchInfo::without_winner(match_data, player_info, assigned_court)
        })
    } else {
        let court =
            append_to_queue_and_get_placement(storage, match_data.tournament_id, match_id).await?;
        Ok(MatchInfo::without_winner(match_data, player_info, court))
    }
}

#[tracing::instrument(name = "Finish match", skip(storage))]
pub async fn finish_match(
    match_id: i64,
    result: MatchResult,
    storage: &PgPool,
) -> Result<MatchInfo, ServerError> {
    let match_data = storage.get_match(match_id).await?;

    let match_data = match match_data {
        Some(data) => data,
        None => return Err(ServerError::MatchNotFound),
    };

    let _ = check_valid_match_result(&result, &match_data)?;

    if storage.get_match_result(match_id).await.is_some() {
        return Err(ServerError::MatchAlreadyCompleted);
    }

    if storage
        .get_match_court(match_data.tournament_id, match_id)
        .await
        .is_none()
    {
        return Err(ServerError::MatchNotStarted);
    }

    storage.insert_match_result(match_id, &result).await?;
    // will rollback if dropped -> failures will result in rollback
    // 1. create transaction
    // 2. remove court assoication to the match
    // 3. pop court queue
    // 4. assign next match in the queue the free court
    let mut transaction = storage.begin().await?;
    let _ = transaction
        .remove_assigned_court(match_data.tournament_id, match_id)
        .await?;
    if let Some(waiting_match) = pop_court_queue(&mut transaction, match_data.tournament_id).await?
    {
        // There should always exist a free court here since the finished match just
        // freed up the court it used
        let court_name = transaction
            .try_assign_free_court(match_data.tournament_id, waiting_match)
            .await?;
        info!(
            "Assigning court: {} to match: {}",
            court_name, waiting_match
        );
        transaction.commit().await.map_err(|err| {
            error!("Transaction failed!");
            err
        })?;
    } else {
        transaction.commit().await.map_err(|err| {
            error!("Transaction failed!");
            err
        })?;
    }
    let player_info = get_match_player_info(storage, &match_data).await?;
    Ok(MatchInfo::with_winner(match_data, player_info, result))
}

// HELPERS:
#[derive(Debug)]
struct PlayerMatchInfo {
    first_player: Player,
    first_player_arrived: bool,
    second_player: Player,
    second_player_arrived: bool,
}

async fn get_match_player_info<S: PlayerStore + PlayerRegistrationStore>(
    storage: &S,
    match_data: &Match,
) -> Result<PlayerMatchInfo, ServerError> {
    if let (Ok(Some(first_player)), Ok(Some(second_player))) = future::join(
        storage.get_player(match_data.player_one),
        storage.get_player(match_data.player_two),
    )
    .await
    {
        let registered_players = storage.get_registered_players(match_data.id).await?;
        let mut first_player_arrived = false;
        let mut second_player_arrived = false;

        for registration in registered_players.iter() {
            if registration.player_id == first_player.id {
                first_player_arrived = true;
            } else if registration.player_id == second_player.id {
                second_player_arrived = true;
            }
        }

        Ok(PlayerMatchInfo {
            first_player_arrived,
            second_player_arrived,
            first_player,
            second_player,
        })
    } else {
        Err(ServerError::PlayerNotFound)
    }
}

const PATTERN: &str = r"^([0-9]+-[0-9]+(\([0-9]+\))?\s{1})+([0-9]+-[0-9]+(\([0-9]+\))?)$";
static RESULT_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(PATTERN).expect("Regex is invalid"));

fn check_valid_match_result(result: &MatchResult, match_data: &Match) -> Result<(), ServerError> {
    if result.winner != match_data.player_one && result.winner != match_data.player_two {
        Err(ServerError::InvalidWinner)
    } else if !RESULT_REGEX.is_match(&result.result.trim()) {
        Err(ServerError::InvalidResult)
    } else {
        Ok(())
    }
}

fn get_placement_string(placement: usize) -> String {
    match placement {
        1 => "Först i kön",
        2 => "Andra plats i kön",
        _ => "Köplats: {}",
    }
    .into()
}

async fn append_to_queue_and_get_placement(
    storage: &PgPool,
    tournament_id: i32,
    match_id: i64,
) -> Result<String, sqlx::Error> {
    storage.append_court_queue(tournament_id, match_id).await?;
    let placement = storage
        .get_court_queue_placement(tournament_id, match_id)
        .await?;
    Ok(get_placement_string(placement))
}
