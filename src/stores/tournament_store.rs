#![allow(clippy::toplevel_ref_arg)]
use crate::ServerError;
use async_trait::async_trait;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tracing::error;
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone, Eq)]
pub struct Tournament {
    #[serde(default)]
    pub id: i32,
    pub name: String,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
}

impl PartialEq for Tournament {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

#[async_trait]
pub trait TournamentStore {
    async fn insert_tournament(&self, tournament: Tournament) -> Result<i32, ServerError>;
    async fn get_tournaments(&self) -> Result<Vec<Tournament>, ServerError>;
}

#[async_trait]
impl TournamentStore for PgPool {
    #[tracing::instrument(name = "Inserting new tournament", skip(self))]
    async fn insert_tournament(&self, tournament: Tournament) -> Result<i32, ServerError> {
        let row = sqlx::query!(
            "INSERT INTO tournaments (name, start_date, end_date) VALUES ($1, $2, $3)
            RETURNING id",
            tournament.name,
            tournament.start_date,
            tournament.end_date
        )
        .fetch_one(self)
        .await
        .map_err(|err| {
            error!("Failed to insert match {}", err);
            err
        })?;
        Ok(row.id)
    }

    #[tracing::instrument(name = "Fetching tournament list", skip(self))]
    async fn get_tournaments(&self) -> Result<Vec<Tournament>, ServerError> {
        let tournaments = sqlx::query_as!(
            Tournament,
            "SELECT * FROM tournaments WHERE end_date >= CURRENT_DATE"
        )
        .fetch_all(self)
        .await
        .map_err(|err| {
            error!("Failed to fetch matches {}", err);
            err
        })?;

        Ok(tournaments)
    }
}
