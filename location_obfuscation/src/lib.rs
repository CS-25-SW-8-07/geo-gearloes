pub mod anonymity;

use geo_types::geometry::Point;
//use rayon::prelude::*;
use rstar::{PointDistance, RTreeObject};
use rusty_roads::{Id, NearestNeighbor};
use std::collections::VecDeque;
use thiserror::Error;

const WINDOW_SIZE: usize = 5; // Must be an odd number.

const _: () = assert!(WINDOW_SIZE % 2 == 1, "`WINDOW_SIZE` must be odd");

#[derive(Error, Debug, PartialEq)]
pub enum LocationObfuscationError {
    #[error("Cannot do obfuscation when no points are provided")]
    NoPointsProvided,
}

pub fn obfuscate_points<T, U, W, V>(
    points: T,
    roads: V,
) -> Result<Vec<Point>, LocationObfuscationError>
where
    T: Iterator<Item = U> + Clone,
    U: PointDistance + RTreeObject + Clone,
    W: PointDistance + RTreeObject + Clone,
    V: NearestNeighbor<U, W>,
{
    let mut ids: VecDeque<Id> = points
        .clone()
        .filter_map(|x| roads.nearest_neighbor(x))
        .map(|x| x.data)
        .collect();

    let first = *ids
        .front()
        .ok_or(LocationObfuscationError::NoPointsProvided)?;
    let last = *ids
        .back()
        .ok_or(LocationObfuscationError::NoPointsProvided)?;

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
        })
        .map(|x| x.0);

    Ok(points
        .zip(freq_ids)
        .filter_map(|(point, id)| roads.nearest_neighbor_road(point.clone(), *id))
        .collect())
}

#[cfg(test)]
mod tests {
    use std::cmp::min_by;

    use super::*;
    use geo_types::geometry::LineString;
    use geo_types::{Coord, Point, coord, line_string, point};
    use rstar::primitives::GeomWithData;

    struct Roads {
        ids: Vec<Id>,
        roads: Vec<LineString<f64>>,
    }

    impl Roads {
        fn new() -> Self {
            let linestring1 = line_string![
                (x:0.,y:0.),
                (x:2.,y:2.),
                (x:4.,y:4.),
            ];
            let linestring2 = line_string![
                (x:2.,y:2.),
                (x:2.,y:1.),
                (x:1.,y:1.),
            ];
            let linestring3 = line_string![
                (x:5.,y:4.),
                (x:5.,y:2.),
                (x:4.,y:2.),
            ];
            Self {
                ids: vec![0, 1, 2],
                roads: vec![linestring1, linestring2, linestring3],
            }
        }
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

    #[test]
    fn map_point_to_road() {
        let road_network = Roads::new();

        let points = vec![point! { x: 1.5, y: 2.3 }];

        let points = obfuscate_points(points.into_iter(), road_network).unwrap();

        assert_eq!(point! { x: 2.0 , y: 2.0 }, points[0]);
    }

    #[test]
    fn map_no_point_to_road() {
        let road_network = Roads::new();

        let points: Vec<Point> = vec![];

        let points = obfuscate_points(points.into_iter(), road_network);

        assert!(points.is_err_and(|x| x == LocationObfuscationError::NoPointsProvided))
    }
}
