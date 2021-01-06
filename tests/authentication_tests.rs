mod common;

use chrono::{Duration, Local};
use common::spawn_server_and_authenticate;
use reqwest::StatusCode;
use tournament_tracker_backend::{
    endpoints::{CredentialsPayload, PlayerMatchRegistrationPayload},
    stores::{match_store::MatchResult, tournament_store::Tournament},
};

#[actix_rt::test]
async fn should_not_allow_invalid_email_or_password() {
    let client = spawn_server_and_authenticate().await.unauthenticated_client;
    let response = client
        .create_user(&CredentialsPayload {
            email: "invalid_email.com".into(),
            password: "some secure password".into(),
        })
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let response = client
        .create_user(&CredentialsPayload {
            email: "valid.email@google.com".into(),
            password: "short".into(),
        })
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[actix_rt::test]
async fn should_not_allow_unauthenticated_requests_to_protected_endpoints() {
    let client = spawn_server_and_authenticate().await.unauthenticated_client;
    let start_date = Local::today().naive_local();
    let response = client
        .insert_tournament(&Tournament {
            id: 0,
            name: "test".into(),
            start_date,
            end_date: start_date + Duration::days(1),
        })
        .await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let response = client
        .finish_match(
            0,
            &MatchResult {
                result: "1-2 3-4 5-3".into(),
                winner: 1,
            },
        )
        .await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let response = client
        .register_player(
            0,
            &PlayerMatchRegistrationPayload {
                player_id: 0,
                registered_by: "Gösta".into(),
            },
        )
        .await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[actix_rt::test]
async fn should_block_deleted_users() {
    let client = spawn_server_and_authenticate().await;
    let response = client
        .insert_tournament(&Tournament {
            id: 0,
            name: "Dummy".to_string(),
            start_date: Local::today().naive_local(),
            end_date: Local::today().naive_local(),
        })
        .await;
    // allowed
    assert_eq!(response.status(), StatusCode::OK);
    let response = client.delete_user().await;
    assert_eq!(response.status(), StatusCode::OK);
    let response = client
        .insert_tournament(&Tournament {
            id: 1,
            name: "Dummy 2".to_string(),
            start_date: Local::today().naive_local(),
            end_date: Local::today().naive_local(),
        })
        .await;
    // token no longer valid because the user is deleted
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        response.text().await.unwrap(),
        "Invalid auth token: User no longer exists".to_string()
    );
}

// TODO: Add tests for token expiration
