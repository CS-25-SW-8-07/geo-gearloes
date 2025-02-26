use geo_types::{CoordNum, LineString};

pub enum Direction {
    Forward,
    Backward,
    Bidirectional,
}

type Id = usize;

pub struct RoadKey(pub Id);

pub struct Road<T: CoordNum> {
    pub id: Id,
    pub geom: LineString<T>,
    pub osm_id: u64,
    pub code: u16,
    pub direction: Direction,
    pub maxspeed: u16,
    pub layer: i16,
    pub bridge: bool,
    pub tunnel: bool,
}

pub struct Roads<T: CoordNum> {
    pub id: Vec<Id>, // Primary key
    pub geom: Vec<LineString<T>>,
    pub osm_id: Vec<u64>,
    pub code: Vec<u16>, // Foreign key to FeatureClass
    pub direction: Vec<Direction>,
    pub maxspeed: Vec<u16>,
    pub layer: Vec<i16>,
    pub bridge: Vec<bool>,
    pub tunnel: Vec<bool>,
}

pub struct NameKey(pub Id);

pub struct NameRow {
    pub id: Id,
    pub name: String,
}

pub struct Name {
    pub id: Vec<Id>, // Primary key
    pub name: Vec<String>,
}

pub struct RefManyKey(pub RoadKey, pub RefKey);
pub struct RefMany {
    pub road_id: Vec<Id>, // Composite key 1
    pub ref_id: Vec<Id>,  // Composite key 2
}

pub struct RefKey(pub Id);
pub struct Ref {
    pub id: Vec<Id>, // Primary key
    pub reff: Vec<String>,
}

pub struct FeatureClassKey(pub u16);

pub struct FeatureClassRow {
    pub code: u16,
    pub fclass: Vec<String>,
}

pub struct FeatureClass {
    pub code: Vec<u16>, // Primary key
    pub fclass: Vec<String>,
}

pub trait Insertable<T> {
    fn insert(&mut self, data: T) -> usize;
}

pub trait Deleteable<T> {
    fn delete(&mut self, key: T) -> usize;
}

pub trait Queryable<T> {
    fn find_index(&self, key: T) -> usize;
}
