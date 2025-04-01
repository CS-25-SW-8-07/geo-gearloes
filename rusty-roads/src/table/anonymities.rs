use crate::Id;

pub struct Anonymities {
    pub road_id: Vec<Id>,
    pub current_k: Vec<f64>,
}
pub type Meter = f64;
pub struct AnonymityConf {
    pub min_k: u32,
    pub min_k_percentile: f64,
    pub min_area_size: Meter, // In meter
}
