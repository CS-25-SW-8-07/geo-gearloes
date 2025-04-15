use crate::RoadIndex;
use crate::Roads;

use super::super::Road;
use super::super::RoadWithNode;
use geo::closest_point::ClosestPoint;
use geo::Closest;
use geo::Distance;
use geo::Euclidean;
use geo::Length;
use geo::Point;
use geo::{Line, LineString, MultiLineString};
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

/// Compares direction of 2 lines
/// returns a number between 0 and 2 (inclusive) where 0 means their direction is identical and 2 means they are opposite (sqrt(2) meaning a perfect right angle)
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

/// attempts to match an input trajectory to the given road network
///
/// # Panics
///
/// Panics if the rtree is empty .
///
/// # Errors
///
/// This function will return an error if any point in the trajectory cannot be matched (i.e. closest point is indeterminate).
///
/// # Example
/// ```
/// use rusty_roads::segment_match;
/// use rusty_roads::RoadIndex;
/// use geo::wkt;
/// use geo::MultiLineString;
/// use geo_traits::MultiLineStringTrait;
///
/// let a = 1+1;
/// let traj = wkt!{LINESTRING(1.0 2.0, 2.0 3.0, 3.0 4.0)};
/// let road_network: MultiLineString<f64> = wkt!{MULTILINESTRING((0.5 2.0, 2.0 3.0, 3.0 4.0, 4.0 5.0),(50.0 100.0, 100.0 200.0))};
/// let (ids, lss): (Vec<u64>, Vec<_>) = road_network.line_strings().enumerate().map(|(id, traj)| (id as u64, traj.clone())).unzip();
/// let rtree = RoadIndex::from_ids_and_roads(&ids, &lss);
/// let matched = segment_match(traj.lines(),&rtree);
/// assert_eq!(matched.unwrap().len(), traj.lines().count());
/// ```
pub fn segment_match<I>(sub_traj: I, index: &RoadIndex) -> Result<Vec<Line>, (usize, Line)>
where
    I: Iterator<Item = Line>,
{
    const MAX_CANDIDATES: usize = 20; // completely arbitrary

    debug_assert!(index.index.size() >= 1, "rtree index should be nonempty");

    let matched = sub_traj.enumerate().map(|(idx, l)| {
        let candidate_roads_start = index
            .index
            .nearest_neighbor_iter_with_distance_2(&l.start_point())
            .take(MAX_CANDIDATES);
        let candidate_roads_end = index
            .index
            .nearest_neighbor_iter_with_distance_2(&l.end_point())
            .take(MAX_CANDIDATES);

        // gather candidate roads from start and end roads
        let all_candidates = candidate_roads_start.chain(candidate_roads_end);

        // find the road with with smallest distance to a line segment
        let (best, _dist) = all_candidates
            .filter_map(|(g, _)| {
                let (closest_start, _) = closest(&l.start_point(), g.geom()).ok()?;
                let (closest_end, _) = closest(&l.end_point(), g.geom()).ok()?; // Note: if every candidate causes a None value here, the matched trajectory will have smaller cardinality

                let f_dist = Euclidean.distance(closest_start, l.start_point());
                let l_dist = Euclidean.distance(closest_end, l.end_point());
                let w = match closest_start == closest_end {
                    false => 1.0,
                    true => 2.0, // also completely arbitrary
                };

                Some((g, (f_dist + l_dist) * w))
            })
            .min_by(|(_, fst), (_, snd)| fst.total_cmp(snd))
            .ok_or((idx, l))?; // unlikely, but can be triggered if all nn's have indeterminate closest point
        let start_matched = closest(&l.start_point(), best.geom()).map_err(|_| (idx, l))?;
        let end_matched = closest(&l.end_point(), best.geom()).map_err(|_| (idx, l))?;

        Ok((start_matched, end_matched))
    });

    let result: Result<Vec<_>, (usize, Line)> =
        matched.map_ok(|(a, b)| Line::new(a.0, b.0)).try_collect();
    result
}

fn closest(p: &Point, first_nn: &LineString) -> Result<(Point, Point), Point> {
    match first_nn.closest_point(p) {
        Closest::SinglePoint(s) => Ok((s, *p)),
        Closest::Intersection(i) => Ok((i, *p)),
        Closest::Indeterminate => Err(*p),
    }
}

#[cfg(test)]
mod tests {

    use geo::line_measures::FrechetDistance;
    use geo::{coord, wkt, Closest, Coord, Euclidean, Point};
    use geo_traits::MultiLineStringTrait;
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

        let mut traj = vec![f.first().unwrap()];
        traj.extend(s.iter());
        let traj = LineString::from_iter(traj.iter().cloned().cloned());
        // let traj = LineString::from_iter(f.iter().interleave(s.iter()).cloned());
        dbg!(Euclidean.frechet_distance(&traj_orig, &traj));
        let mut buf = String::new();
        let _ = wkt::to_wkt::write_linestring(&mut buf, &traj).unwrap();
        dbg!(&buf);
        // assert!(false);
        dbg!(Euclidean.length(&traj_orig) - Euclidean.length(&traj));
        assert_eq!(traj_orig.0.len(), traj.0.len());
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
