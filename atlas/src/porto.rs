// #![cfg_attr(test)]
#![cfg(test)]
use std::collections::HashMap;
use std::fmt::Write;
use std::{borrow::Cow, collections::HashSet};

use geo_types::{Line, LineString, Point};
use rstar::{primitives::GeomWithData, PointDistance};
use rusty_roads::{Id, NearestNeighbor};
use sqlx::Pool;
use sqlx::{pool::PoolConnection, Postgres};

use crate::wkb_to_linestring;
use location_obfuscation::*;

type Trajectory = (i32, LineString);

#[derive(Clone)]
struct Roads {
    ids: Vec<Id>,
    roads: Vec<LineString<f64>>,
}
impl NearestNeighbor<Point, LineString<f64>> for Roads {
    fn nearest_neighbor(&self, point: Point) -> Option<GeomWithData<LineString<f64>, u64>> {
        let data = self.ids.iter().zip(self.roads.iter()).fold(None, |acc, x| {
            let distance = x.1.points().fold(None, |acc: Option<f64>, element: Point| {
                Some(acc.map_or(point.distance_2(&element), |a| {
                    a.min(point.distance_2(&element))
                }))
            });
            if distance.is_none() {
                return acc;
            }

            if acc.is_none() {
                return Some((x.0, x.1, distance.unwrap()));
            }

            if distance.unwrap() < acc.unwrap().2 {
                return Some((x.0, x.1, distance.unwrap()));
            }

            acc
        });

        let something = data.map_or(None, |(id, line, _)| {
            Some(GeomWithData::<LineString<f64>, u64>::new(line.clone(), *id))
        });

        return something;
    }

    fn nearest_neighbor_road(&self, point: Point<f64>, id: Id) -> Option<Point> {
        self.roads[id as usize]
            .points()
            .fold(None, |acc, x| {
                if acc.is_none() {
                    return Some(x);
                }
                let distance = point.distance_2(&x);
                if point.distance_2(&acc.unwrap()) > distance {
                    return Some(x);
                } else {
                    return acc;
                }
            })
            .map(|x| geo_types::Point::from(x))
    }
}

pub async fn map_match_porto<T, F>(pool: Pool<Postgres>, f: F) -> Result<T, sqlx::Error>
where
    F: Fn(&[Trajectory]) -> T,
{
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

    // keep fetching chunks until every trajectory has been fetched
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
        received.extend(ids.iter().copied().zip(trajs.clone().into_iter()));
        let res = f(&ids.iter().copied().zip(trajs).collect::<Vec<Trajectory>>());
    }

    todo!()
}

async fn insert_matched_trajectories(
    mut conn: PoolConnection<Postgres>,
    trajs: &[Trajectory],
) -> Result<(), sqlx::Error> {
    let (ids,trajs): (Vec<i32>, Vec<Vec<u8>>) = trajs.into_iter().filter_map(|(id, ls)| {
        let mut buffer: Vec<u8> = vec![];
        let a =
            wkb::writer::write_line_string(&mut buffer, ls, wkb::Endianness::LittleEndian).ok()?;
        Some((*id, buffer))
    }).unzip();
    let sql = format!("insert into matched_taxa values ($1, $2)");
    let insert: () = sqlx::query_as(&sql).bind(ids).bind(trajs).fetch_one(&mut *conn).await?;
    Ok(insert) //TIHI
}

fn map_match(trajs: &[Trajectory], roadnetwork: Roads) -> Vec<Trajectory> {
    let matched = trajs
        .iter()
        .filter_map(|(id, traj)| {
            Some((
                *id,
                obfuscate_points(
                    traj.points().collect::<Vec<Point>>().into_iter(),
                    roadnetwork.clone(),
                )
                .ok()?,
            ))
        })
        .map(|(id, ps)| (id, LineString::from(ps)));
    matched.collect()
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
