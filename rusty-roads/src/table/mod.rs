pub mod name;
pub use name::*;
pub mod road;
pub use road::*;
pub mod ref_many;
pub use ref_many::*;
pub mod ref_item;
pub use ref_item::*;
pub mod feature_class;
pub use feature_class::*;
pub mod anonymities;
pub use anonymities::*;
pub mod trajectories;
pub use trajectories::*;

/// Type T is insertable into Self
pub trait Insertable<Data> {
    type Key;
    /// Insert Data into Self
    fn insert(&mut self, data: Data) -> Self::Key;
    /// Insert many Data into Self
    fn insert_many<I: IntoIterator<Item = Data>>(&mut self, data: I) -> Vec<Self::Key> {
        data.into_iter().map(|x| self.insert(x)).collect()
    }
}

/// Type Key is deleteable from Self
pub trait Deleteable<Key> {
    type Output;
    /// Deletes Key from Self
    fn delete(&mut self, key: &Key) -> Option<Self::Output>;
    /// Deletes many Keys from Self
    fn delete_many(&mut self, keys: &[Key]) -> Vec<Option<Self::Output>> {
        keys.iter().map(|x| self.delete(x)).collect()
    }
}

/// Type Key is queryable from Self
pub trait Queryable<Key> {
    /// Find the index of T in Self
    fn find_index(&self, key: &Key) -> Option<usize>;
    /// Find many indecies of T in Self
    fn find_many_indexes(&self, keys: &[Key]) -> Vec<Option<usize>> {
        keys.iter().map(|x| self.find_index(x)).collect()
    }
}
