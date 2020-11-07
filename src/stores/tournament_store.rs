use actix_web::{
    http::{self},
    ResponseError,
};

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use thiserror::Error;
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
/*
Actix will log these via the Debug trait and not the display string from the error attribute.
This means data in the error variants that's not part of the attribute string won't reach the
caller which means private info useful for debugging can be part of the variants.
*/
// TODO This error shouldn't be defined here, Invalid date is application error not database error
#[derive(Error, Debug)]
pub enum TournamentStoreError {
    #[error("Invalid start or end date")]
    InvalidDate,
    #[error("Internal Database error")]
    InternalDataBaseError(#[from] sqlx::Error),
}

impl ResponseError for TournamentStoreError {
    fn status_code(&self) -> http::StatusCode {
        match &self {
            TournamentStoreError::InvalidDate => http::StatusCode::BAD_REQUEST,
            TournamentStoreError::InternalDataBaseError(err) => {
                println!("{}", err);
                http::StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }
}

// Potentially wrap this into a TournamentStore trait that can have an In memory backend used for unit tests
// would also allow for implementation directly on PgPool instead of using wrapper structs.
#[derive(Clone)]
pub struct TournamentStore {
    pub pool: PgPool,
}

impl TournamentStore {
    pub async fn insert_tournament(
        &self,
        tournament: Tournament,
    ) -> Result<(), TournamentStoreError> {
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

    pub async fn get_tournaments(&self) -> Result<Vec<Tournament>, TournamentStoreError> {
        let tournaments = sqlx::query_as!(
            Tournament,
            "SELECT * FROM tournaments WHERE end_date >= CURRENT_DATE"
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(tournaments)
    }
}
