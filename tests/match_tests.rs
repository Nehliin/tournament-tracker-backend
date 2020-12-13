use chrono::{Duration, Local};
use common::{spawn_server, TournamentTrackerClient};
use reqwest::StatusCode;
use tournament_tracker_backend::{
    endpoints::PlayerMatchRegistrationRequest,
    match_operations::TournamentMatchList,
    stores::{
        match_store::Match, player_registration_store::PlayerMatchRegistration,
        player_store::Player, tournament_store::Tournament,
    },
};

mod common;

async fn insert_tournament_and_players(client: &TournamentTrackerClient) -> (i32, i64, i64) {
    let start_date = Local::today().naive_local();
    let tournament = Tournament {
        id: 0, // doesn't matter
        name: "Södertälje open".into(),
        start_date,
        end_date: start_date + Duration::days(1),
    };

    // insert tournament
    let response = client.insert_tournament(&tournament).await;
    assert!(response.status().is_success());
    let tournament_id = response.text().await.unwrap();

    let player = Player {
        id: 0,
        name: "Göte svensson".into(),
    };

    // insert player 1
    let response = client.insert_player(&player).await;
    assert!(response.status().is_success());

    let player = Player {
        id: 1,
        name: "Sture svensson".into(),
    };

    // insert player 2
    let response = client.insert_player(&player).await;
    assert!(response.status().is_success());

    (tournament_id.parse::<i32>().unwrap(), 0, 1)
}

async fn register_player(client: &TournamentTrackerClient, match_id: i64, player_id: i64) {
    let player_registration = PlayerMatchRegistrationRequest {
        player_id,
        registered_by: "Svante".to_string(),
    };

    // register player 1
    let response = client.register_player(match_id, &player_registration).await;
    assert!(response.status().is_success());
    let actual = response.json::<PlayerMatchRegistration>().await.unwrap();

    assert_eq!(player_id, actual.player_id);
    assert_eq!(match_id, actual.match_id);
    assert_eq!("Svante".to_string(), actual.registerd_by);
}

#[actix_rt::test]
async fn should_fail_to_register_match_with_invalid_rooster() {
    let client = spawn_server().await;

    let (tournament_id, player_one, _) = insert_tournament_and_players(&client).await;

    let match_data = Match {
        id: 0, // not important
        player_one,
        player_two: player_one, // Can't play against yourself!
        tournament_id,
        class: "p96".to_string(),
        start_time: Local::now().naive_local() + Duration::hours(2),
    };

    let response = client.insert_match(&match_data).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[actix_rt::test]
async fn should_fail_to_register_match_with_invalid_start_date() {
    let client = spawn_server().await;

    let (tournament_id, player_one, player_two) = insert_tournament_and_players(&client).await;

    let match_data = Match {
        id: 0, // not important
        player_one,
        player_two,
        tournament_id,
        class: "p96".to_string(),
        start_time: Local::now().naive_local() - Duration::hours(2),
    };

    let response = client.insert_match(&match_data).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[actix_rt::test]
async fn should_fail_to_register_to_missing_match() {
    let client = spawn_server().await;

    let player_registration = PlayerMatchRegistrationRequest {
        player_id: 0,
        registered_by: "Svante".to_string(),
    };

    let response = client.register_player(2, &player_registration).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[actix_rt::test]
async fn should_register_valid_player_and_start_match() {
    let client = spawn_server().await;

    let (tournament_id, player_one, player_two) = insert_tournament_and_players(&client).await;

    // insert match
    let match_data = Match {
        id: 0, // not important
        player_one,
        player_two,
        tournament_id,
        class: "p96".to_string(),
        start_time: Local::now().naive_local() + Duration::hours(2),
    };

    let response = client.insert_match(&match_data).await;
    assert!(response.status().is_success());
    let match_id: i64 = response.text().await.unwrap().parse().unwrap();

    // register players to start the match
    register_player(&client, match_id, player_one).await;
    register_player(&client, match_id, player_two).await;

    // ensure the match has started, the match will be #1 in the court queue
    let response = client.get_tournaments_matches(tournament_id).await;
    assert!(response.status().is_success());
    let match_list = response.json::<TournamentMatchList>().await.unwrap();
    let scheduled_match = &match_list.scheduled[0];
    assert_eq!(scheduled_match.id, match_id);
    // first in the queue
    assert_eq!(scheduled_match.court, Some("Först i kön".into()));
    assert_eq!(scheduled_match.player_one.id, player_one);
    assert_eq!(scheduled_match.player_two.id, player_two);
    assert!(scheduled_match.player_one_arrived);
    assert!(scheduled_match.player_two_arrived);
}

#[actix_rt::test]
async fn should_not_register_invalid_player() {
    let client = spawn_server().await;

    let (tournament_id, player_one, player_two) = insert_tournament_and_players(&client).await;

    // insert match
    let match_data = Match {
        id: 0, // not important
        player_one,
        player_two,
        tournament_id,
        class: "p96".to_string(),
        start_time: Local::now().naive_local() + Duration::hours(2),
    };

    let response = client.insert_match(&match_data).await;
    assert!(response.status().is_success());
    let match_id = response.text().await.unwrap();

    // Try to register player not part of rooster
    let player_registration = PlayerMatchRegistrationRequest {
        player_id: 1337,
        registered_by: "Svante".to_string(),
    };
    let response = client
        .register_player(match_id.parse::<i64>().unwrap(), &player_registration)
        .await;
    assert!(response.status().is_client_error());

    // Try to register player twice
    let player_registration = PlayerMatchRegistrationRequest {
        player_id: player_one,
        registered_by: "Svante".to_string(),
    };
    let response = client
        .register_player(match_id.parse::<i64>().unwrap(), &player_registration)
        .await;
    assert!(response.status().is_success());

    // Second attempt should fail
    let player_registration = PlayerMatchRegistrationRequest {
        player_id: player_one,
        registered_by: "Svante".to_string(),
    };
    let response = client
        .register_player(match_id.parse::<i64>().unwrap(), &player_registration)
        .await;
    assert!(response.status().is_client_error())
}
