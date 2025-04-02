use crate::Roads;

use super::super::Road;
use super::super::RoadWithNode;
use geo::closest_point::ClosestPoint;
use geo::{LineString, MultiLineString};

type Trajectory = LineString<f64>;

impl ClosestPoint<f64> for Road {
    fn closest_point(&self, p: &geo::Point<f64>) -> geo::Closest<f64> {
        self.geom.closest_point(p)
    }
}
impl ClosestPoint<f64> for Roads {
    fn closest_point(&self, p: &geo::Point<f64>) -> geo::Closest<f64> {
        let mls = MultiLineString::new(self.geom.clone()); // this is potentially quite bad
        mls.closest_point(p)
    }
}
impl ClosestPoint<f64> for RoadWithNode<'_> {
    fn closest_point(&self, p: &geo::Point<f64>) -> geo::Closest<f64> {
        self.road.closest_point(p)
    }
}

#[cfg(test)]
mod tests {
    use geo::{Closest, Point};
    use geo_types::line_string;

    use super::*;

    fn new_road(ls: LineString<f64>) -> Road {
        Road {
            id: 0,
            geom: ls,
            osm_id: 69,
            code: 42,
            direction: crate::Direction::Bidirectional,
            maxspeed: 2137,
            layer: 1,
            bridge: false,
            tunnel: false,
        }
    }

    #[test]
    fn road_identical_closest() {
        let ls = line_string![(x:1.0,y:2.0),(x:3.0,y:4.0),(x:5.0,y:6.0)];
        let road = new_road(ls.clone());
        let res = ls.0.iter().all(|p| {
            matches!(
                road.closest_point(&Point::from(*p)),
                Closest::Intersection(_)
            )
        });
        assert!(res)
    }
    #[test]
    fn road_indeterminate_closest() {
        let ls = line_string![(x:1.0,y:1.0),(x:1.0,y:2.0)];
        let road = new_road(line_string![(x:0.0,y:0.0),(x:2.0,y:0.0)]);
        let res =
            ls.0.iter()
                .all(|p| matches!(road.closest_point(&Point::from(*p)), Closest::Indeterminate));
    }
}
