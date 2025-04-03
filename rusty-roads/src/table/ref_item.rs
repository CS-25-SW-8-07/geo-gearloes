use comms::Parquet;

use crate::Id;

use super::*;

#[derive(Debug, Clone, Copy)]
pub struct RefKey(pub Id);
pub struct RefRow {
    pub id: Id,
    pub reff: String,
}

#[derive(Debug, Default, Parquet)]
pub struct Ref {
    pub id: Vec<Id>, // Primary key
    pub reff: Vec<String>,
}

impl Insertable<RefRow> for Ref {
    type Key = RefKey;

    fn insert(&mut self, data: RefRow) -> Self::Key {
        if let Some((id, _)) = self
            .id
            .iter()
            .zip(self.reff.iter())
            .find(|(&_, r)| data.reff == **r)
        {
            return RefKey(*id);
        }

        let next_id = if let Some(id) = self.id.last() {
            id + 1
        } else {
            0
        };

        self.id.push(next_id);
        self.reff.push(data.reff);

        RefKey(next_id)
    }
}

impl Queryable<RefKey> for Ref {
    fn find_index(&self, key: &RefKey) -> Option<usize> {
        self.id.iter().position(|&x| x == key.0)
    }
}

impl Deleteable<RefKey> for Ref {
    type Output = RefRow;

    fn delete(&mut self, key: &RefKey) -> Option<Self::Output> {
        if let Some(index) = self.id.iter().position(|&x| x == key.0) {
            Some(Self::Output {
                id: self.id.remove(index),
                reff: self.reff.remove(index),
            })
        } else {
            None
        }
    }
}
