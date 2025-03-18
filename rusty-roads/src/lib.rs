pub mod table;
pub use table::*;

pub mod index;
pub use index::*;

#[inline]
pub(crate) fn default<T: Default>() -> T {
    T::default()
}

pub type Id = u64;
