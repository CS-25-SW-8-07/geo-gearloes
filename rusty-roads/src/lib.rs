use geo_types::{CoordNum, LineString};

pub mod parquet;
pub use parquet::*;

#[derive(Debug)]
#[repr(u8)]
pub enum Direction {
    Forward = 0,
    Backward = 1,
    Bidirectional = 2,
}

type Id = u64;

#[derive(Debug)]
pub struct RoadKey(pub Id);

#[derive(Debug)]
pub struct RoadRow {
    pub id: Id,
    pub geom: LineString<f64>,
    pub osm_id: u64,
    pub code: u16,
    pub direction: Direction,
    pub maxspeed: u16,
    pub layer: i16,
    pub bridge: bool,
    pub tunnel: bool,
}

#[derive(Debug, Default)]
pub struct Road {
    pub id: Vec<Id>, // Primary key
    pub geom: Vec<LineString<f64>>,
    pub osm_id: Vec<u64>,
    pub code: Vec<u16>, // Foreign key to FeatureClass
    pub direction: Vec<Direction>,
    pub maxspeed: Vec<u16>,
    pub layer: Vec<i16>,
    pub bridge: Vec<bool>,
    pub tunnel: Vec<bool>,
}

#[derive(Debug)]
pub struct NameKey(pub Id);

#[derive(Debug)]
pub struct NameRow {
    pub id: Id,
    pub name: String,
}

#[derive(Debug)]
pub struct Name {
    pub id: Vec<Id>, // Primary key
    pub name: Vec<String>,
}

#[derive(Debug)]
pub struct RefManyKey(pub RoadKey, pub RefKey);

#[derive(Debug)]
pub struct RefMany {
    pub road_id: Vec<Id>, // Composite key 1
    pub ref_id: Vec<Id>,  // Composite key 2
}

#[derive(Debug)]
pub struct RefKey(pub Id);

#[derive(Debug)]
pub struct Ref {
    pub id: Vec<Id>, // Primary key
    pub reff: Vec<String>,
}

#[derive(Debug)]
pub struct FeatureClassKey(pub u16);

#[derive(Debug)]
pub struct FeatureClassRow {
    pub code: u16,
    pub fclass: Vec<String>,
}

#[derive(Debug)]
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
