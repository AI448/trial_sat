use super::base;
use num::{PrimInt, Unsigned};

#[derive(Default)]
pub struct Set<IndexT>
where
    IndexT: Unsigned + PrimInt,
{
    map: base::Base<IndexT, (), base::NotSort<IndexT, ()>>,
}

impl<IndexT> Set<IndexT>
where
    IndexT: Unsigned + PrimInt,
{
    pub fn capacity(&self) -> usize {
        self.map.capacity()
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn contains_key(&self, index: IndexT) -> bool {
        self.map.contains_key(index)
    }

    pub fn iter(&self) -> impl Iterator<Item = &IndexT> {
        self.map.iter().map(|(index, ..)| index)
    }

    pub fn reserve(&mut self, additional: usize) {
        self.map.reserve(additional);
    }

    pub fn clear(&mut self) {
        self.map.clear();
    }

    pub fn insert(&mut self, index: IndexT) {
        self.map.insert(index, ());
    }

    pub fn remove(&mut self, index: IndexT) {
        self.map.remove(index);
    }
}
