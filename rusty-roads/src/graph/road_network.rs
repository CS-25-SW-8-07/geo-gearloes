use super::super::*;
use bimap::{BiHashMap, BiMap};
use derive_more::Into;
use geo::{Coord, Point};
use petgraph::Direction::{Incoming, Outgoing};
use petgraph::{matrix_graph::*, visit::EdgeRef};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct RoadWithNode<'a> {
    road: &'a Road,
    source: i32,
    target: i32,
}

impl RoadWithNode<'_> {
    fn direction(&self) -> Direction {
        self.road.direction
    }
}
#[allow(type_alias_bounds)]
type RoadNetworkGraph<'a, Idx: IndexType> = DiMatrix<i32, &'a Road, Option<&'a Road>, Idx>;

type NodeId = i32;

pub struct RoadNetwork<'a, Idx: IndexType> {
    network: DiMatrix<NodeId, &'a Road, Option<&'a Road>, Idx>,
    bi_map: BiMap<NodeId, NodeIndex<Idx>>,
}

impl<'a, Idx: IndexType> RoadNetwork<'a, Idx> {
    pub fn new<I>(roads: I) -> Option<Self>
    where
        I: Iterator<Item = RoadWithNode<'a>> + Clone,
    {
        let size = roads.clone().count();
        assert!(
            <Idx as petgraph::adj::IndexType>::max().index() > size,
            "Road network is greater than maximum index of graph"
        ); //TODO better error handling
        let mut graph = RoadNetworkGraph::<Idx>::with_capacity(size);
        let mut bi_map = BiHashMap::with_capacity(size);
        for RoadWithNode {
            road,
            source,
            target,
        } in roads
        {
            let s = match bi_map.get_by_left(&source) {
                Some(e) => *e,
                None => {
                    let idx = graph.add_node(source);
                    let _ = bi_map.insert(source, idx);
                    idx
                }
            };
            let dest = match bi_map.get_by_left(&target) {
                Some(e) => *e,
                None => {
                    let idx = graph.add_node(target);
                    let _ = bi_map.insert(target, idx);
                    idx
                }
            };
            match road.direction {
                Direction::Forward => graph.add_edge(s, dest, road),
                Direction::Backward => graph.add_edge(dest, s, road),
                Direction::Bidirectional => {
                    graph.add_edge(s, dest, road);
                    graph.add_edge(dest, s, road);
                }
            };
        }
        debug_assert_eq!(
            graph.node_count(),
            bi_map.len(),
            "number of graph nodes should equal number of entries in hashmap"
        );
        Some(RoadNetwork {
            network: graph,
            bi_map,
        })
    }

    pub fn path_find<F, H>(
        &self,
        source: NodeId,
        target: NodeId,
        cost: F,
        mut heuristic: H,
    ) -> Option<(NonNegativef64, Vec<i32>)>
    where
        F: Fn(&Road) -> NonNegativef64,
        H: FnMut(NodeId) -> NonNegativef64,
    {
        let edge_cost = |e: (_, _, &&Road)| cost(e.weight()).into();
        let start = self.bi_map.get_by_left(&source)?;
        let target = self.bi_map.get_by_left(&target)?;
        let is_goal = |n| n == *target;
        let new_heuristic = |idx: NodeIndex<Idx>| {
            heuristic(
                *self
                    .bi_map
                    .get_by_right(&idx)
                    .expect("expected to find a node id corresponding to graph index"),
            )
            .0
        };

        let (total_cost, track) =
            petgraph::algo::astar(&self.network, *start, is_goal, edge_cost, new_heuristic)?;
        let ids = track
            .into_iter()
            .map(|idx| *self.network.node_weight(idx))
            .collect::<Vec<_>>();
        Some((NonNegativef64::try_from(total_cost)?, ids))
    }

    /// The point where a given node lies
    fn point_from_node(&self, id: NodeId) -> Option<Point> {
        let a = self.bi_map.get_by_left(&id)?;
        let io = self
            .network
            .edges_directed(*a, Outgoing)
            .filter_map(|e| e.2.geom.0.first())
            .chain(
                self.network
                    .edges_directed(*a, Incoming)
                    .filter_map(|e| e.2.geom.0.last()),
            );
        let (xs, ys): (Vec<f64>, Vec<f64>) = io.map(|c| c.x_y()).unzip();
        let (size_x, size_y) = (xs.len() as f64, ys.len() as f64);
        let avg_x: f64 = xs.into_iter().sum::<f64>() / size_x;
        let avg_y = ys.into_iter().sum::<f64>() / size_y;
        Some(Point::new(avg_x, avg_y))
        // todo!()
    }
}

