use geo_types::LineString;

use crate::Id;

pub struct Trajectories {
    pub id: Vec<Id>,
    pub geom: Vec<LineString<f64>>,
}
