//use geo_types::geometry::{LineString, Point};
//use rayon::prelude::*;
use rstar::primitives::GeomWithData;
use rstar::{PointDistance, RTree, RTreeObject};
//use std::collections::VecDeque;
//use rusty_roads::Road;




pub struct RoadIndex<T: RTreeObject + PointDistance + Clone> {
    pub index: RTree<GeomWithData<T, u64>>,
}

impl<T: RTreeObject + PointDistance + Clone> RoadIndex<T> {
    pub fn new(ids: &[u64], roads: &[T]) -> RoadIndex<T> {
        let geomdata: Vec<GeomWithData<T, u64>> = roads.iter().zip(ids.iter()).map(|(road, id)| GeomWithData::<T, u64>::new(road.clone(), *id)).collect();

        RoadIndex::<T> {
            index: RTree::<GeomWithData<T, u64>>::bulk_load(geomdata),
        }

    }
}
