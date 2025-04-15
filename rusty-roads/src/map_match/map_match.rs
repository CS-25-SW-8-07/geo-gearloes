use std::iter;
use std::iter::repeat;

use crate::RoadIndex;
use crate::Roads;

use super::super::Road;
use super::super::RoadWithNode;
use geo::closest_point::ClosestPoint;
use geo::line_measures::LengthMeasurable;
use geo::polygon;
use geo::wkt;
use geo::Closest;
use geo::Distance;
use geo::Euclidean;
use geo::GeodesicArea;
use geo::Length;
use geo::Point;
use geo::Polygon;
use geo::RemoveRepeatedPoints;
use geo::{Line, LineString, MultiLineString};
use geo_traits::PointTrait;
use itertools::put_back;
use itertools::put_back_n;
use itertools::Itertools;
use rstar::primitives::GeomWithData;
use rstar::PointDistance;

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
        .collect::<Result<Vec<_>, _>>();
    LineString::from(matched.expect("all points should be matched"))
    // todo!()
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
            (closest(&p, first_nn.geom()), first_nn)
        })
        .map(|r| match r.0 {
            Ok(p) => (p.0, (p.1, r.1)),
            Err(p) => (p, (p, r.1)), // ! handle error points better
        })
        .collect::<Vec<_>>();

    todo!()
}

/// Compares direction of 2 lines
/// returns a number between 0 and 2 (inclusive) where 0 means their slope is similar and 2 means they are opposite (sqrt(2) meaning a perfect right angle)
fn line_similarity(fst: &Line, snd: &Line) -> f64 {
    let fst = Line::new(
        fst.start / Euclidean.length(fst),
        fst.end / Euclidean.length(fst),
    );
    let snd = Line::new(
        snd.start / Euclidean.length(snd),
        snd.end / Euclidean.length(snd),
    );
    let res = Line::new(fst.start - snd.start, fst.end - snd.end);
    let length = match Euclidean.length(&res) {
        l if l.is_normal() => l,
        _ => 0.,
    };
    debug_assert!((0.0..=2.0).contains(&length), "{length}");
    length
}

// indicates what part of a trajectory should be matched to a singular road
fn when_to_skip(idx: usize, traj: &Trajectory, _index: &RoadIndex) -> usize {
    const SIMILARITY_THRESHOLD: f64 = 1.0; //? perhaps it should be an input parameter

    // const MIN_OFFSET: usize = 4;
    // let mut to: usize = idx + MIN_OFFSET;
    // let a = traj.points().skip(idx).take(MIN_OFFSET).map(|p|)
    let a = traj
        .lines() //Note: this iterator yields 1 less element compared to .points()
        .enumerate()
        .skip(idx)
        .tuple_windows()
        .map(|(sl, el)| (sl.0, line_similarity(&sl.1, &el.1)))
        .take_while(|(i, e)| *e < 1.0)
        .map(|(e, _)| e);
    // dbg!(a.count());
    let res = a.last();
    // .unwrap_or(idx);
    res.unwrap_or(idx)
}

fn segment_match<I>(sub_traj: I, index: &RoadIndex) -> Result<Vec<Line>, (usize, Line)>
where
    I: Iterator<Item = Line>,
{
    const MAX_CANDIDATES: usize = 20;

    let matched = sub_traj.enumerate().map(|(idx, l)| {
        let candidate_roads_start = index
            .index
            .nearest_neighbor_iter_with_distance_2(&l.start_point())
            .take(MAX_CANDIDATES);
        let candidate_roads_end = index
            .index
            .nearest_neighbor_iter_with_distance_2(&l.end_point())
            .take(MAX_CANDIDATES);

        let all_candidates = candidate_roads_start.chain(candidate_roads_end);

        let (best, best_poly) = all_candidates
            .filter_map(|(g, _dist_2)| {
                let (closest_start, _) = closest(&l.start_point(), &g.geom()).ok()?;
                let closest_start = closest_start.coord().expect("should be infallible");
                let (closest_end, _) = closest(&l.end_point(), &g.geom()).ok()?; // Note: if every candidate causes a None value here, the matched trajectory will have smaller cardinality
                let closest_end = closest_end.coord().expect("should be infallible");
                // let poly = polygon!(l.start, l.end, *closest_end, *closest_start, /*l.start*/);
                // dbg!(poly.remove_repeated_points().exterior().0.len());
                let poly = if closest_start != closest_end {
                    // if start and end matches to same point, a bad match will occur
                    Some(polygon!(
                        l.start,
                        l.end,
                        *closest_end,
                        *closest_start,
                        // l.start
                    )) // FIXME i think first point must be repeated to close the polygon
                } else {
                    None
                }?;
                // dbg!(idx);
                Some((g, poly))
            })
            // .inspect(|f| {dbg!((idx,f));})
            .min_by(|(_, fst), (_, snd)| {
                fst.geodesic_area_signed()
                    .abs()
                    .total_cmp(&snd.geodesic_area_signed().abs())
            }) // this will not work correctly for very large polygons
            .expect("there should be at least one candidate"); // this is not guaranteed
        let start_matched = closest(&l.start_point(), &best.geom());
        let end_matched = closest(&l.end_point(), &best.geom());
        let result = start_matched
            .and_then(|fp| end_matched.and_then(|sp| Ok((fp, sp))))
            .map_err(|_| (idx, l));
        result
    });

    let result: Result<Vec<_>, (usize, Line)> =
        matched.map_ok(|(a, b)| Line::new(a.0, b.0)).try_collect();
    // .try_fold(vec![], |mut acc,r|r.map(|(a,b) | acc.push(a)) );
    result
}

