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
    use std::fs;
    use std::path::PathBuf;

    use geo::line_measures::FrechetDistance;
    use geo::{wkt, Closest, Coord, Euclidean, Point};
    use geo_traits::{LineStringTrait, MultiLineStringTrait};
    use geo_types::line_string;

    use super::*;

    const TRAJ_277_NEARBY: &str = include_str!("../../resources/277_nearby_roads.txt"); //? might not be windows compatible
    const TRAJ_277: &str = include_str!("../../resources/277_traj.txt"); //? might not be windows compatible

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

    fn add_noise(ls: &Trajectory, noise: f64) -> Trajectory {
        let a = ls
            .points()
            .map(|Point(Coord { x, y })| Point::new(x + noise, y - noise));
        LineString::from_iter(a)
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

    #[test]
    #[ignore]
    fn noisy_trajectory_match() {
        // id 277 in porto taxa
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("resources/");
        dbg!(&d);
        let traj_orig: Trajectory = wkt::TryFromWkt::try_from_wkt_str(
            &fs::read_to_string(d.clone().join("277_traj.txt"))
                .expect("there should be a file here"),
        )
        .unwrap();
        let noisy = add_noise(&traj_orig, 0.0005);
        let file = fs::read_to_string(d.join("277_nearby_roads.txt").to_str().unwrap())
            .expect("there should be a file here");

        let network: MultiLineString<f64> = wkt::TryFromWkt::try_from_wkt_str(&file).unwrap();
        let (id, ls): (Vec<u64>, Vec<_>) = network
            .line_strings()
            .enumerate()
            .map(|(id, traj)| (id as u64, traj.clone()))
            .unzip();

        let rtree = RoadIndex::from_ids_and_roads(&id, &ls);
        let matched = map_match_index(&noisy, &rtree);
        dbg!(&matched);
        assert!(matched.iter().all(|p| p.is_ok_and(|pp| pp.is_some())));
        let matched = matched
            .into_iter()
            .flatten_ok()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        let traj = LineString::from(matched);
        let mut buf = String::new();
        let _ = wkt::to_wkt::write_linestring(&mut buf, &traj).unwrap();
        dbg!(&buf);
        assert_eq!(
            traj_orig.0.len(),
            traj.0.len(),
            "original and matched trajectory should have cardinality"
        );
    }

    #[test]
    fn match_with_varying_noise() {
        const NOISE: f64 = f64::EPSILON;
        let traj_orig: Trajectory = wkt::TryFromWkt::try_from_wkt_str(TRAJ_277).unwrap();
        let network: MultiLineString<f64> =
            wkt::TryFromWkt::try_from_wkt_str(&TRAJ_277_NEARBY).unwrap();
        let (id, ls): (Vec<u64>, Vec<_>) = network
            .line_strings()
            .enumerate()
            .map(|(id, traj)| (id as u64, traj.clone()))
            .unzip();

        let rtree = RoadIndex::from_ids_and_roads(&id, &ls);

        let matched = (1..10).map(|f| {
            let noisy = add_noise(&traj_orig, NOISE * f as f64*1000.0);
            let matched = map_match_index(&noisy, &rtree)
                .into_iter()
                .flatten_ok()
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            let matched = LineString::from(matched);
            let frechet_dist = Euclidean.frechet_distance(&traj_orig, &matched);
            (frechet_dist,matched)
        }).zip(0..);
        for ele in matched {
            println!("dist: {}\tnoise:{}",ele.0.0,ele.1 as f64 * NOISE);
        }
    }

    #[test]
    fn match_noisy_traj() {
        let traj_orig: Trajectory = wkt::TryFromWkt::try_from_wkt_str(TRAJ_277).unwrap();
        let noisy = add_noise(&traj_orig, 0.0005);
        let network: MultiLineString<f64> =
            wkt::TryFromWkt::try_from_wkt_str(&TRAJ_277_NEARBY).unwrap();
        let (id, ls): (Vec<u64>, Vec<_>) = network
            .line_strings()
            .enumerate()
            .map(|(id, traj)| (id as u64, traj.clone()))
            .unzip();

        let rtree = RoadIndex::from_ids_and_roads(&id, &ls);
        let matched = map_match_index(&noisy, &rtree);
        dbg!(&matched);
        assert!(matched.iter().all(|p| p.is_ok_and(|pp| pp.is_some())));
        let matched = matched
            .into_iter()
            .flatten_ok()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        let traj = LineString::from(matched);
        let mut buf = String::new();
        let _ = wkt::to_wkt::write_linestring(&mut buf, &traj).unwrap();
        dbg!(&buf);

        assert_eq!(
            traj_orig.0.len(),
            traj.0.len(),
            "original and matched trajectory should have same cardinality"
        );
    }
}
