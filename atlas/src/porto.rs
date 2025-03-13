use std::fmt::Write;
use std::{borrow::Cow, collections::HashSet};

use geo_types::{Line, LineString};
use sqlx::{pool::PoolConnection, Postgres};

use crate::wkb_to_linestring;

pub async fn map_match_porto(mut conn: PoolConnection<Postgres>) -> Result<Vec<i32>, sqlx::Error> {
    const CHUNK_SIZE: i32 = 10000;
    const _: () = assert!(CHUNK_SIZE > 0);
    let ids: Vec<(i32,(/*sqlx be trolling */))> = sqlx::query_as("select id from taxadata;")
        .fetch_all(&mut *conn)
        .await?;

    let ko = Cow::from(ids.into_iter().map(|e| e.0).collect::<Vec<_>>().as_slice());
    // let ids: HashSet<i32> = HashSet::from_iter(ids.into_iter().map(|e| e.0));
    let (ids, trajs): (Vec<i32>,Vec<LineString>) = get_trajectories(conn, CHUNK_SIZE, None).await?.into_iter().unzip();
    

    
    todo!()
}

async fn get_trajectories(
    mut conn: PoolConnection<Postgres>,
    chunk_size: i32,
    exclude: Option<&[i32]>,
) -> Result<Vec<(i32, LineString)>, sqlx::Error> {
    let exclude = exclude.unwrap_or(&[]);
    let where_clause = match exclude.len() {
        0 => "".into(),
        _ => {
            let not_in = exclude.iter().fold(String::new(), |mut acc, &id| {
                let _ = write!(acc, "{id},");
                acc
            });
            let not_in = &not_in[0..not_in.len() - 1];
            format!("where id not in ({not_in})")
        }
    };
    let sql = format!("select id, st_asbinary(trajectory::geometry,'NDR') {where_clause} limit {chunk_size}");

    let trajectories: Vec<(i32, Vec<u8>)> = sqlx::query_as(&sql).fetch_all(&mut *conn).await?;
    let trajectories: Vec<(i32, LineString)> = trajectories
        .into_iter()
        .filter_map(|(id, bytea)| {
            Some((id, wkb_to_linestring(&bytea)?)) //TODO: do something about invalid linestrings
        })
        .collect();
    Ok(trajectories)
}