fn best_road_new<I>(sub_traj: I, index: &RoadIndex) -> Vec<Point>
where
    I: Iterator<Item = Point>,
{
    const MAX_CANDIDATES: usize = 5;
    let mut sub_traj = put_back_n(sub_traj);
    let qp = sub_traj.next(); //TODO instead of picking road with least distance to point, use line segment instead (segment to segment match)

    // .expect("trajectory should be nonempty");
    qp.map(|p| {
        let candidate_roads = index
            .index
            .nearest_neighbor_iter_with_distance_2(&p)
            .take(MAX_CANDIDATES);

        // assert!(sub_traj.put_back(qp).is_none());
        // .expect("put back slot should be empty");
        sub_traj.put_back(p);
        let ls = LineString::from_iter(sub_traj);
        let res = candidate_roads.map(|(geom, dist)| {
            let dist_squared: f64 = ls
                .points()
                .take(geom.geom().0.len()) //? not necessarily a good way of handling this
                .zip(repeat(geom.geom()))
                .map(|(p, ls)| Euclidean.distance(ls, &p).powi(2))
                .sum();
            (geom, dist_squared)
        });
        let best = res
            .min_by(|x, y| x.1.total_cmp(&y.1))
            .expect("candidate set should be nonempty");

        map_match_traj_to_road(&ls, best.0.geom())
            .points()
            .collect_vec()
    })
    .unwrap_or(vec![])
}

fn best(traj: &Trajectory, index: &RoadIndex) -> Trajectory {
    let mut idx = 0;
    let mut matched: Vec<Point> = Vec::with_capacity(traj.0.len());

    while idx < traj.0.len() {
        let count = when_to_skip(idx + 1, traj, index);
        let points = traj.points().skip(idx).take(count - idx);
        // dbg!(traj.points().skip(idx).take(count).count());
        // dbg!((count, idx,traj.0.len()));
        idx = count;
        // dbg!(traj.points().count());
        // dbg!(traj.points().skip(idx).take(count).take(count).next());
        matched.extend(best_road_new(points, index));
        // dbg!(matched.len());
    }
    debug_assert_eq!(
        traj.0.len(),
        matched.len(),
        "matched trajectory should have same cardinality as input\n\t |input|={} |matched|={}",
        traj.0.len(),
        matched.len()
    );
    LineString::from(matched)
}

// this only considers the first road near the trajectory's start
#[deprecated = "use `best` instead"]
fn best_road(traj: &Trajectory, index: &RoadIndex) -> Vec<Point> {
    const MAX_CANDIDATES: usize = 5;

    // let mut idx = 0;
    // let mut matched: Vec<Point> = Vec::with_capacity(traj.0.len());

    // while idx <= traj.0.len() {
    //     let count = when_to_skip(idx, traj, index);
    //     idx = count;
    //     let mut points = traj.points().skip(idx).take(count);

    //     // matched.extend(todo!());
    // }

    let candidate_roads = index
        .index
        .nearest_neighbor_iter_with_distance_2(&Point::from(
            *traj.0.first().expect("trajectory should be nonempty"),
        ))
        .take(MAX_CANDIDATES);

    let res = candidate_roads.map(|(geom, dist)| {
        let dist_squared: f64 = traj
            .points()
            .take(geom.geom().0.len()) //? not necessarily a good way of handling this
            .zip(repeat(geom.geom()))
            .map(|(p, ls)| Euclidean.distance(ls, &p).powi(2))
            .sum();
        (geom, dist_squared)
    });
    let best = res
        .min_by(|x, y| x.1.total_cmp(&y.1))
        .expect("candidate set should be nonempty");
    map_match_traj_to_road(&traj, best.0.geom())
        .points()
        .collect()
    // todo!()
}

