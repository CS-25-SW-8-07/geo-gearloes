use comms::Parquet;

use crate::Id;

use super::*;

#[derive(Debug, Clone, Copy)]
pub struct NameKey(pub Id);

#[derive(Debug)]
pub struct NameRow {
    pub id: Id,
    pub name: String,
}

#[derive(Debug, Default, Parquet)]
pub struct Name {
    pub id: Vec<Id>, // Primary key
    pub name: Vec<String>,
}

impl Insertable<NameRow> for Name {
    type Key = NameKey;

    fn insert(&mut self, data: NameRow) -> Self::Key {
        if let Some((id, _)) = self
            .id
            .iter()
            .zip(self.name.iter())
            .find(|(&_, n)| data.name == **n)
        {
            return NameKey(*id);
        }

        let next_id = if let Some(id) = self.id.last() {
            id + 1
        } else {
            0
        };

        self.id.push(next_id);
        self.name.push(data.name);

        NameKey(next_id)
    }
}

impl Queryable<NameKey> for Name {
    fn find_index(&self, key: &NameKey) -> Option<usize> {
        self.id.iter().position(|&x| x == key.0)
    }
}

impl Deleteable<NameKey> for Name {
    type Output = NameRow;

    fn delete(&mut self, key: &NameKey) -> Option<Self::Output> {
        if let Some(index) = self.id.iter().position(|&x| x == key.0) {
            Some(Self::Output {
                id: self.id.remove(index),
                name: self.name.remove(index),
            })
        } else {
            None
        }
    }
}
