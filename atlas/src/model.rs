use sqlx::{pool::PoolConnection, Postgres};

use rusty_roads::burn::Model;

use crate::error::DbError;

pub async fn fetch_model(mut conn: PoolConnection<Postgres>) -> Result<Vec<u8>, DbError> {
    let query: String = "SELECT model FROM model LIMIT 1".into();

    let data: (Vec<u8>,) = sqlx::query_as(&query).fetch_one(&mut *conn).await?;

    Ok(data.0)
}

pub async fn update_model(
    mut conn: PoolConnection<Postgres>,
    model: Vec<u8>,
) -> Result<(), DbError> {
    let query: String = "UPDATE model SET model=$1 WHERE id = 1".into();

    sqlx::query(&query)
        .bind(model)
        .execute(&mut *conn)
        .await
        .map_err(DbError::Sqlx)?;

    Ok(())
}


