use crate::RoadIndex;
use crate::Roads;

use super::super::Road;
use super::super::RoadWithNode;
use geo::closest_point::ClosestPoint;
use geo::Closest;
use geo::Point;
use geo::{LineString, MultiLineString};
use itertools::Itertools;

type Trajectory = LineString<f64>;

impl ClosestPoint<f64> for Road {
    fn closest_point(&self, p: &geo::Point<f64>) -> geo::Closest<f64> {
        // self.geom.lin
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

fn map_match_traj_to_road(traj: &Trajectory, road: impl ClosestPoint<f64>) -> Trajectory {
    let matched = traj
        .points()
        .map(|p| match road.closest_point(&p) {
            Closest::SinglePoint(s) => Ok(s),
            Closest::Intersection(i) => Ok(i),
            Closest::Indeterminate => Err(p), //TODO: special case to be handled, perhaps some sliding window magic
        })
        .collect::<Vec<_>>();
    // .filter_map(|r| r.ok());
    // let matched = matched.windows(3).map(|w|);
    // LineString::from(matched.collect::<Vec<_>>())
    todo!()
}
fn map_match_index(traj: &Trajectory, index: &RoadIndex) -> Vec<Result<Option<Point>, Point>> {
    let matched = traj
        .points()
        .map(|p| {
            let nn = index.index.nearest_neighbor(&p);
            match nn {
                Some(n) => match n.geom().closest_point(&p) {
                    Closest::SinglePoint(s) => Ok(Some(s)),
                    Closest::Intersection(i) => Ok(Some(i)),
                    Closest::Indeterminate => Err(p), //TODO: failed matches, if a linestring has length 1, this arm is matched
                },
                None => Ok(None),
            }
        })
        .collect::<Vec<Result<Option<Point>, Point>>>();

    // .flatten_ok()
    // .collect::<Vec<Result<_, _>>>();
    // .collect::<Result<Vec<_>, _>>()
    // .ok();

    // let matched = matched
    //     .windows(3)
    //     .map(|w| w[1])
    //     .collect::<Result<Vec<_>, _>>()
    //     .ok();
    // for (idx,ele) in matched.enumerate() {
    //     ele.map_or_else(|ok| ok, |err|matched[idx]);
    // }
    // matched.map(|ps| LineString::from(ps))
    matched
}

#[cfg(test)]
mod tests {
    use geo::{wkt, Closest, Point};
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

    #[test]
    fn rtree_nn() {
        let lss = [
            wkt! {LINESTRING(1.0 0.0, 2.0 0.0)},
            wkt! {LINESTRING(2.0 0.0, 3.0 0.0)},
            // wkt! {LINESTRING(3.0 0.0)},
        ];
        let ids = [1, 2, 3];
        let rtree = RoadIndex::from_ids_and_roads(&ids, &lss);
        let qp = wkt! {POINT(1.1 0.0)};
        let nn = rtree
            .index
            .nearest_neighbor(&qp)
            .map(|g| g.geom().closest_point(&qp));
        dbg!(nn);
        assert!(!matches!(nn, Some(Closest::Indeterminate)));
    }

    #[test]
    fn match_using_index() {
        let lss = [
            wkt! {LINESTRING(1.0 0.0, 2.0 0.0)},
            wkt! {LINESTRING(2.0 0.0, 3.0 0.0)},
            // wkt! {LINESTRING(3.0 0.01)},
        ];
        let ids = [1, 2, 3];
        let rtree = RoadIndex::from_ids_and_roads(&ids, &lss);
        let traj = wkt! {LINESTRING(1.0 0.4, 2.1 0.5, 3.2 -1.0)};

        let matched = map_match_index(&traj, &rtree);
        let is_ok = matched
            .iter()
            .enumerate()
            // .inspect(|(idx, e)| {
            //     if e.is_err() {
            //         dbg!((idx, e));
            //     }
            // })
            .all(|p| p.1.is_ok_and(|p| p.is_some()));
        dbg!(matched);
        assert!(is_ok);
    }
}
