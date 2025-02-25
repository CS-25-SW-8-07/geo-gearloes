use geo_types::{CoordNum, LineString};

pub enum Direction {
    Forward,
    Backward,
    Bidirectional,
}

type Id = usize;

pub struct Road<T: CoordNum> {
    pub geom: Vec<LineString<T>>,
    pub id: Vec<Id>,
    pub osm_id: Vec<u64>,
    pub code: Vec<u16>,
    pub direction: Vec<Direction>,
    pub maxspeed: Vec<u16>,
    pub layer: Vec<i16>,
    pub bridge: Vec<bool>,
    pub tunnel: Vec<bool>,
}

pub struct Name {
    pub id: Vec<Id>,
    pub name: Vec<String>,
}

pub struct RefMany {
    pub road_id: Vec<Id>,
    pub ref_id: Vec<Id>,
}

pub struct Ref {
    pub id: Vec<Id>,
    pub reff: Vec<String>,
}

pub struct FeatureClass {
    pub code: Vec<u16>,
    pub fclass: Vec<String>,
}
