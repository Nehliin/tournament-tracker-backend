#![allow(clippy::toplevel_ref_arg)]
use async_trait::async_trait;
use chrono::{Local, NaiveDateTime};
use serde::Serialize;
use sqlx::{Error, Executor, PgPool, Postgres, Transaction};
use tracing::{error, info};

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
// Court service?
#[async_trait]
pub trait CourtStore {
    async fn insert_tournament_court_allocation(
        self,
        tournament_court_allocation: TournamentCourtAllocation,
    ) -> Result<(), sqlx::Error>;

    async fn get_match_court(self, tournament_id: i32, match_id: i64) -> Option<String>;

    async fn try_assign_free_court(
        self,
        tournament_id: i32,
        match_id: i64,
    ) -> Result<String, sqlx::Error>;

    async fn remove_assigned_court(
        self,
        tournament_id: i32,
        match_id: i64,
    ) -> Result<String, sqlx::Error>;

    async fn append_court_queue(self, tournament_id: i32, match_id: i64)
        -> Result<(), sqlx::Error>;
    async fn get_court_queue_placement(
        self,
        tournament_id: i32,
        match_id: i64,
    ) -> Result<usize, sqlx::Error>;
}

async fn insert_tournament_court_allocation(
    executor: impl Executor<'_, Database = Postgres>,
    tournament_court_allocation: TournamentCourtAllocation,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
            "INSERT INTO tournament_court_allocation (court_name, tournament_id, match_id) VALUES ($1, $2, $3)",
            tournament_court_allocation.court_name,
            tournament_court_allocation.tournament_id,
            tournament_court_allocation.match_id,
        )
            .execute(executor)
            .await
            .map_err(|err| {
                error!("Failed to allocate court to tournament: {}", err);
                err
            })?;
    Ok(())
}

async fn get_match_court(
    executor: impl Executor<'_, Database = Postgres>,
    tournament_id: i32,
    match_id: i64,
) -> Option<String> {
    sqlx::query!("SELECT court_name FROM tournament_court_allocation WHERE tournament_id = $1 AND match_id = $2", 
            tournament_id,
            match_id
        )
            .fetch_optional(executor)
            .await
            .map_err(|err| {
                error!("Failed fetch match court: {}", err);
                err
            })
            .ok()
            .flatten()
            .map(|test| test.court_name)
}

async fn try_assign_free_court(
    executor: impl Executor<'_, Database = Postgres>,
    tournament_id: i32,
    match_id: i64,
) -> Result<String, sqlx::Error> {
    let row = sqlx::query!(
        "UPDATE tournament_court_allocation SET match_id = $1 \
                        WHERE tournament_id = $2 AND match_id IS NULL RETURNING court_name",
        match_id,
        tournament_id
    )
    .fetch_optional(executor)
    .await
    .map_err(|err| {
        error!("Failed assign free court: {}", err);
        err
    })?;
    if let Some(free_court) = row {
        Ok(free_court.court_name)
    } else {
        Err(sqlx::Error::RowNotFound)
    }
}

async fn remove_assigned_court(
    executor: impl Executor<'_, Database = Postgres>,
    tournament_id: i32,
    match_id: i64,
) -> Result<String, sqlx::Error> {
    let row = sqlx::query!(
        "UPDATE tournament_court_allocation SET match_id = NULL \
                        WHERE tournament_id = $2 AND match_id = $1 RETURNING court_name",
        match_id,
        tournament_id
    )
    .fetch_optional(executor)
    .await
    .map_err(|err| {
        error!("Failed assign free court: {}", err);
        err
    })?;
    if let Some(free_court) = row {
        Ok(free_court.court_name)
    } else {
        Err(sqlx::Error::RowNotFound)
    }
}

async fn append_court_queue(
    executor: impl Executor<'_, Database = Postgres>,
    tournament_id: i32,
    match_id: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO court_queue (place_in_queue, match_id, tournament_id) VALUES ($1, $2, $3)",
        Local::now().naive_local(),
        match_id,
        tournament_id
    )
    .execute(executor)
    .await
    .map_err(|err| {
        error!("Failed to append match to court queue");
        err
    })?;
    Ok(())
}

async fn get_court_queue_placement(
    executor: impl Executor<'_, Database = Postgres>,
    tournament_id: i32,
    match_id: i64,
) -> Result<usize, sqlx::Error> {
    // TODO: Count in the query itself instead of doing it in memory here, doesn't scale as well
    let queue_entries = sqlx::query!(
        "SELECT match_id FROM court_queue \
            WHERE tournament_id = $1 ORDER BY place_in_queue ASC LIMIT 100",
        tournament_id
    )
    .fetch_all(executor)
    .await
    .map_err(|err| {
        error!("Failed to fetch queue placement");
        err
    })?;

    if let Some(queue_index) = queue_entries
        .iter()
        .position(|rec| rec.match_id == match_id)
    {
        Ok(queue_index + 1)
    } else {
        error!("Match {} not found in court queue!", match_id);
        Err(sqlx::Error::RowNotFound)
    }
}

