use geo_types::LineString;

use comms::Parquet;

#[derive(Parquet)]
pub struct Trajectories {
    pub geom: Vec<LineString<f64>>,
}
