#![allow(clippy::toplevel_ref_arg)]

use async_trait::async_trait;
use chrono::{Local, NaiveDateTime};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tracing::error;
#[derive(Debug, sqlx::FromRow, Deserialize, Serialize)]
pub struct PlayerMatchRegistration {
    pub player_id: i64,
    pub match_id: i64,
    pub time_registerd: NaiveDateTime,
    pub registerd_by: String,
}

#[async_trait]
pub trait PlayerRegistrationStore {
    async fn insert_player_registration(
        &self,
        player_id: i64,
        match_id: i64,
        registerd_by: String,
    ) -> Result<PlayerMatchRegistration, sqlx::Error>;

    async fn get_registered_players(
        &self,
        player_id: i64,
    ) -> Result<Vec<PlayerMatchRegistration>, sqlx::Error>;
}

#[async_trait]
impl PlayerRegistrationStore for PgPool {
    #[tracing::instrument(name = "Inserting player registration", skip(self))]
    async fn insert_player_registration(
        &self,
        player_id: i64,
        match_id: i64,
        registerd_by: String,
    ) -> Result<PlayerMatchRegistration, sqlx::Error> {
        let match_registration = PlayerMatchRegistration {
            player_id,
            match_id,
            time_registerd: Local::now().naive_local(),
            registerd_by,
        };
        sqlx::query!("INSERT INTO register (player_id, match_id, time_registerd, registerd_by) VALUES ($1, $2, $3, $4)",
            match_registration.player_id,
            match_registration.match_id,
            match_registration.time_registerd,
            match_registration.registerd_by,
        ).execute(self).await
        .map_err(|err| {
            error!("Failed to register player {}", err);
            err
        })?;

        Ok(match_registration)
    }

    #[tracing::instrument(name = "Fetching registerad players", skip(self))]
    async fn get_registered_players(
        &self,
        match_id: i64,
    ) -> Result<Vec<PlayerMatchRegistration>, sqlx::Error> {
        Ok(sqlx::query_as!(
            PlayerMatchRegistration,
            "SELECT * FROM register WHERE match_id = $1",
            match_id,
        )
        .fetch_all(self)
        .await
        .map_err(|err| {
            error!("Failed fetchig player registrations {}", err);
            err
        })?)
    }
}