#[derive(Into)]
pub struct NonNegativef64(f64);

impl NonNegativef64 {
    pub const fn try_from(num: f64) -> Option<NonNegativef64> {
        match num {
            n if n.signum() == 1.0 => Some(NonNegativef64(n)),
            _ => None,
        }
    }
}


#[cfg(test)]
mod test {
    use std::u8;

    use geo::line_measures::LengthMeasurable;
    use geo::{Distance, Point};
    use geo_types::{coord, LineString};

    use crate::graph::road_network::NodeId;
    use crate::Road;

    macro_rules! big_graph_tests {
        ($($name:ident <$t:tt>,)*) => {
            $(
                #[test]
                #[should_panic(expected = "Road network is greater than maximum index of graph")]
                fn $name() {
                    let r = road();
                    let roads = vec![road_factory(&r, 1, 1); $t ::MAX as usize];
                    let roads = roads.into_iter().enumerate().map(|(i, r)| RoadWithNode {
                        road: r.road,
                        source: i as i32,
                        target: r.target,
                    });
                    let _big_graph = RoadNetwork::<$t>::new(roads);
                }
            )
            *
        };
    }

    big_graph_tests! {
        too_big_graphu8<u8>,
        too_big_graphu16<u16>,
        // too_big_graphu32<u32>, //! this takes a while to compute
    }

    use super::{NonNegativef64, RoadNetwork, RoadWithNode};
    // static mut ID: u64 = 1;
    fn road() -> Road {
        Road {
            id: 1,
            geom: LineString::new(vec![coord! {x:1.,y:2.}, coord! {x:3.,y:4.}]),
            osm_id: 42,
            code: 69,
            direction: crate::Direction::Forward,
            maxspeed: 2137,
            layer: 0,
            bridge: false,
            tunnel: false,
        }
    }
    fn road_bidirectional() -> Road {
        let mut road = road();
        road.direction = crate::Direction::Bidirectional;
        road
    }
    fn road_factory(road: &Road, s: i32, t: i32) -> RoadWithNode {
        RoadWithNode {
            road: road,
            source: s,
            target: t,
        }
    }

    #[test]
    fn graph_with_hashmap() {
        let r = road();
        let network = vec![
            road_factory(&r, 1, 2),
            road_factory(&r, 2, 3),
            road_factory(&r, 3, 1),
        ];
        let network = RoadNetwork::<u8>::new(network.into_iter()).unwrap();

        let a = petgraph::dot::Dot::with_config(
            &network.network,
            &[petgraph::dot::Config::EdgeNoLabel],
        );
        println!("{:?}", a); // use this tool to visualize https://dreampuf.github.io/GraphvizOnline/
        assert_eq!(
            network.network.node_count(),
            network.bi_map.len(),
            "Hashmap should contain as many values, as there are nodes"
        );
    }
    #[test]
    fn graph_bidirectional() {
        let br = road_bidirectional();
        let network = vec![
            road_factory(&br, 1, 2),
            road_factory(&br, 2, 3),
            road_factory(&br, 3, 1),
        ];
        let network = RoadNetwork::<u8>::new(network.into_iter()).unwrap();

        let a = petgraph::dot::Dot::with_config(
            &network.network,
            &[petgraph::dot::Config::EdgeNoLabel],
        );
        println!("{:?}", a); // use this tool to visualize https://dreampuf.github.io/GraphvizOnline/
        assert_eq!(network.network.node_count(), network.bi_map.len());
        assert_eq!(
            network.network.edge_count(),
            network.bi_map.len() * 2,
            "edge count should be twice as high in a fully bi-directional road network"
        );
    }

