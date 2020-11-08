use chrono::{Duration, Local};
use common::{spawn_server, TournamentTrackerClient};
use reqwest::StatusCode;
use tournament_tracker_backend::{
    endpoints::PlayerMatchRegistrationRequest,
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
    assert_eq!(response.status(), StatusCode::from_u16(400).unwrap());
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
    assert_eq!(response.status(), StatusCode::from_u16(400).unwrap());
}

#[actix_rt::test]
async fn should_fail_invalid_player_registration() {
    let client = spawn_server().await;

    let player_registration = PlayerMatchRegistrationRequest {
        player_id: 0,
        registered_by: "Svante".to_string(),
    };

    let response = client.register_player(2, &player_registration).await;
    assert_eq!(response.status(), StatusCode::from_u16(500).unwrap());
}

#[actix_rt::test]
async fn should_register_valid_player() {
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

    let player_registration = PlayerMatchRegistrationRequest {
        player_id: 1,
        registered_by: "Svante".to_string(),
    };

    // register player 2
    let response = client
        .register_player(match_id.parse::<i64>().unwrap(), &player_registration)
        .await;
    assert!(response.status().is_success());
    let actual = response.json::<PlayerMatchRegistration>().await.unwrap();

    assert_eq!(1, actual.player_id);
    assert_eq!(match_id.parse::<i64>().unwrap(), actual.match_id);
    assert_eq!("Svante".to_string(), actual.registerd_by);
}

#[actix_rt::test]
async fn should_not_register_invalid_player() {
    // TODO
}
