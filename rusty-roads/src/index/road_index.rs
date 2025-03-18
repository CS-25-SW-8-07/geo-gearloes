use geo_types::LineString;
use rstar::{primitives::GeomWithData, RTree};

#[derive(Debug, Clone)]
pub struct RoadIndex {
    pub index: RTree<GeomWithData<LineString<f64>, u64>>,
}

impl RoadIndex {
    pub fn new() -> RoadIndex {
        Self {
            index: RTree::new(),
        }
    }

    pub fn from_ids_and_roads(ids: &[u64], roads: &[LineString<f64>]) -> RoadIndex {
        let geomdata: Vec<GeomWithData<LineString<f64>, u64>> = roads
            .iter()
            .zip(ids.iter())
            .map(|(road, id)| GeomWithData::<LineString<f64>, u64>::new(road.clone(), *id))
            .collect();

        RoadIndex {
            index: RTree::<GeomWithData<LineString<f64>, u64>>::bulk_load(geomdata),
        }
    }

    pub fn insert(&mut self, id: u64, road: LineString<f64>) {
        let geomdata: GeomWithData<LineString<f64>, u64> = GeomWithData::new(road, id);
        self.index.insert(geomdata);
    }

    pub fn empty(&mut self) {
        self.index = RTree::<GeomWithData<LineString<f64>, u64>>::new();
    }

    pub fn remove(&mut self, _id: u64) {
        todo!()
    }
}

impl Default for RoadIndex {
    fn default() -> Self {
        Self::new()
    }
}
