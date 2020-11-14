#![allow(clippy::toplevel_ref_arg)]

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

pub struct PlayerRegistrationStore {
    pub pool: PgPool,
}

impl PlayerRegistrationStore {
    #[tracing::instrument(name = "Inserting player registration", skip(self))]
    pub async fn insert_player_registration(
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
        ).execute(&self.pool).await
        .map_err(|err| {
            error!("Failed to register player {}", err);
            err
        })?;

        Ok(match_registration)
    }

    #[tracing::instrument(name = "Fetching player registration", skip(self))]
    pub async fn get_player_registration(
        &self,
        player_id: i64,
        match_id: i64,
    ) -> Result<Option<PlayerMatchRegistration>, sqlx::Error> {
        Ok(sqlx::query_as!(
            PlayerMatchRegistration,
            "SELECT * FROM register WHERE player_id = $1 AND match_id = $2",
            player_id,
            match_id,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|err| {
            error!("Failed get player registration {}", err);
            err
        })?)
    }
}
