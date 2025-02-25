use geo_types::{CoordNum, LineString};

pub enum Direction {
    Forward,
    Backward,
    Bidirectional,
}

type Id = usize;

pub struct Road<T: CoordNum> {
    pub geom: LineString<T>,
    pub id: Id,
    pub osm_id: u64,
    pub code: u16,
    pub direction: Direction,
    pub maxspeed: u16,
    pub layer: i16,
    pub bridge: bool,
    pub tunnel: bool,
}

pub struct Name {
    pub id: Id,
    pub name: String,
}

pub struct RefMany {
    pub road_id: Id,
    pub ref_id: Id,
}

pub struct Ref {
    pub id: Id,
    pub reff: String,
}

pub struct FeatureClass {
    pub code: u16,
    pub fclass: String,
}
