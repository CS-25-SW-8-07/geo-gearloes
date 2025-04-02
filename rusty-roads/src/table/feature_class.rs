use comms::Parquet;

use super::*;

#[derive(Debug, Clone, Copy)]
pub struct FeatureClassKey(pub u16);

#[derive(Debug)]
pub struct FeatureClassRow {
    pub code: u16, // Primary key
    pub fclass: String,
}

#[derive(Debug, Default, Parquet)]
pub struct FeatureClass {
    pub code: Vec<u16>, // Primary key
    pub fclass: Vec<String>,
}

impl Insertable<FeatureClassRow> for FeatureClass {
    type Key = FeatureClassKey;

    fn insert(&mut self, data: FeatureClassRow) -> Self::Key {
        if let Some((code, _)) = self
            .code
            .iter()
            .zip(self.fclass.iter())
            .find(|(&_, o)| data.fclass == **o)
        {
            return FeatureClassKey(*code);
        }

        self.code.push(data.code);
        self.fclass.push(data.fclass);

        FeatureClassKey(data.code)
    }
}

impl Deleteable<FeatureClassKey> for FeatureClass {
    type Output = FeatureClassRow;

    fn delete(&mut self, key: &FeatureClassKey) -> Option<Self::Output> {
        if let Some(index) = self.code.iter().position(|&x| x == key.0) {
            Some(Self::Output {
                code: self.code.remove(index),
                fclass: self.fclass.remove(index),
            })
        } else {
            None
        }
    }
}

impl Queryable<FeatureClassKey> for FeatureClass {
    fn find_index(&self, key: &FeatureClassKey) -> Option<usize> {
        self.code.iter().position(|&x| x == key.0)
    }
}
