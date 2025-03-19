mod road_index;
pub use road_index::*;
use rstar::{primitives::GeomWithData, PointDistance, RTreeObject};

use crate::Id;
use geo_types::Point;

pub trait NearestNeighbor<T, U>
where
    T: RTreeObject + PointDistance,
    U: RTreeObject + PointDistance,
{
    fn nearest_neighbor(&self, point: T) -> Option<GeomWithData<U, Id>>;
    fn nearest_neighbor_road(&self, point: T, id: Id) -> Option<Point>;
}
