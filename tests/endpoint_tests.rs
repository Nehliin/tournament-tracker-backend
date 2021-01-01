use reqwest::StatusCode;
use tournament_tracker_backend::stores::player_store::Player;

use common::spawn_server_and_authenticate;
mod common;

#[actix_rt::test]
async fn health_check_test() {
    let tt_client = spawn_server_and_authenticate().await;

    let response = tt_client
        .unauthenticated_client
        .client
        .get(&format!("{}/health_check", &tt_client.unauthenticated_client.server_addr))
        .send()
        .await
        .expect("Request failed");

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

#[actix_rt::test]
async fn insert_and_get_player_test() {
    let client = spawn_server_and_authenticate().await;

    let player = Player {
        id: 3,
        name: "Göte svensson".into(),
    };

    let response = client.insert_player(&player).await;

    assert!(response.status().is_success());

    let response = client.get_player(player.id).await;

    assert!(response.status().is_success());
    assert_eq!(
        player,
        response.json::<Player>().await.expect("Response body")
    );
}

#[actix_rt::test]
async fn should_404_on_missing_player() {
    let client = spawn_server_and_authenticate().await;
    let response = client.get_player(3).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
