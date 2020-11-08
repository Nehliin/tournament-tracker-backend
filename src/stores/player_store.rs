#![allow(clippy::toplevel_ref_arg)]

use serde::{Deserialize, Serialize};
use sqlx::PgPool;
#[derive(Debug, Default, sqlx::FromRow, Serialize, Deserialize, Clone, PartialEq)]
pub struct Player {
    pub id: i64,
    pub name: String,
}

// Postgres store
pub struct PlayerStore {
    pub pool: PgPool,
}

impl PlayerStore {
    pub async fn insert_player(&self, player: &Player) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO players (id, name) VALUES ($1, $2)",
            player.id,
            player.name
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_player(&self, id: i64) -> Result<Option<Player>, sqlx::Error> {
        let player = sqlx::query_as!(Player, "SELECT * FROM players WHERE id = $1", id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(player)
    }
}

pub async fn get_or_insert_player(
    player: Player,
    storage: PlayerStore,
) -> Result<Player, sqlx::Error> {
    if let Some(player) = storage.get_player(player.id).await? {
        Ok(player)
    } else {
        storage.insert_player(&player).await?;
        Ok(player)
    }
}
