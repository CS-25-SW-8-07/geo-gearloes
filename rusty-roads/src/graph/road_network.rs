use super::super::*;
use derive_more::Into;
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
type RoadNetwork<'a, Ix: IndexType> = DiMatrix<i32, &'a Road, Option<&'a Road>, Ix>;
type NodeId = i32;

pub struct Roadnetwork<'a, Ix: IndexType> {
    network: DiMatrix<NodeId, &'a Road, Option<&'a Road>, Ix>,
    index: HashMap<NodeId, NodeIndex<Ix>>,
}

impl<'a, Ix: IndexType> Roadnetwork<'a, Ix> {
    pub fn new<I>(roads: I) -> Option<Self>
    where
        I: Iterator<Item = RoadWithNode<'a>> + Clone,
    {
        let size = roads.clone().count();
        let mut graph = RoadNetwork::<Ix>::with_capacity(size);
        let mut map = HashMap::<NodeId, NodeIndex<Ix>>::with_capacity(size);
        for RoadWithNode {
            road,
            source,
            target,
        } in roads
        {
            let s = *map.entry(source).or_insert_with(|| graph.add_node(source));
            let dest = *map.entry(target).or_insert_with(|| graph.add_node(target));
            match road.direction {
                Direction::Forward => graph.add_edge(s, dest, road),
                Direction::Backward => graph.add_edge(dest, s, road),
                Direction::Bidirectional => {
                    graph.add_edge(s, dest, road);
                    graph.add_edge(dest, s, road);
                }
            };
        }
        Some(Roadnetwork {
            network: graph,
            index: map,
        })
    }

    // fn sources<I>(&self, s: &NodeId) -> Option<impl Iterator<Item=i32>>
    // where
    //     I: Iterator<Item = i32 >,
    // {
    //     let idx = self.index.get(s)?;
    //     let a = self.network.edges(*idx).map(|(_,_,r)| 1);
    //     // Some(a)
    //     todo!()
    // }

    fn path_find<F, H>(&self, source: NodeId, target: NodeId, cost: F, heuristic: H) -> Option<()>
    where
        F: Fn(&Road) -> f64,
        H: FnMut(petgraph::prelude::NodeIndex<Ix>) -> f64,
    {
        let edge_cost = |e: (_, _, &&Road)| cost(*e.weight());
        let start = self.index.get(&source)?;
        let target = self.index.get(&target)?;
        let is_goal = |n| n == *target;
        let (total_cost, track) =
            petgraph::algo::astar(&self.network, *start, is_goal, edge_cost, heuristic)?;
        let ids = track
            .into_iter()
            .map(|idx| *self.network.node_weight(idx))
            .collect::<Vec<_>>();

        // const A:std::num::NonZero<i32> = std::num::NonZero::<i32>::new(0).unwrap();
        todo!()
    }
}

#[derive(Into)]
pub struct NonNegativef64(f64);

impl NonNegativef64 {
    pub const fn new(num: f64) -> Option<NonNegativef64> {
        match num {
            n if n.signum() == 1.0 => Some(NonNegativef64(n)),
            _ => None,
        }
    }
}