#[deprecated]
fn perpendicular_case<'a>(
    points: &'a [ADDDD],
    rtree: &RoadIndex,
    window_size: usize,
) -> Vec<ADDDD<'a>> {
    debug_assert!(window_size >= 3);
    let res = points.windows(window_size).map(|s| {
        match s {
            [start @ .., last] => {
                if start.iter().map(|f| f.1 .1).all_equal() {
                    // if all in start is matched to same road, then last should be as well (if direction is equal)
                    todo!()
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
            g if prev == next && prev != g => f(closest(&snd.1, prev.geom())),
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

fn closest(p: &Point, first_nn: &LineString) -> Result<(Point, Point), Point> {
    match first_nn.closest_point(&p) {
        Closest::SinglePoint(s) => Ok((s, *p)),
        Closest::Intersection(i) => Ok((i, *p)),
        Closest::Indeterminate => Err(*p),
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
    use geo::{coord, wkt, Closest, Coord, Euclidean, Point, Within};
    use geo_traits::{LineStringTrait, MultiLineStringTrait};
    use geo_types::line_string;

    use super::*;

    const TRAJ_277_NEARBY: &str = include_str!("../../resources/277_nearby_roads.txt"); //? might not be windows compatible
    const TRAJ_277: &str = include_str!("../../resources/277_traj.txt"); //? might not be windows compatible
    const INTERSECTION_ROAD_NETWORK: &str =
        include_str!("../../resources/road_network_intersection.txt");
    const TRAJ_INTERSECTION_ROAD_NETWORK: &str =
        include_str!("../../resources/traj_network_intersection.txt");

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
        let matched = best_road(&noisy, &rtree);
        // dbg!(&matched);
        // assert!(matched.iter().all(|p| p.is_ok_and(|pp| pp.is_some())));
        // let matched = matched
        //     .into_iter()
        //     .flatten_ok()
        //     .collect::<Result<Vec<_>, _>>()
        //     .unwrap();
        let traj = LineString::from(matched);
        let mut buf = String::new();
        let _ = wkt::to_wkt::write_linestring(&mut buf, &traj).unwrap();
        dbg!(&buf);
        // assert!(false);
        assert_eq!(
            traj_orig.0.len(),
            traj.0.len(),
            "original and matched trajectory should have same cardinality"
        );
    }

    #[test]
    #[ignore = "old implementation"]
    fn best_road_test() {
        let network: MultiLineString =
            wkt::TryFromWkt::try_from_wkt_str(INTERSECTION_ROAD_NETWORK).unwrap();
        let traj_orig: Trajectory =
            wkt::TryFromWkt::try_from_wkt_str(TRAJ_INTERSECTION_ROAD_NETWORK).unwrap();

        let (id, ls): (Vec<u64>, Vec<_>) = network
            .line_strings()
            .enumerate()
            .map(|(id, traj)| (id as u64, traj.clone()))
            .unzip();

        let rtree = RoadIndex::from_ids_and_roads(&id, &ls);

        let traj = LineString::from(best_road(&traj_orig, &rtree));
        let new_traj = LineString::from(best_road_new(traj.points(), &rtree));
        let dist = Euclidean.frechet_distance(&traj, &new_traj);
        assert!((0.0 - dist) < 0.0000001, "frechet dist = {dist}");
        let mut buf = String::new();
        let _ = wkt::to_wkt::write_linestring(&mut buf, &traj).unwrap();
        dbg!(&buf);
        let match_target_road = &network.0[0];
        let frechet_dist = Euclidean.frechet_distance(match_target_road, &traj);
        let frechet_orig_dist = Euclidean.frechet_distance(match_target_road, &traj_orig);

        let min_dist = Euclidean.distance(match_target_road, &traj);
        let min_orig_dist = Euclidean.distance(match_target_road, &traj_orig);
        assert!(
            frechet_dist <= frechet_orig_dist,
            "frechet (dissimilarity) should be smaller after map matching"
        );
        assert!(
            min_dist <= min_orig_dist,
            "minimum distance should be smaller after map matching"
        );
    }

    #[test]
    fn new_best_test() {
        let network: MultiLineString =
            wkt::TryFromWkt::try_from_wkt_str(INTERSECTION_ROAD_NETWORK).unwrap();
        let traj_orig: Trajectory =
            wkt::TryFromWkt::try_from_wkt_str(TRAJ_INTERSECTION_ROAD_NETWORK).unwrap();

        let (id, ls): (Vec<u64>, Vec<_>) = network
            .line_strings()
            .enumerate()
            .map(|(id, traj)| (id as u64, traj.clone()))
            .unzip();

        let rtree = RoadIndex::from_ids_and_roads(&id, &ls);

        let traj = best(&traj_orig, &rtree);
        dbg!(Euclidean.frechet_distance(&traj_orig, &traj));
        let mut buf = String::new();
        let _ = wkt::to_wkt::write_linestring(&mut buf, &traj).unwrap();
        dbg!(&buf);
        dbg!(Euclidean.length(&traj_orig) - Euclidean.length(&traj));

        let match_target_road = &network.0[0];
        let frechet_dist = Euclidean.frechet_distance(match_target_road, &traj);
        let frechet_orig_dist = Euclidean.frechet_distance(match_target_road, &traj_orig);

        let min_dist = Euclidean.distance(match_target_road, &traj);
        let min_orig_dist = Euclidean.distance(match_target_road, &traj_orig);
        assert!(
            frechet_dist <= frechet_orig_dist,
            "frechet (dissimilarity) should be smaller after map matching"
        );
        assert!(
            min_dist <= min_orig_dist,
            "minimum distance should be smaller after map matching"
        );
    }

    #[test]
    #[ignore = "does not work yet"]
    fn new_best_test_277() {
        let network: MultiLineString = wkt::TryFromWkt::try_from_wkt_str(TRAJ_277_NEARBY).unwrap();
        let traj_orig: Trajectory = wkt::TryFromWkt::try_from_wkt_str(TRAJ_277).unwrap();

        let (id, ls): (Vec<u64>, Vec<_>) = network
            .line_strings()
            .enumerate()
            .map(|(id, traj)| (id as u64, traj.clone()))
            .unzip();

        let rtree = RoadIndex::from_ids_and_roads(&id, &ls);

        let traj = best(&traj_orig, &rtree);
        dbg!(Euclidean.frechet_distance(&traj_orig, &traj));
        let mut buf = String::new();
        let _ = wkt::to_wkt::write_linestring(&mut buf, &traj).unwrap();
        dbg!(&buf);
        dbg!(Euclidean.length(&traj_orig) - Euclidean.length(&traj));
        assert!(false);

        // let match_target_road = &network.0[0];
        // let frechet_dist = Euclidean.frechet_distance(match_target_road, &traj);
        // let frechet_orig_dist = Euclidean.frechet_distance(match_target_road, &traj_orig);

        // let min_dist = Euclidean.distance(match_target_road, &traj);
        // let min_orig_dist = Euclidean.distance(match_target_road, &traj_orig);
        // assert!(
        //     frechet_dist <= frechet_orig_dist,
        //     "frechet (dissimilarity) should be smaller after map matching"
        // );
        // assert!(
        //     min_dist <= min_orig_dist,
        //     "minimum distance should be smaller after map matching"
        // );
    }

    #[test]
    fn segment_test() {
        let network: MultiLineString = wkt::TryFromWkt::try_from_wkt_str(TRAJ_277_NEARBY).unwrap();
        let traj_orig: Trajectory = wkt::TryFromWkt::try_from_wkt_str(TRAJ_277).unwrap();

        let (id, ls): (Vec<u64>, Vec<_>) = network
            .line_strings()
            .enumerate()
            .map(|(id, traj)| (id as u64, traj.clone()))
            .unzip();

        let rtree = RoadIndex::from_ids_and_roads(&id, &ls);

        let (f, s): (Vec<_>, Vec<_>) = segment_match(traj_orig.lines(), &rtree)
            .expect("should be able to match all lines")
            .iter()
            .map(|l| (l.start_point(), l.end_point()))
            .unzip();
        let traj = LineString::from_iter(f.iter().interleave(s.iter()).cloned());
        dbg!(Euclidean.frechet_distance(&traj_orig, &traj));
        let mut buf = String::new();
        let _ = wkt::to_wkt::write_linestring(&mut buf, &traj).unwrap();
        dbg!(&buf);
        dbg!(Euclidean.length(&traj_orig) - Euclidean.length(&traj));
        assert!(false);
    }

    #[test]
    fn slope() {
        const LINE: Line = Line {
            start: coord! {x: 0.,y:0.},
            end: coord! {x: 0., y: 1.0},
        };
        const OTHER_LINE: Line = Line {
            start: coord! {x:0.0,y:1.0},
            end: coord! {x:1.0,y:1.0},
        };

        assert_eq!(line_similarity(&LINE, &OTHER_LINE), f64::sqrt(2.0));
    }

    #[test]
    #[ignore = "just playing with things"]
    fn lines_vs_points() {
        let ls = wkt! {LINESTRING (1.0 2.0, 2.0 3.0, 3.0 4.0, 4.0 5.0)};

        let _ = ls
            .lines()
            .inspect(|e| println!("{:?}", e))
            .collect::<Vec<_>>();
        assert_eq!(ls.lines().count(), ls.points().count() - 1)
    }
}
