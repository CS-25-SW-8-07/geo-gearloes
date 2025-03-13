use std::{borrow::Cow, collections::HashSet};
use std::fmt::Write;

use geo_types::{Line, LineString};
use sqlx::{pool::PoolConnection, Postgres};

pub async fn map_match_porto(mut conn: PoolConnection<Postgres>) -> Result<Vec<i32>, sqlx::Error> {
    const CHUNK_SIZE: i32 = 10000;
    const _: () = assert!(CHUNK_SIZE > 0);
    let ids: Vec<(i32,(/*sqlx be trolling */))> = sqlx::query_as("select id from taxadata;")
        .fetch_all(&mut *conn)
        .await?;

    let ko = Cow::from(ids.into_iter().map(|e| e.0).collect::<Vec<_>>().as_slice());
    // let ids: HashSet<i32> = HashSet::from_iter(ids.into_iter().map(|e| e.0));

    todo!()
}

async fn get_trajectories(
    mut conn: PoolConnection<Postgres>,
    _ids: &[i32],
    exclude: Option<&[i32]>,
) -> Result<Vec<(i32, LineString)>, sqlx::Error> {

    let exclude = exclude.unwrap_or(&[]);
    let where_clause = match exclude.len() {
        0 => "".into(),
        _ => {
            let not_in = exclude.iter().fold(String::new(), |mut acc, &id|{
                let _ = write!(acc,"{id},");
                acc
            });
            let not_in = &not_in[0..not_in.len()-1];
            format!("where id not in ({not_in})")
        }
    };
    let sql = format!("select id, st_asbinary(trajectory::geometry,'NDR') {where_clause}");

    let trajectories: Vec<(i32,Vec<u8>)> = sqlx::query_as(&sql).fetch_all(&mut *conn).await?;
    // let trajectories: Vec<(i32,LineString)>  = trajecto
    todo!()
}
