#![allow(clippy::toplevel_ref_arg)]

use chrono::{Local, NaiveDateTime};
use serde::Serialize;
use sqlx::PgPool;

#[derive(Debug, sqlx::FromRow, Serialize)]
pub struct CourtQueueEntry {
    pub place_in_queue: NaiveDateTime,
    pub match_id: i64,
    pub tournament_id: i32,
}

#[derive(Debug, sqlx::FromRow, Serialize)]
pub struct TournamentCourtAllocation {
    pub court_name: String,
    pub tournament_id: i32,
    pub match_id: Option<i64>,
}

pub struct CourtStore {
    pub pool: PgPool,
}

impl CourtStore {
    pub async fn insert_tournament_court_allocation(
        &self,
        tournament_court_allocation: TournamentCourtAllocation,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO tournament_court_allocation (court_name, tournament_id, match_id) VALUES ($1, $2, $3)",
            tournament_court_allocation.court_name,
            tournament_court_allocation.tournament_id,
            tournament_court_allocation.match_id,
            )
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn assign_free_court(
        &self,
        _tournament_id: i32,
        _match_id: i64,
    ) -> Result<(), sqlx::Error> {
        todo!()
    }

    pub async fn append_court_queue(
        &self,
        tournament_id: i32,
        match_id: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO court_queue (place_in_queue, match_id, tournament_id) VALUES ($1, $2, $3)",
            Local::now().naive_local(),
            match_id,
            tournament_id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_court_queue_placement(
        &self,
        tournament_id: i32,
        match_id: i64,
    ) -> Result<usize, sqlx::Error> {
        let queue_entries = sqlx::query!(
            "SELECT match_id FROM court_queue WHERE tournament_id = $1 ORDER BY place_in_queue ASC",
            tournament_id
        )
        .fetch_all(&self.pool)
        .await?;

        if let Some(queue_index) = queue_entries.iter().position(|rec| rec.match_id == match_id) {
            Ok(queue_index + 1)
        } else {
            Err(sqlx::Error::RowNotFound)
        }
    }

    pub async fn pop_court_queue(&self, _tournament_id: i32) -> Result<i64, sqlx::Error> {
        todo!()
    }
}

