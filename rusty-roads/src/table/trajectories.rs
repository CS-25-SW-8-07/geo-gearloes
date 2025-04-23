use geo_types::LineString;

use comms::Parquet;

use crate::Id;

#[derive(Parquet)]
pub struct Trajectories {
    pub id: Vec<Id>,
    pub geom: Vec<LineString<f64>>,
}