#[tracing::instrument(name = "Transactional Peek court queue", skip(executor))]
async fn peek_court_queue(
    executor: &mut Transaction<'_, Postgres>,
    tournament_id: i32,
) -> Result<Option<i64>, sqlx::Error> {
    if let Some(head_of_queue) = sqlx::query!(
        "SELECT match_id FROM court_queue WHERE \
                tournament_id = $1 ORDER BY place_in_queue ASC LIMIT 1",
        tournament_id
    )
    .fetch_optional(executor)
    .await
    .map_err(|err| {
        error!("Failed to fetch head of court queue");
        err
    })? {
        Ok(Some(head_of_queue.match_id))
    } else {
        info!("No court found in the court queue!");
        Ok(None)
    }
}

#[tracing::instrument(name = "Transactional Delete from court queue", skip(executor))]
async fn delete_from_court_queue(
    executor: &mut Transaction<'_, Postgres>,
    tournament_id: i32,
    match_id: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "DELETE FROM court_queue WHERE tournament_id = $1 AND match_id = $2",
        tournament_id,
        match_id
    )
    .execute(executor)
    .await
    .map_err(|err| {
        error!("Failed to delete head of court queue");
        err
    })?;
    Ok(())
}

#[tracing::instrument(name = "Transactional Popping of court queue", skip(executor))]
pub async fn pop_court_queue(
    executor: &mut Transaction<'_, Postgres>, // detta går nog att lösa med unsafe
    tournament_id: i32,
) -> Result<Option<i64>, Error> {
    match peek_court_queue(executor, tournament_id).await? {
        Some(match_id) => {
            delete_from_court_queue(executor, tournament_id, match_id).await?;
            Ok(Some(match_id))
        }
        None => Ok(None),
    }
}

#[async_trait]
impl CourtStore for &PgPool {
    #[tracing::instrument(name = "Inserting court -> tournament allocation", skip(self))]
    async fn insert_tournament_court_allocation(
        self,
        tournament_court_allocation: TournamentCourtAllocation,
    ) -> Result<(), sqlx::Error> {
        insert_tournament_court_allocation(self, tournament_court_allocation).await
    }

    #[tracing::instrument(name = "Fetching match court", skip(self))]
    async fn get_match_court(self, tournament_id: i32, match_id: i64) -> Option<String> {
        get_match_court(self, tournament_id, match_id).await
    }

    #[tracing::instrument(name = "Trying to assign free court to match", skip(self))]
    async fn try_assign_free_court(
        self,
        tournament_id: i32,
        match_id: i64,
    ) -> Result<String, sqlx::Error> {
        try_assign_free_court(self, tournament_id, match_id).await
    }

    #[tracing::instrument(name = "Removing assigned court", skip(self))]
    async fn remove_assigned_court(
        self,
        tournament_id: i32,
        match_id: i64,
    ) -> Result<String, sqlx::Error> {
        remove_assigned_court(self, tournament_id, match_id).await
    }

    #[tracing::instrument(name = "Appending match to court queue", skip(self))]
    async fn append_court_queue(
        self,
        tournament_id: i32,
        match_id: i64,
    ) -> Result<(), sqlx::Error> {
        append_court_queue(self, tournament_id, match_id).await
    }

    #[tracing::instrument(name = "Fetch court queue placement", skip(self))]
    async fn get_court_queue_placement(
        self,
        tournament_id: i32,
        match_id: i64,
    ) -> Result<usize, sqlx::Error> {
        get_court_queue_placement(self, tournament_id, match_id).await
    }
}

#[async_trait]
impl CourtStore for &mut Transaction<'_, Postgres> {
    #[tracing::instrument(
        name = "Transactional Inserting court -> tournament allocation",
        skip(self)
    )]
    async fn insert_tournament_court_allocation(
        self,
        tournament_court_allocation: TournamentCourtAllocation,
    ) -> Result<(), Error> {
        insert_tournament_court_allocation(self, tournament_court_allocation).await
    }

    #[tracing::instrument(name = "Transactional Fetching match court", skip(self))]
    async fn get_match_court(self, tournament_id: i32, match_id: i64) -> Option<String> {
        get_match_court(self, tournament_id, match_id).await
    }

    #[tracing::instrument(
        name = "Transactional Trying to assign free court to match",
        skip(self)
    )]
    async fn try_assign_free_court(
        self,
        tournament_id: i32,
        match_id: i64,
    ) -> Result<String, Error> {
        try_assign_free_court(self, tournament_id, match_id).await
    }

    #[tracing::instrument(name = "Transactional Removing assigned court", skip(self))]
    async fn remove_assigned_court(
        self,
        tournament_id: i32,
        match_id: i64,
    ) -> Result<String, Error> {
        remove_assigned_court(self, tournament_id, match_id).await
    }

    #[tracing::instrument(name = "Transactional Appending match to court queue", skip(self))]
    async fn append_court_queue(self, tournament_id: i32, match_id: i64) -> Result<(), Error> {
        append_court_queue(self, tournament_id, match_id).await
    }

    #[tracing::instrument(name = "Transactional Fetch court queue placement", skip(self))]
    async fn get_court_queue_placement(
        self,
        tournament_id: i32,
        match_id: i64,
    ) -> Result<usize, Error> {
        get_court_queue_placement(self, tournament_id, match_id).await
    }
}
