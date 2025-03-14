use std::collections::HashMap;
use std::fmt::Write;
use std::{borrow::Cow, collections::HashSet};

use geo_types::{Line, LineString};
use sqlx::Pool;
use sqlx::{pool::PoolConnection, Postgres};

use crate::wkb_to_linestring;

type Trajectory = (i32,LineString);

pub async fn map_match_porto<T,F>(pool: Pool<Postgres>, f: F) -> Result<T, sqlx::Error>
where F: Fn(&[Trajectory]) -> T {
    const CHUNK_SIZE: i32 = 100000;
    const _: () = assert!(CHUNK_SIZE > 0);
    let ids: Vec<(i32,(/*sqlx be trolling */))> = sqlx::query_as("select id from taxadata;")
        .fetch_all(&pool)
        .await?;

    let ko = ids.into_iter().map(|e| e.0);
    // let ids: HashSet<i32> = HashSet::from_iter(ids.into_iter().map(|e| e.0));
    let mut received = HashMap::<i32, LineString>::with_capacity(ko.len());
    let all_ids: HashSet<i32> = HashSet::from_iter(ko);
    let mut keys: HashSet<i32> = HashSet::with_capacity(all_ids.len());
    // let mut ids_copy: Vec<i32> = Vec::with_capacity(CHUNK_SIZE as usize);
    while all_ids != keys {
        let (ids, trajs): (Vec<i32>, Vec<LineString>) =
            get_trajectories(pool.acquire().await?, CHUNK_SIZE, None)
                .await?
                .into_iter()
                .unzip();
        // ids_copy = ids.clone();
        // let ress = get_trajectories(pool.acquire().await?, CHUNK_SIZE, None)
        //     .await?;
        keys = keys
            .union(&HashSet::from_iter(ids.iter().copied()))
            .copied()
            .collect();
        received.extend(ids.into_iter().zip(trajs.into_iter()));


    }

    todo!()
}

async fn insert_matched_trajectories(trajs: &[Trajectory]) -> Result<(),sqlx::Error> {

    todo!()
}

fn map_match(trajs: &[Trajectory]) -> Vec<Trajectory> {

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
    let sql = format!(
        "select id, st_asbinary(trajectory::geometry,'NDR') {where_clause} limit {chunk_size}"
    );

    let trajectories: Vec<(i32, Vec<u8>)> = sqlx::query_as(&sql).fetch_all(&mut *conn).await?;
    let trajectories: Vec<(i32, LineString)> = trajectories
        .into_iter()
        .filter_map(|(id, bytea)| {
            Some((id, wkb_to_linestring(&bytea)?)) //TODO: do something about invalid linestrings
        })
        .collect();
    Ok(trajectories)
}
