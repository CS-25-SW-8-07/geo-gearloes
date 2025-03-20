use std::collections::HashMap;

use super::super::*;
use petgraph::matrix_graph::*;

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
        let dest = *map.entry(target).or_insert_with(||graph.add_node(target));
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

#[cfg(test)]
mod test {
    use geo_types::{coord, LineString};

    use crate::Road;

    use super::{graph_from_road_network, RoadWithNode};
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
}