pub fn graph_from_road_network<Ix: IndexType>(
    road_network: Vec<RoadWithNode>,
) -> Option<RoadNetwork<Ix>> {
    assert!(
        <Ix as petgraph::adj::IndexType>::max() > Ix::new(road_network.len()),
        "Road network is greater than maximum index of graph"
    ); //TODO better error handling
       // let mut graph = MatrixGraph::<i32, &Road>::new();
       // let mut graph = RoadNetwork::<i32>::new();
    let mut graph = RoadNetwork::<Ix>::with_capacity(road_network.len());
    let mut map = HashMap::<i32, NodeIndex<Ix>>::with_capacity(road_network.len());
    for &RoadWithNode {
        road,
        source,
        target,
    } in &road_network
    {
        let s = *map.entry(source).or_insert_with(|| graph.add_node(source));
        let dest = *map.entry(target).or_insert_with(|| graph.add_node(target));
        // let s = graph.add_node(source);
        // let dest = graph.add_node(target);
        match road.direction {
            Direction::Forward => graph.add_edge(s, dest, road),
            Direction::Backward => graph.add_edge(dest, s, road),
            Direction::Bidirectional => {
                graph.add_edge(s, dest, road);
                graph.add_edge(dest, s, road);
            }
        };
        // let cost = graph.add_edge(s, dest, road);
    }
    debug_assert!(
        graph.edge_count() >= road_network.len(),
        "expected a graph with {} edges, got {} edges",
        graph.edge_count(),
        road_network.len()
    );

    Some(graph)
}

// pub fn a<'a,Ix: IndexType>(network: impl Iterator<Item= RoadNetwork<'a,Ix>>) -> Option<RoadNetwork<'a,Ix>> {
//     assert!(
//         <Ix as petgraph::adj::IndexType>::max() > Ix::new(network.count()),
//         "Road network is greater than maximum index of graph"
//     ); //TODO better error handling
//        // let mut graph = MatrixGraph::<i32, &Road>::new();
//        // let mut graph = RoadNetwork::<i32>::new();
//     let mut graph = RoadNetwork::<Ix>::with_capacity(network.count());
//     let mut map = HashMap::<i32, NodeIndex<Ix>>::with_capacity(network.count());
//     todo!()
// }

#[cfg(test)]
mod test {
    use std::num::NonZero;

    use geo_types::{coord, LineString};

    use crate::Road;

    use super::{graph_from_road_network, NonNegativef64, RoadWithNode};
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
    fn road_factory(road: &Road, s: i32, t: i32) -> RoadWithNode {
        RoadWithNode {
            road: road,
            source: s,
            target: t,
        }
    }

    #[test]
    fn graph_construct() {
        let r = road();
        let network = vec![
            road_factory(&r, 1, 2),
            road_factory(&r, 2, 3),
            road_factory(&r, 3, 1),
        ];

        let graph = graph_from_road_network::<u32>(network).expect("failed to construct graph");
        let a = petgraph::dot::Dot::with_config(&graph, &[petgraph::dot::Config::EdgeNoLabel]);
        println!("{:?}", a); // use this tool to visualize https://dreampuf.github.io/GraphvizOnline/
        assert_eq!(graph.edge_count(), 3, "expected a graph with 3 edges");
        assert_eq!(graph.node_count(), 3, "expected a graph with 3 nodes");
    }
    #[test]
    fn get_road() {
        let r = road();
        let network = vec![
            road_factory(&r, 1, 2),
            road_factory(&r, 2, 3),
            road_factory(&r, 3, 1),
        ];

        let graph = graph_from_road_network::<u32>(network).unwrap();

        // graph.
    }
    #[test]
    fn non_negative() {
        const _: () = assert!(NonNegativef64::new(-0.0).is_none());
        const _: () = assert!(NonNegativef64::new(-1.0).is_none());
        const _: () = assert!(NonNegativef64::new(f64::NAN).is_none());
        const _: () = assert!(NonNegativef64::new(f64::NEG_INFINITY).is_none());
        const _: () = assert!(NonNegativef64::new(0.0 - f64::EPSILON).is_none());
        const _: () = assert!(NonNegativef64::new(0.0).is_some());
        const _: () = assert!(NonNegativef64::new(f64::INFINITY).is_some());

        // let illegal_input = [f64::NAN, f64::NEG_INFINITY, -1.0, 0.0 - f64::EPSILON];

        // for (idx, ele) in illegal_input.into_iter().enumerate() {
        //     assert!(
        //         NonNegativef64::new(ele).is_none(),
        //         "the number {} should not be considered nonnegative",
        //         ele
        //     );
        // }
    }
}
