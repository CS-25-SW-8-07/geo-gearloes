use crate::RoadIndex;
use crate::Roads;

use super::super::Road;
use super::super::RoadWithNode;
use geo::closest_point::ClosestPoint;
use geo::Closest;
use geo::Distance;
use geo::Euclidean;
use geo::Point;
use geo::{LineString, MultiLineString};
use itertools::Itertools;
use rstar::primitives::GeomWithData;

type Trajectory = LineString<f64>;
type ADDDD<'a> = (Point, (Point, &'a GeomWithData<LineString<f64>, u64>));

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
/// an implementation that handles some cases listed in [this paper](https://www.sciencedirect.com/science/article/pii/S0968090X00000267)
fn map_match_index_v2(traj: &Trajectory, index: &RoadIndex) -> Result<Vec<Point>, Point> {
    debug_assert!(
        index.index.iter().all(|p| p.geom().0.len() > 1),
        "Linestrings with only one point are not allowed"
    ); // might be checked my `geo_types``
    debug_assert!(
        index.index.size() > 1,
        "index must be non-empty (and have more than 1 element to simplify return type)"
    );

    // same as `map_match_index` but should handle `Closest::Indeterminate` better
    let matched = traj
        .points()
        .map(|p| {
            let mut inn = index.index.nearest_neighbor_iter(&p);
            let first_nn = inn.next().unwrap();
            // .map(|g| g.geom())
            // .expect("rtree should have 1 element");
            // let second = inn.next().map(|g| g.geom()).expect("rtree should have at least 2 elements");
            // let min_dist = Euclidean.distance(first_nn, &p);
            // let candidates = inn.take_while(|pred| Euclidean.distance(pred.geom(), &p) <= min_dist);
            // in most cases, this is probably empty
            (closest(p, first_nn.geom()), first_nn)
        })
        .map(|r| match r.0 {
            Ok(p) => (p.0, (p.1, r.1)),
            Err(p) => (p, (p, r.1)), // ! handle error points better
        })
        .collect::<Vec<_>>();

    // for (idx, ele) in matched.iter().enumerate().skip(1) {
    //     let [prev, mid, next] = [matched[idx - 1], matched[idx], matched[idx]]
    //         .map(|(p, _)| index.index.nearest_neighbor(&p).unwrap());

    //     // figure 6 in paper (oscillating match )
    //     if prev == next && prev != mid {
    //         // matched[idx].0 = prev.geom().closest_point(&matched[idx].1);
    //         matched[idx].0 = match closest(matched[idx].1, prev.geom()).map(|(p, _)| p) {
    //             Ok(p) => p,
    //             Err(p) => p,
    //         };
    //     }
    // }

    todo!()
}

fn perpendicular_case(
    points: &[ADDDD],
    rtree: &RoadIndex,
    window_size: usize,
) -> Vec<(Point, Point)> {
    debug_assert!(window_size >= 3);
    let res = points.windows(window_size).map(|s| {
        match s {
            [start @ .., last] => {
                if start.iter().map(|f| f.1 .1).all_equal() { // if all in start is matched to same road, then last should be as well (if direction is equal)

                }
            }
            _ => unreachable!(),
        }
    });
    // for (fst, mid, lst) in points.iter().tuple_windows() {
    //     // if fst.1.1 == snd.1.1
    // }

    todo!()
}

// attemtps to re-match points if oscillations are detected
fn oscillating_case(points: &[(Point, Point)], rtree: &RoadIndex) -> Vec<(Point, Point)> {
    debug_assert!(rtree.index.size() > 1);

    let iter = points.iter().tuple_windows().map(|(fst, snd, thd)| {
        let [prev, mid, next] = [fst, snd, thd].map(|f| {
            rtree
                .index
                .nearest_neighbor(&f.0)
                .expect("rtree should be nonemtpy")
        });

        let new_match = match mid {
            g if prev == next && prev != g => f(closest(snd.1, prev.geom())),
            _ => *snd,
            // g if prev != next => {},
        };
        (*fst, new_match, *thd)
    });
    let (first, second, third) = iter
        .clone()
        .take(1)
        .exactly_one()
        .expect("there should be atleast one element in the iterator");
    let resulting = [first, second, third]
        .into_iter()
        .chain(iter.skip(1).map(|(_, _, last)| last));
    resulting.collect()
}

fn closest(p: Point, first_nn: &LineString) -> Result<(Point, Point), Point> {
    match first_nn.closest_point(&p) {
        Closest::SinglePoint(s) => Ok((s, p)),
        Closest::Intersection(i) => Ok((i, p)),
        Closest::Indeterminate => Err(p),
    }
}

fn f(r: Result<(Point, Point), Point>) -> (Point, Point) {
    match r {
        Ok(p) => p,
        Err(e) => (e, e),
    }
}

#[deprecated]
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
        const NOISE: f64 = 0.00005 * 100.0;
        let traj_orig: Trajectory = wkt::TryFromWkt::try_from_wkt_str(TRAJ_277).unwrap();
        let network: MultiLineString<f64> =
            wkt::TryFromWkt::try_from_wkt_str(&TRAJ_277_NEARBY).unwrap();
        let (id, ls): (Vec<u64>, Vec<_>) = network
            .line_strings()
            .enumerate()
            .map(|(id, traj)| (id as u64, traj.clone()))
            .unzip();

        let rtree = RoadIndex::from_ids_and_roads(&id, &ls);

        let matched = (1..10)
            .map(|f| {
                let noisy = add_noise(&traj_orig, NOISE * f as f64);
                let matched = map_match_index(&noisy, &rtree)
                    .into_iter()
                    .flatten_ok()
                    .collect::<Result<Vec<_>, _>>()
                    .unwrap();
                let matched = LineString::from(matched);
                let frechet_dist = Euclidean.frechet_distance(&traj_orig, &matched);
                (frechet_dist, matched)
            })
            .zip(0..);
        for ele in matched.clone() {
            println!("dist: {}\tnoise:{}", ele.0 .0, ele.1 as f64 * NOISE); // use --show-output to show this
        }
        matched.collect::<Vec<_>>().windows(2).for_each(|e| {
            assert!(
                e[0].0 .0 < e[1].0 .0,
                "frechet distance should be smaller with a lower noise level"
            )
        });
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
