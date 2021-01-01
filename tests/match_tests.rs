use chrono::{Duration, Local};
use common::{spawn_server_and_authenticate, AuthenticatedClient};
use reqwest::{Response, StatusCode};
use tournament_tracker_backend::match_operations::MatchInfo;
use tournament_tracker_backend::stores::match_store::MatchResult;
use tournament_tracker_backend::{
    endpoints::PlayerMatchRegistrationPayload,
    match_operations::TournamentMatchList,
    stores::{
        match_store::Match, player_registration_store::PlayerMatchRegistration,
        player_store::Player, tournament_store::Tournament,
    },
};

mod common;

async fn insert_tournament_and_players(client: &AuthenticatedClient) -> (i32, i64, i64) {
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

async fn insert_match(
    client: &AuthenticatedClient,
    tournament_id: i32,
    player_one: i64,
    player_two: i64,
) -> i64 {
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
    response.text().await.unwrap().parse().unwrap()
}

async fn register_player(client: &AuthenticatedClient, match_id: i64, player_id: i64) {
    let player_registration = PlayerMatchRegistrationPayload {
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
    let client = spawn_server_and_authenticate().await;

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
    let client = spawn_server_and_authenticate().await;

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
    let client = spawn_server_and_authenticate().await;

    let player_registration = PlayerMatchRegistrationPayload {
        player_id: 0,
        registered_by: "Svante".to_string(),
    };

    let response = client.register_player(2, &player_registration).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[actix_rt::test]
async fn should_register_valid_player_and_start_match() {
    let client = spawn_server_and_authenticate().await;

    let (tournament_id, player_one, player_two) = insert_tournament_and_players(&client).await;

    let match_id = insert_match(&client, tournament_id, player_one, player_two).await;

    // register players to start the match
    register_player(&client, match_id, player_one).await;
    register_player(&client, match_id, player_two).await;

    // ensure the match is scheduled, the match will be #1 in the court queue
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
    let client = spawn_server_and_authenticate().await;

    let (tournament_id, player_one, player_two) = insert_tournament_and_players(&client).await;

    let match_id = insert_match(&client, tournament_id, player_one, player_two).await;

    // Try to register player not part of rooster
    let player_registration = PlayerMatchRegistrationPayload {
        player_id: 1337,
        registered_by: "Svante".to_string(),
    };
    let response = client.register_player(match_id, &player_registration).await;
    assert!(response.status().is_client_error());

    // Try to register player twice
    let player_registration = PlayerMatchRegistrationPayload {
        player_id: player_one,
        registered_by: "Svante".to_string(),
    };
    let response = client.register_player(match_id, &player_registration).await;
    assert!(response.status().is_success());

    // Second attempt should fail
    let player_registration = PlayerMatchRegistrationPayload {
        player_id: player_one,
        registered_by: "Svante".to_string(),
    };
    let response = client.register_player(match_id, &player_registration).await;
    assert!(response.status().is_client_error())
}

#[actix_rt::test]
async fn should_assign_free_court_if_available() {
    let client = spawn_server_and_authenticate().await;

    let (tournament_id, player_one, player_two) = insert_tournament_and_players(&client).await;

    let match_id = insert_match(&client, tournament_id, player_one, player_two).await;
    let response = client
        .add_court_to_tournament(tournament_id, "Bana 1".to_string())
        .await;
    assert!(response.status().is_success());
    // register players to start the match
    register_player(&client, match_id, player_one).await;
    register_player(&client, match_id, player_two).await;

    // ensure the match has started
    let response = client.get_tournaments_matches(tournament_id).await;
    assert!(response.status().is_success());
    let match_list = response.json::<TournamentMatchList>().await.unwrap();
    let playing_match = &match_list.playing[0];
    assert_eq!(playing_match.id, match_id);
    // assigned the free court
    assert_eq!(playing_match.court, Some("Bana 1".into()));
    assert_eq!(playing_match.player_one.id, player_one);
    assert_eq!(playing_match.player_two.id, player_two);
    assert!(playing_match.player_one_arrived);
    assert!(playing_match.player_two_arrived);

    let match_id_2 = insert_match(&client, tournament_id, player_one, player_two).await;
    // register players to start the match
    register_player(&client, match_id_2, player_one).await;
    register_player(&client, match_id_2, player_two).await;

    let response = client.get_tournaments_matches(tournament_id).await;
    let match_list = response.json::<TournamentMatchList>().await.unwrap();
    // same playing match as above
    assert_eq!(playing_match, &match_list.playing[0]);
    let scheduled_match = &match_list.scheduled[0];
    assert_eq!(scheduled_match.id, match_id_2);
    // First place in the court queue
    assert_eq!(scheduled_match.court, Some("Först i kön".into()));
    assert_eq!(scheduled_match.player_one.id, player_one);
    assert_eq!(scheduled_match.player_two.id, player_two);
    assert!(scheduled_match.player_one_arrived);
    assert!(scheduled_match.player_two_arrived);
}

#[actix_rt::test]
async fn should_assign_court_when_match_is_finished() {
    let client = spawn_server_and_authenticate().await;

    let (tournament_id, player_one, player_two) = insert_tournament_and_players(&client).await;

    let match_id_1 = insert_match(&client, tournament_id, player_one, player_two).await;
    let response = client
        .add_court_to_tournament(tournament_id, "Bana 1".to_string())
        .await;
    assert!(response.status().is_success());

    let player = Player {
        id: 2,
        name: "Kalle kula".into(),
    };

    // insert player 1
    let response = client.insert_player(&player).await;
    assert!(response.status().is_success());

    let player = Player {
        id: 3,
        name: "Snurre Sprätt".into(),
    };

    // insert player 2
    let response = client.insert_player(&player).await;
    assert!(response.status().is_success());
    let match_id_2 = insert_match(&client, tournament_id, 2, 3).await;

    // register players to start the match
    register_player(&client, match_id_1, player_one).await;
    register_player(&client, match_id_1, player_two).await;
    // register players which adds the match to the queue
    register_player(&client, match_id_2, 2).await;
    register_player(&client, match_id_2, 3).await;

    // assert one match is playing and one is waiting for a court
    let response = client.get_tournaments_matches(tournament_id).await;
    assert!(response.status().is_success());
    let match_list = response.json::<TournamentMatchList>().await.unwrap();
    assert_eq!(match_list.playing.len(), 1);
    assert_eq!(match_list.playing[0].id, match_id_1);
    assert_eq!(match_list.scheduled.len(), 1);
    assert_eq!(match_list.scheduled[0].id, match_id_2);

    // finish match
    let response = client
        .finish_match(
            match_id_1,
            &MatchResult {
                result: "2-3(2) 4-4 3-3(2)".to_string(),
                winner: player_one,
            },
        )
        .await;
    assert!(response.status().is_success());
    let match_info = response.json::<MatchInfo>().await.unwrap();
    assert_eq!(match_info.result, Some("2-3(2) 4-4 3-3(2)".to_string()));
    assert_eq!(match_info.winner, Some(player_one));
    assert_eq!(match_info.court, None);
    assert!(match_info.player_two_arrived);
    assert!(match_info.player_one_arrived);
    assert_eq!(match_info.player_one.id, player_one);
    assert_eq!(match_info.player_two.id, player_two);
    // assert tournament_matches has been updated
    let response = client.get_tournaments_matches(tournament_id).await;
    assert!(response.status().is_success());
    let match_list = response.json::<TournamentMatchList>().await.unwrap();
    // match has been finished
    assert_eq!(match_list.finished.len(), 1);
    assert_eq!(match_list.finished[0].id, match_id_1);
    assert_eq!(match_list.finished[0], match_info);
    // match 2 has started playing on the correct court
    assert_eq!(match_list.playing.len(), 1);
    assert_eq!(match_list.playing[0].id, match_id_2);
    assert_eq!(match_list.playing[0].court, Some("Bana 1".to_string()));
    // no scheduled matches
    assert!(match_list.scheduled.is_empty());
}

async fn create_and_finish_match(
    client: &AuthenticatedClient,
    match_result: &MatchResult,
) -> Response {
    let (tournament_id, player_one, player_two) = insert_tournament_and_players(&client).await;

    let match_id = insert_match(&client, tournament_id, player_one, player_two).await;
    let response = client
        .add_court_to_tournament(tournament_id, "Bana 1".to_string())
        .await;
    assert!(response.status().is_success());
    // register players to start the match
    register_player(&client, match_id, player_one).await;
    register_player(&client, match_id, player_two).await;

    client.finish_match(match_id, match_result).await
}

#[actix_rt::test]
async fn should_not_allow_invalid_result() {
    let client = spawn_server_and_authenticate().await;
    let response = create_and_finish_match(
        &client,
        &MatchResult {
            result: "2-3-4-5 6-2(2)".to_string(),
            winner: 0, // player_one
        },
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[actix_rt::test]
async fn should_not_allow_invalid_winner() {
    let client = spawn_server_and_authenticate().await;
    let response = create_and_finish_match(
        &client,
        &MatchResult {
            result: "2-3(2) 4-4 3-3(2)".to_string(),
            winner: 10,
        },
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
