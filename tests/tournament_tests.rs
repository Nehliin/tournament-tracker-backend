use chrono::{Duration, Local};
use common::spawn_server;
use reqwest::StatusCode;
use tournament_tracker_backend::stores::tournament_store::Tournament;

mod common;

#[actix_rt::test]
async fn insert_tournament_test() {
    let client = spawn_server().await;

    let start_date = Local::today().naive_local();

    let tournament = Tournament {
        id: 0, // doesn't matter
        name: "Södertälje open".into(),
        start_date,
        end_date: start_date + Duration::days(1),
    };

    let response = client.insert_tournament(&tournament).await;
    assert!(response.status().is_success());
    // id should be part of the response
    assert!(!response.text().await.unwrap().is_empty());
}

#[actix_rt::test]
async fn invalid_date_test() {
    let client = spawn_server().await;

    let start_date = Local::today().naive_local();

    let tournament = Tournament {
        id: 0, // doesn't matter
        name: "Södertälje open".into(),
        start_date,
        end_date: start_date - Duration::days(1),
    };

    let response = client.insert_tournament(&tournament).await;
    assert!(!response.status().is_success());
    assert_eq!(response.status(), StatusCode::from_u16(403).unwrap());
    // id should not be part of the response
    assert!(response.text().await.unwrap().is_empty());
}

#[actix_rt::test]
async fn get_tournament_test() {
    let client = spawn_server().await;

    let start_date = Local::today().naive_local();

    let tournament = Tournament {
        id: 0, // doesn't matter
        name: "Södertälje open".into(),
        start_date,
        end_date: start_date + Duration::days(1),
    };

    let response = client.insert_tournament(&tournament).await;
    assert!(response.status().is_success());

    let response = client.get_tournaments().await;

    assert!(response.status().is_success());

    let tournament_list = response
        .json::<Vec<Tournament>>()
        .await
        .expect("Response body");
    assert_eq!(tournament_list.len(), 1);

    let tournament = Tournament {
        id: tournament_list[0].id,
        ..tournament
    };

    assert_eq!(tournament_list[0], tournament);
}
