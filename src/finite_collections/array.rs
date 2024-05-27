use std::marker::PhantomData;
use std::ops::{FnMut, Index, IndexMut};
use std::slice::{Iter, IterMut};

use super::size::Size;

#[derive(Clone)]
pub struct Array<SizeT, ValueT>
where
    SizeT: Size,
{
    vec: Vec<ValueT>,
    phantom: PhantomData<SizeT>,
}

impl<SizeT, ValueT> Default for Array<SizeT, ValueT>
where
    SizeT: Size,
{
    fn default() -> Self {
        Array { vec: Vec::default(), phantom: PhantomData::default() }
    }
}

impl<SizeT, ValueT> Array<SizeT, ValueT>
where
    SizeT: Size,
{
    pub fn capacity(&self) -> SizeT {
        debug_assert!(self.vec.capacity() <= SizeT::MAX.as_usize());
        unsafe { SizeT::from(self.vec.capacity()).unwrap_unchecked() }
    }

    pub fn len(&self) -> SizeT {
        debug_assert!(self.vec.len() <= SizeT::MAX.as_usize());
        unsafe { SizeT::from(self.vec.len()).unwrap_unchecked() }
    }

    pub fn is_empty(&self) -> bool {
        self.vec.is_empty()
    }

    pub fn first(&self) -> Option<&ValueT> {
        self.vec.first()
    }

    pub fn iter(&self) -> Iter<ValueT> {
        self.vec.iter()
    }

    pub fn iter_mut(&mut self) -> IterMut<ValueT> {
        self.vec.iter_mut()
    }

    pub fn reserve(&mut self, additional: SizeT) {
        self.vec.reserve(additional.as_usize());
    }

    pub fn resize_with<F>(&mut self, new_len: SizeT, f: F)
    where
        F: FnMut() -> ValueT,
    {
        self.vec.resize_with(new_len.as_usize(), f);
    }

    pub fn push(&mut self, value: ValueT) {
        self.vec.push(value);
    }

    pub fn pop(&mut self) -> Option<ValueT> {
        self.vec.pop()
    }

    pub fn clear(&mut self) {
        self.vec.clear();
    }

    pub fn shrink_to_fit(&mut self) {
        self.vec.shrink_to_fit();
    }

    pub fn swap(&mut self, a: SizeT, b: SizeT) {
        self.vec.swap(a.as_usize(), b.as_usize());
    }

    pub fn swap_remove(&mut self, index: SizeT) -> ValueT {
        self.vec.swap_remove(index.as_usize())
    }

    pub fn sort_by_cached_key<K, F>(&mut self, f: F)
    where
        F: FnMut(&ValueT) -> K,
        K: Ord,
    {
        self.vec.sort_by_cached_key(f)
    }
}

impl<SizeT, ValueT> Array<SizeT, ValueT>
where
    SizeT: Size,
    ValueT: Clone,
{
    pub fn resize(&mut self, new_len: SizeT, value: ValueT) {
        self.vec.resize(new_len.as_usize(), value);
    }

    pub fn clone_from(&mut self, other: &Self) {
        self.vec.clone_from(&other.vec)
    }
}

impl<SizeT, ValueT> Index<SizeT> for Array<SizeT, ValueT>
where
    SizeT: Size,
{
    type Output = ValueT;
    fn index(&self, index: SizeT) -> &Self::Output {
        &self.vec[index.as_usize()]
    }
}

impl<SizeT, ValueT> IndexMut<SizeT> for Array<SizeT, ValueT>
where
    SizeT: Size,
{
    fn index_mut(&mut self, index: SizeT) -> &mut Self::Output {
        &mut self.vec[index.as_usize()]
    }
}
