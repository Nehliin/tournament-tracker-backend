#![allow(clippy::toplevel_ref_arg)]

use crate::ServerError;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
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

// Potentially wrap this into a TournamentStore trait that can have an In memory backend used for unit tests
// would also allow for implementation directly on PgPool instead of using wrapper structs.
#[derive(Clone)]
pub struct TournamentStore {
    pub pool: PgPool,
}

impl TournamentStore {
    pub async fn insert_tournament(&self, tournament: Tournament) -> Result<(), ServerError> {
        sqlx::query!(
            "INSERT INTO tournaments (name, start_date, end_date) VALUES ($1, $2, $3)",
            tournament.name,
            tournament.start_date,
            tournament.end_date
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_tournaments(&self) -> Result<Vec<Tournament>, ServerError> {
        let tournaments = sqlx::query_as!(
            Tournament,
            "SELECT * FROM tournaments WHERE end_date >= CURRENT_DATE"
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(tournaments)
    }
}
