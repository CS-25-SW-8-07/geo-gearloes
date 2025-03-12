use geo_types::geometry::{Point};
//use rayon::prelude::*;
use rstar::primitives::GeomWithData;
use rstar::{PointDistance, RTreeObject};
use rusty_roads::{Id, NearestNeighbor, Queryable, RoadKey};
use std::collections::VecDeque;
use thiserror::Error;

const WINDOW_SIZE: usize = 5; // Must be an odd number.

const _: () = assert!(WINDOW_SIZE%2==1,"`WINDOW_SIZE` must be odd");

#[derive(Error, Debug)]
pub enum LocationObfuscationError {
    #[error("Cannot do obfuscation when no points are provided")]
    NoPointsProvided,
}

pub fn obfuscate_points<T, U, V>(points: T, roads: V) -> Result<Vec<Point>, LocationObfuscationError>
where
    T: Iterator<Item = U> + Clone,
    U: PointDistance + RTreeObject + Clone,
    V: NearestNeighbor<U, GeomWithData<U, Id>> + Queryable<RoadKey>,
{
    let mut ids: VecDeque<Id> = points.clone()
        .filter_map(|x| roads.nearest_neighbor(x))
        .map(|x| x.data)
        .collect();


    let first = *ids.front().ok_or(LocationObfuscationError::NoPointsProvided)?;
    let last = *ids.back().ok_or(LocationObfuscationError::NoPointsProvided)?;

    for _ in 0..WINDOW_SIZE / 2 {
        ids.push_front(first);
        ids.push_back(last);
    }

    let freq_ids = ids
        .make_contiguous()
        .windows(WINDOW_SIZE)
        .filter_map(|x| {
            x.iter()
                .map(|y| (y, x.iter().filter(|z| *z == y).count()))
                .reduce(|acc, x| if acc.1 < x.1 { x } else { acc })
        }).map(|x| x.0);



    Ok(points.zip(freq_ids).filter_map(|(point, id)|
        roads.nearest_neighbor_road(point, *id)
    ).collect())
}

// #[cfg(test)]
// mod tests {
//     use super::*;

// }
