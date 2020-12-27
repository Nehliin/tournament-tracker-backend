use crate::ServerError;
use async_trait::async_trait;
use bcrypt::{hash, DEFAULT_COST};
use chrono::{Local, NaiveDateTime};
use serde::Serialize;
use sqlx::PgPool;
use tracing::error;
use uuid::Uuid;

#[derive(Debug, PartialEq, sqlx::FromRow, Serialize)]
pub struct UserInfoRow {
    pub id: Uuid,
    pub email: String,
    // (hashed)
    pub password: String,
    pub created_at: NaiveDateTime,
}

#[async_trait]
pub trait UserStore {
    async fn insert_user(&self, email: String, password: String) -> Result<Uuid, ServerError>;
    async fn find_user(&self, email: &str) -> Option<UserInfoRow>;
    async fn get_user(&self, id: Uuid) -> Option<UserInfoRow>;
}

#[async_trait]
impl UserStore for PgPool {
    async fn insert_user(&self, email: String, password: String) -> Result<Uuid, ServerError> {
        let id = Uuid::new_v4();
        let created_at = Local::now().naive_local();
        let hashed_password = hash(password, DEFAULT_COST).map_err(|err| {
            error!("Failed to hash password {}", err);
            ServerError::InvalidPassword
        })?;
        let row = sqlx::query!("INSERT INTO users (id, email, password, created_at) VALUES ($1, $2, $3, $4) RETURNING id",
         id,
         email,
         hashed_password,
         created_at
        )
        .fetch_one(self)
        .await
        .map_err(|err| {
            error!("Failed to create user {}", err);
            err
        })?;
        Ok(row.id)
    }

    async fn find_user(&self, email: &str) -> Option<UserInfoRow> {
        sqlx::query_as!(
            UserInfoRow,
            "SELECT * FROM users WHERE email = $1",
            email
        )
        .fetch_optional(self)
        .await
        .map_err(|err| {
            error!("Find user query failed with error: {}", err);
            err
        })
        .ok()
        .flatten()
    }

    async fn get_user(&self, id: Uuid) -> Option<UserInfoRow> {
        sqlx::query_as!(
            UserInfoRow,
            "SELECT * FROM users WHERE id = $1",
            id
        )
        .fetch_optional(self)
        .await
        .map_err(|err| {
            error!("Failed to fetch user {}", err);
            err
        })
        .ok()
        .flatten()
    }
}
