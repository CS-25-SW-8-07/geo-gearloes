use comms::Parquet;
use geo_types::LineString;

use crate::{default, Id};

use super::*;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Direction {
    Forward = 0,
    Backward = 1,
    Bidirectional = 2,
}

impl From<u8> for Direction {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Forward,
            1 => Self::Backward,
            _ => Self::Bidirectional,
        }
    }
}

impl From<Direction> for u8 {
    fn from(value: Direction) -> Self {
        value as Self
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RoadKey(pub Id);

#[derive(Debug, Clone)]
pub struct Road {
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

#[derive(Debug, Default, Clone, Parquet)]
pub struct Roads {
    pub id: Vec<Id>, // Primary key
    pub geom: Vec<LineString<f64>>,
    pub osm_id: Vec<u64>,
    pub code: Vec<u16>, // Foreign key to FeatureClass
    #[parquet_type(u8)]
    pub direction: Vec<Direction>,
    pub maxspeed: Vec<u16>,
    pub layer: Vec<i16>,
    pub bridge: Vec<bool>,
    pub tunnel: Vec<bool>,
}

impl FromIterator<Road> for Roads {
    fn from_iter<I: IntoIterator<Item = Road>>(iter: I) -> Self {
        let mut slf: Self = default();
        slf.insert_many(iter);
        slf
    }
}

impl Insertable<Road> for Roads {
    type Key = RoadKey;

    fn insert(&mut self, data: Road) -> Self::Key {
        // Does not insert duplicates
        if let Some((id, _)) = self
            .id
            .iter()
            .zip(self.osm_id.iter())
            .find(|(&_, &o)| data.osm_id == o)
        {
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

impl Deleteable<RoadKey> for Roads {
    type Output = Road;
    fn delete(&mut self, key: &RoadKey) -> Option<Self::Output> {
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

impl Queryable<RoadKey> for Roads {
    fn find_index(&self, key: &RoadKey) -> Option<usize> {
        self.id.iter().position(|&x| x == key.0)
    }
}
