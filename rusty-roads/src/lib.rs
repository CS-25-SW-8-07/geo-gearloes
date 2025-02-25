use geo_types::{CoordNum, LineString};

pub enum Direction {
    Forward,
    Backward,
    Bidirectional,
}

type Id = usize;

pub struct RoadKey(pub Id);

pub struct RoadRow<T: CoordNum> {
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

pub struct Road<T: CoordNum> {
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

impl<T: CoordNum> Insertable<RoadRow<T>> for Road<T> {
    type Key = RoadKey;

    fn insert(&mut self, data: RoadRow<T>) -> Self::Key {
        // Does not insert duplicates
        if let Some((id, _)) = self.id.iter().zip(self.osm_id.iter()).find(|(&_, &o)| data.osm_id == o) {
            return RoadKey(*id);
        }

        // Finds the next id for the table
        let next_id = if let Some(id) = self.id.last() {
            id + 1
        } else {
            0
        };



        self.id.push(next_id);
        self.geom.push(data.geom);
        self.osm_id.push(data.osm_id);
        self.code.push(data.code);
        self.direction.push(data.direction);
        self.maxspeed.push(data.maxspeed);
        self.layer.push(data.layer);
        self.bridge.push(data.bridge);
        self.tunnel.push(data.tunnel);

        RoadKey(next_id)
    }
}

impl<T: CoordNum> Queryable<RoadKey> for Road<T> {
    fn find_index(&self, key: RoadKey) -> Option<usize> {
        self.id.iter().position(|&x| x == key.0)
    }
}

impl<T: CoordNum> Deleteable<RoadKey> for Road<T> {
    type Output = RoadRow<T>;
    fn delete(&mut self, key: RoadKey) -> Option<Self::Output> {
        if let Some(index) = self.id.iter().position(|&x| x == key.0) {
            Some(Self::Output {
                id: self.id.remove(index),
                geom: self.geom.remove(index),
                osm_id: self.osm_id.remove(index),
                code: self.code.remove(index),
                direction: self.direction.remove(index),
                maxspeed: self.maxspeed.remove(index),
                layer: self.layer.remove(index),
                bridge: self.bridge.remove(index),
                tunnel: self.tunnel.remove(index),
            })
        } else {
            None
        }
    }
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
    pub code: u16, // Primary key
    pub fclass: Vec<String>,
}

pub struct FeatureClass {
    pub code: Vec<u16>, // Primary key
    pub fclass: Vec<String>,
}



pub trait Insertable<T> {
    type Key;
    fn insert(&mut self, data: T) -> Self::Key;
}

pub trait Deleteable<T> {
    type Output;
    fn delete(&mut self, key: T) -> Option<Self::Output>;
}

pub trait Queryable<T> {
    fn find_index(&self, key: T) -> Option<usize>;
}
