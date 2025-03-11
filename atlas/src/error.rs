use derive_more::derive::From;
use thiserror::Error;

#[non_exhaustive]
#[derive(From,Debug,Error)]
pub enum DbError{
    #[error("sqlx error: {0}")]
    Sqlx(sqlx::Error),

    #[error("expected \"B\", \"F\" or \"T\", got: {0}")]
    DirectionDecode(String),

    #[error("invalid linestring with id: {0}")]
    Linestring(u64),
    
}