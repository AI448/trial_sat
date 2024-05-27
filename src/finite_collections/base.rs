

use std::{cmp::Ordering, marker::PhantomData};
use super::{array::Array, size::Size};


pub trait Comparator<SizeT, ValueT> {
    fn compare(lhs: &(SizeT, ValueT), rhs: &(SizeT, ValueT)) -> Ordering;
}

pub trait Sort<SizeT, ValueT>: Default
where
    SizeT: Size
{
    fn update(&self, heap_array: &mut Array<SizeT, (SizeT, ValueT)>, position_array: &mut Array<SizeT, SizeT>, position: SizeT);
    fn up_heap(&self, heap_array: &mut Array<SizeT, (SizeT, ValueT)>, position_array: &mut Array<SizeT, SizeT>, position: SizeT);
    fn down_heap(&self, heap_array: &mut Array<SizeT, (SizeT, ValueT)>, position_array: &mut Array<SizeT, SizeT>, position: SizeT);
}

pub struct Base<SizeT, ValueT, SortT>
where
    SizeT: Size,
    SortT: Sort<SizeT, ValueT>,
{
    heap_array: Array<SizeT, (SizeT, ValueT)>,
    position_array: Array<SizeT, SizeT>,
    sort: SortT,
}

impl<SizeT, ValueT, SortT> Default for Base<SizeT, ValueT, SortT>
where
    SizeT: Size,
    SortT: Sort<SizeT, ValueT>,
{
    #[inline(never)]
    fn default() -> Self {
        Base { heap_array: Array::default(), position_array: Array::default(), sort: SortT::default() }
    }
}