    #[test]
    fn graph_astar() {
        let r = road();
        let network = vec![
            road_factory(&r, 1, 2),
            road_factory(&r, 2, 3),
            road_factory(&r, 3, 1),
            road_factory(&r, 2, 4),
            road_factory(&r, 4, 5),
            road_factory(&r, 2, 5),
        ]; // assuming uniform weights, shortest path from 1 to 5 should be 1 -> 2 -> 5

        let network = RoadNetwork::<u8>::new(network.into_iter()).unwrap();
        let a = petgraph::dot::Dot::with_config(
            &network.network,
            &[petgraph::dot::Config::EdgeNoLabel],
        );
        println!("{:?}", a); // use this tool to visualize https://dreampuf.github.io/GraphvizOnline/
        let (cost, path) = network
            .path_find(1, 5, |_| NonNegativef64(1.0), |_| NonNegativef64(0.0))
            .expect("expected to find a path");

        assert_eq!(cost.0, 2.0);
        assert_eq!(path, vec![1, 2, 5])
    }

    #[test]
    fn disconnected_graphs_astar() {
        let r = road();
        let network = vec![
            road_factory(&r, 1, 2),
            road_factory(&r, 2, 3),
            road_factory(&r, 3, 1),
            road_factory(&r, 2, 4),
            road_factory(&r, 4, 5),
            road_factory(&r, 2, 5),
            road_factory(&r, 6, 7),
        ]; // assuming uniform weights, shortest path from 1 to 5 should be 1 -> 2 -> 5

        let network = RoadNetwork::<u8>::new(network.into_iter()).unwrap();
        let a = petgraph::dot::Dot::with_config(
            &network.network,
            &[petgraph::dot::Config::EdgeNoLabel],
        );
        println!("{:?}", a); // use this tool to visualize https://dreampuf.github.io/GraphvizOnline/
        let res = network.path_find(1, 6, |_| NonNegativef64(1.0), |_| NonNegativef64(0.0));
        assert!(
            res.is_none(),
            "No path should be possible between disconnected graphs"
        );
    }
    #[test]
    fn non_negative() {
        const _: () = assert!(NonNegativef64::try_from(-0.0).is_none());
        const _: () = assert!(NonNegativef64::try_from(-1.0).is_none());
        const _: () = assert!(NonNegativef64::try_from(f64::NAN).is_none());
        const _: () = assert!(NonNegativef64::try_from(f64::NEG_INFINITY).is_none());
        const _: () = assert!(NonNegativef64::try_from(0.0 - f64::EPSILON).is_none());
        const _: () = assert!(NonNegativef64::try_from(0.0).is_some());
        const _: () = assert!(NonNegativef64::try_from(f64::INFINITY).is_some());

        assert!(true, "does not really need to be a test");
    }

    #[test]
    fn graph_dist_example() {
        use geo::geometry::LineString;
        use geo::Euclidean;
        use geo::Haversine;
        let r = road();
        let network = vec![
            road_factory(&r, 1, 2),
            road_factory(&r, 2, 3),
            road_factory(&r, 3, 1),
            road_factory(&r, 2, 4),
            road_factory(&r, 4, 5),
            road_factory(&r, 2, 5),
        ]; // assuming uniform weights, shortest path from 1 to 5 should be 1 -> 2 -> 5

        let network = RoadNetwork::<u8>::new(network.into_iter()).unwrap();
        let a = petgraph::dot::Dot::with_config(
            &network.network,
            &[petgraph::dot::Config::EdgeNoLabel],
        );
        println!("{:?}", a); // use this tool to visualize https://dreampuf.github.io/GraphvizOnline/

        fn cost_fn(r: &Road) -> NonNegativef64 {
            let (first_lon, first_lat) = r.geom.0.first().unwrap().x_y();
            let (last_lon, last_lat) = r.geom.0.last().unwrap().x_y();

            let dist = Haversine.distance( //? the total cost will not make a lot of sense using, it is just an example
                Point::new(first_lon, first_lat), 
                Point::new(last_lon, last_lat),
            );
            NonNegativef64::try_from(dist).expect("distance should always be nonnegative")
        }
        // let cost_fn = |p:&Road| p.geom.length(metric_space)
        // let cost_fn = |p:&Road| Euclidean.distance(p.geom.0.first().unwrap(), p.geom.0.last().unwrap());
        // let cost_fn = |p:&Road| Haversine.distance(Point::, destination);

        let target = 5;
        let (cost, path) = network
            .path_find(
                1,
                target,
                |r| cost_fn(r),
                |id| {
                    NonNegativef64::try_from(Haversine.distance(
                        network.point_from_node(id).unwrap(),
                        network.point_from_node(target).unwrap(),
                    ))
                    .unwrap()
                },
            )
            .expect("expected to find a path");

            assert!(cost.0>0.0);
            assert_eq!(path, vec![1, 2, 5])
    }
}
