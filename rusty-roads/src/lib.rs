pub mod table;
pub use table::*;

pub mod index;
pub use index::*;

pub mod graph;
pub use graph::*;

pub mod burn;
pub use burn::*;

#[inline]
pub(crate) fn default<T: Default>() -> T {
    T::default()
}

pub type Id = u64;
