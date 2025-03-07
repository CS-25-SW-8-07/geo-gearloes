use std::{error::Error, fmt::Display};
use derive_more::derive::{Display};
use thiserror::Error;

#[non_exhaustive]
#[derive(Debug,Error)]
pub enum DbError<'a>{
    #[error("sqlx error: {0}")]
    Sqlx(sqlx::Error),
    
    #[error("expected \"B\", \"F\" or \"T\", got: {0}")]
    DirectionDecode(&'a str),

    #[error("invalid linestring")]
    Linestring(&'a [u8]),
    
}