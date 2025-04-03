use comms::Parquet;

use crate::{Id, RefKey};

use super::{road::RoadKey, *};

#[derive(Debug, Clone, Copy)]
pub struct RefManyKey(pub RoadKey, pub RefKey);
pub struct RefManyRow {
    pub road_id: Id,
    pub ref_id: Id,
}
#[derive(Debug, Default, Parquet)]
pub struct RefMany {
    pub road_id: Vec<Id>, // Composite key 1
    pub ref_id: Vec<Id>,  // Composite key 2
}

impl Insertable<RefManyRow> for RefMany {
    type Key = RefManyKey;

    fn insert(&mut self, data: RefManyRow) -> Self::Key {
        if let Some((road_id, ref_id)) = self
            .road_id
            .iter()
            .zip(self.ref_id.iter())
            .find(|(&ro, &re)| data.road_id == ro && data.ref_id == re)
        {
            return RefManyKey(RoadKey(*road_id), RefKey(*ref_id));
        }

        self.road_id.push(data.road_id);
        self.ref_id.push(data.ref_id);

        RefManyKey(RoadKey(data.road_id), RefKey(data.ref_id))
    }
}

impl Deleteable<RefManyKey> for RefMany {
    type Output = RefManyRow;

    fn delete(&mut self, key: &RefManyKey) -> Option<Self::Output> {
        if let Some(index) = self
            .road_id
            .iter()
            .zip(self.ref_id.iter())
            .position(|(&ro, &re)| key.0 .0 == ro && key.1 .0 == re)
        {
            Some(Self::Output {
                road_id: self.road_id.remove(index),
                ref_id: self.ref_id.remove(index),
            })
        } else {
            None
        }
    }
}

impl Queryable<RefManyKey> for RefMany {
    fn find_index(&self, key: &RefManyKey) -> Option<usize> {
        self.road_id
            .iter()
            .zip(self.ref_id.iter())
            .position(|(&ro, &re)| key.0 .0 == ro && key.1 .0 == re)
    }
}