impl<SizeT, ValueT, SortT> Base<SizeT, ValueT, SortT>
where
    SizeT: Size,
    SortT: Sort<SizeT, ValueT>,
{
    const NULL_POSITION: SizeT = SizeT::MAX;

    #[inline(always)]
    pub fn capacity(&self) -> SizeT {
        self.position_array.len()
    }

    #[inline(always)]
    pub fn len(&self) -> SizeT {
        self.heap_array.len()
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.heap_array.is_empty()
    }

    #[inline(always)]
    pub fn first_key_value(&self) -> Option<&(SizeT, ValueT)> {
        self.heap_array.first()
    }

    #[inline(always)]
    pub fn contains_key(&self, index: SizeT) -> bool {
        self.position_array[index] != Self::NULL_POSITION
    }

    #[inline(always)]
    pub fn get(&self, index: SizeT) -> Option<&ValueT>
    {
        let p = self.position_array[index];
        if p != Self::NULL_POSITION {
            Some(&self.heap_array[p].1)
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn iter(&self) -> impl Iterator<Item = &(SizeT, ValueT)> {
        self.heap_array.iter()
    }

    #[inline(never)]
    pub fn reserve(&mut self, additional: SizeT) {
        self.heap_array.reserve(additional);
        let n = self.position_array.len() + additional;
        self.position_array.resize_with(n, || Self::NULL_POSITION);
    }

    #[inline(never)]
    pub fn clear(&mut self) {
        for (p, (i, _)) in self.heap_array.iter().enumerate() {
            assert!(self.position_array[*i] == SizeT::from(p).unwrap());
            self.position_array[*i] = Self::NULL_POSITION;
        }
        self.heap_array.clear();
    }

    #[inline(always)]
    pub fn insert(&mut self, index: SizeT, value: ValueT) {
        let p = self.position_array[index];
        if self.position_array[index] == Self::NULL_POSITION {
            let p = self.heap_array.len();
            self.position_array[index] = p;
            self.heap_array.push((index, value));
            self.sort.up_heap(&mut self.heap_array, &mut self.position_array, p);
        } else {
            let (i, v) = &mut self.heap_array[p];
            assert!(*i == index);
            *v = value;
            self.sort.update(&mut self.heap_array, &mut self.position_array, p);
        }
    }

    #[inline(always)]
    pub fn pop_first(&mut self) -> Option<(SizeT, ValueT)> {
        if self.heap_array.is_empty() {
            None
        } else {
            let (i, v) = self.heap_array.swap_remove(SizeT::zero());
            assert!(self.position_array[i] == SizeT::zero());
            self.position_array[i] = Self::NULL_POSITION;
            if !self.heap_array.is_empty() {
                self.position_array[self.heap_array[SizeT::zero()].0] = SizeT::zero();
                self.sort.down_heap(&mut self.heap_array, &mut self.position_array, SizeT::zero());
            }
            Some((i, v))
        }
    }

    #[inline(always)]
    pub fn remove(&mut self, index: SizeT) -> Option<ValueT> {
        if self.position_array[index] == Self::NULL_POSITION {
            None
        } else {
            assert!(self.heap_array.len() != SizeT::zero());
            let position = self.position_array[index];
            let (i, value) = self.heap_array.swap_remove(position);
            assert!(i == index);
            self.position_array[index] = Self::NULL_POSITION;
            if position != self.heap_array.len() {
                self.position_array[self.heap_array[position].0] = position;
                self.sort.update(&mut self.heap_array, &mut self.position_array, position);
            }
            Some(value)
        }
    }
}

pub struct NotSort<SizeT, ValueT> {
    phantom: PhantomData<(SizeT, ValueT)>,
}

impl<SizeT, ValueT> Default for NotSort<SizeT, ValueT> {
    fn default() -> Self {
        NotSort { phantom: PhantomData }
    }
}

impl<SizeT, ValueT> Sort<SizeT, ValueT> for NotSort<SizeT, ValueT>
where
    SizeT: Size
{
    #[inline(always)]
    fn update(&self, _heap_array: &mut Array<SizeT, (SizeT, ValueT)>, _position_array: &mut Array<SizeT, SizeT>, _position: SizeT) {}
    #[inline(always)]
    fn up_heap(&self, _heap_array: &mut Array<SizeT, (SizeT, ValueT)>, _position_array: &mut Array<SizeT, SizeT>, _position: SizeT) {}
    #[inline(always)]
    fn down_heap(&self, _heap_array: &mut Array<SizeT, (SizeT, ValueT)>, _position_array: &mut Array<SizeT, SizeT>, _position: SizeT) {}
}

pub struct CustomSort<SizeT, ValueT, ComparaT>
where
    SizeT: Size,
    ComparaT: Comparator<SizeT, ValueT>,
{
    phantom: PhantomData<(SizeT, ValueT, ComparaT)>,
}

impl<SizeT, ValueT, CompareT> Default for CustomSort<SizeT, ValueT, CompareT>
where
    SizeT: Size,
    CompareT: Comparator<SizeT, ValueT>,
{
    fn default() -> Self {
        CustomSort { phantom: PhantomData::default() }
    }
}

impl<SizeT, ValueT, ComparatorT> Sort<SizeT, ValueT> for CustomSort<SizeT, ValueT, ComparatorT>
where
    SizeT: Size,
    ComparatorT: Comparator<SizeT, ValueT>,
{
    #[inline(always)]
    fn update(&self, heap_array: &mut Array<SizeT, (SizeT, ValueT)>, position_array: &mut Array<SizeT, SizeT>, position: SizeT) {
        if 
            position != SizeT::zero() && 
            ComparatorT::compare(&heap_array[(position + SizeT::from(1).unwrap()) / SizeT::from(2).unwrap() - SizeT::from(1).unwrap()], &heap_array[position]) == Ordering::Greater
        {
            self.up_heap(heap_array, position_array, position);
        } else {
            self.down_heap(heap_array, position_array, position);
        }
    }

    #[inline(never)]
    fn up_heap(&self, heap_array: &mut Array<SizeT, (SizeT, ValueT)>, position_array: &mut Array<SizeT, SizeT>, position: SizeT) {
        let mut current = position;
        loop {
            if current == SizeT::zero() {
                break;
            }
            let parent = ((current + SizeT::from(1).unwrap()) / SizeT::from(2).unwrap() - SizeT::from(1).unwrap()).try_into().unwrap();
            if ComparatorT::compare(&heap_array[parent], &heap_array[current]) == Ordering::Greater {
                heap_array.swap(parent, current);
                position_array.swap(heap_array[parent].0, heap_array[current].0);
                current = parent;
            } else {
                break;
            }
        }
    }

    #[inline(never)]
    fn down_heap(&self, heap_array: &mut Array<SizeT, (SizeT, ValueT)>, position_array: &mut Array<SizeT, SizeT>, position: SizeT) {
        let mut current = position;
        loop {
            let left = (current + SizeT::from(1).unwrap()) * SizeT::from(2).unwrap() - SizeT::from(1).unwrap();
            let right = (current + SizeT::from(1).unwrap()) * SizeT::from(2).unwrap();
            if left >= heap_array.len() {
                break;
            }
            let smaller_child =
                if right >= heap_array.len() || ComparatorT::compare(&heap_array[left], &heap_array[right]) == Ordering::Less {
                    left
                } else {
                    right
                };
            if ComparatorT::compare(&heap_array[current], &heap_array[smaller_child]) == Ordering::Greater {
                heap_array.swap(current, smaller_child);
                position_array.swap(heap_array[current].0, heap_array[smaller_child].0);
                current = smaller_child;
            } else {
                break;
            }
        }
    }
}

pub type FiniteHeapedMap<SizeT, ValueT, CompareT> = Base<SizeT, ValueT, CustomSort<SizeT, ValueT, CompareT>>;

pub type FiniteMap<SizeT, ValueT> = Base<SizeT, ValueT, NotSort<SizeT, ValueT>>;
