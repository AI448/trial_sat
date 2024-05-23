use num::{PrimInt, Unsigned};
use std::{cmp::Ordering, marker::PhantomData};

pub trait Comparator<IndexT, ValueT> {
    fn compare(lhs: &(IndexT, ValueT), rhs: &(IndexT, ValueT)) -> Ordering;
}

pub trait Sort<IndexT, ValueT>: Default {
    fn update(&self, heap_array: &mut Vec<(IndexT, ValueT)>, position_array: &mut Vec<IndexT>, position: IndexT);
    fn up_heap(&self, heap_array: &mut Vec<(IndexT, ValueT)>, position_array: &mut Vec<IndexT>, position: IndexT);
    fn down_heap(&self, heap_array: &mut Vec<(IndexT, ValueT)>, position_array: &mut Vec<IndexT>, position: IndexT);
}

pub struct Base<IndexT, ValueT, SortT>
where
    IndexT: Unsigned + PrimInt,
    SortT: Sort<IndexT, ValueT>,
{
    heap_array: Vec<(IndexT, ValueT)>,
    position_array: Vec<IndexT>,
    sort: SortT,
}

impl<IndexT, ValueT, SortT> Default for Base<IndexT, ValueT, SortT>
where
    IndexT: Unsigned + PrimInt,
    SortT: Sort<IndexT, ValueT>,
{
    fn default() -> Self {
        Base { heap_array: Vec::default(), position_array: Vec::default(), sort: SortT::default() }
    }
}

impl<IndexT, ValueT, SortT> Base<IndexT, ValueT, SortT>
where
    IndexT: Unsigned + PrimInt,
    SortT: Sort<IndexT, ValueT>,
{
    #[inline(always)]
    fn null_position() -> IndexT {
        IndexT::max_value()
    }

    #[inline(always)]
    pub fn capacity(&self) -> usize {
        self.position_array.len()
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.heap_array.len()
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.heap_array.is_empty()
    }

    #[inline(always)]
    pub fn first_key_value(&self) -> Option<&(IndexT, ValueT)> {
        self.heap_array.first()
    }

    #[inline(always)]
    pub fn contains_key(&self, index: IndexT) -> bool {
        self.position_array[index.to_usize().unwrap()] != Self::null_position()
    }

    #[inline(always)]
    pub fn get(&self, index: IndexT) -> Option<&ValueT> {
        let p = self.position_array[index.to_usize().unwrap()];
        if p != Self::null_position() {
            Some(&self.heap_array[p.to_usize().unwrap()].1)
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn iter(&self) -> impl Iterator<Item = &(IndexT, ValueT)> {
        self.heap_array.iter()
    }

    #[inline(never)]
    pub fn reserve(&mut self, additional: usize) {
        self.heap_array.reserve(additional);
        self.position_array.extend(std::iter::repeat(Self::null_position()).take(additional));
    }

    #[inline(never)]
    pub fn clear(&mut self) {
        for (p, (i, _)) in self.heap_array.iter().enumerate() {
            assert!(self.position_array[i.to_usize().unwrap()].to_usize().unwrap() == p);
            self.position_array[i.to_usize().unwrap()] = Self::null_position();
        }
        self.heap_array.clear();
    }

    #[inline(always)]
    pub fn insert(&mut self, index: IndexT, value: ValueT) {
        let p = self.position_array[index.to_usize().unwrap()];
        if p == Self::null_position() {
            let p = IndexT::from(self.heap_array.len()).unwrap();
            self.position_array[index.to_usize().unwrap()] = p;
            self.heap_array.push((index, value));
            self.sort.up_heap(&mut self.heap_array, &mut self.position_array, p);
        } else {
            let (i, v) = &mut self.heap_array[p.to_usize().unwrap()];
            assert!(*i == index);
            *v = value;
            self.sort.update(&mut self.heap_array, &mut self.position_array, p);
        }
    }

    #[inline(always)]
    pub fn pop_first(&mut self) -> Option<(IndexT, ValueT)> {
        if self.heap_array.len() == 0 {
            None
        } else {
            let (i, v) = self.heap_array.swap_remove(0usize);
            assert!(self.position_array[i.to_usize().unwrap()] == IndexT::zero());
            self.position_array[i.to_usize().unwrap()] = Self::null_position();
            if self.heap_array.len() != 0 {
                self.position_array[self.heap_array[0].0.to_usize().unwrap()] = IndexT::zero();
                self.sort.down_heap(&mut self.heap_array, &mut self.position_array, IndexT::zero());
            }
            Some((i, v))
        }
    }

    #[inline(always)]
    pub fn remove(&mut self, index: IndexT) -> Option<ValueT> {
        if self.position_array[index.to_usize().unwrap()] == Self::null_position() {
            None
        } else {
            assert!(self.heap_array.len() != 0);
            let position = self.position_array[index.to_usize().unwrap()];
            let (i, value) = self.heap_array.swap_remove(position.to_usize().unwrap());
            assert!(i == index);
            self.position_array[index.to_usize().unwrap()] = Self::null_position();
            if position.to_usize().unwrap() < self.heap_array.len() {
                self.position_array[self.heap_array[position.to_usize().unwrap()].0.to_usize().unwrap()] = position;
                self.sort.update(&mut self.heap_array, &mut self.position_array, position);
            }
            Some(value)
        }
    }
}

pub struct NotSort<IndexT, ValueT> {
    phantom: PhantomData<(IndexT, ValueT)>,
}

impl<IndexT, ValueT> Default for NotSort<IndexT, ValueT> {
    fn default() -> Self {
        NotSort { phantom: PhantomData::default() }
    }
}

impl<IndexT, ValueT> Sort<IndexT, ValueT> for NotSort<IndexT, ValueT> {
    #[inline(always)]
    fn update(&self, _: &mut Vec<(IndexT, ValueT)>, _: &mut Vec<IndexT>, _: IndexT) {}

    #[inline(always)]
    fn up_heap(&self, _: &mut Vec<(IndexT, ValueT)>, _: &mut Vec<IndexT>, _: IndexT) {}

    #[inline(always)]
    fn down_heap(&self, _: &mut Vec<(IndexT, ValueT)>, _: &mut Vec<IndexT>, _: IndexT) {}
}

pub struct CustomSort<IndexT, ValueT, ComparatorT>
where
    ComparatorT: Comparator<IndexT, ValueT>,
{
    phantom: PhantomData<(IndexT, ValueT, ComparatorT)>,
}

impl<IndexT, ValueT, ComparatorT> Default for CustomSort<IndexT, ValueT, ComparatorT>
where
    ComparatorT: Comparator<IndexT, ValueT>,
{
    fn default() -> Self {
        CustomSort { phantom: PhantomData::default() }
    }
}

impl<IndexT, ValueT, ComparatorT> Sort<IndexT, ValueT> for CustomSort<IndexT, ValueT, ComparatorT>
where
    IndexT: Unsigned + PrimInt,
    ComparatorT: Comparator<IndexT, ValueT>,
{
    #[inline(always)]
    fn update(&self, heap_array: &mut Vec<(IndexT, ValueT)>, position_array: &mut Vec<IndexT>, position: IndexT) {
        if position.to_usize().unwrap() != 0
            && ComparatorT::compare(
                &heap_array[(position.to_usize().unwrap() + 1) / 2 - 1],
                &heap_array[position.to_usize().unwrap()],
            ) == Ordering::Greater
        {
            self.up_heap(heap_array, position_array, position);
        } else {
            self.down_heap(heap_array, position_array, position);
        }
    }

    #[inline(never)]
    fn up_heap(&self, heap_array: &mut Vec<(IndexT, ValueT)>, position_array: &mut Vec<IndexT>, position: IndexT) {
        let mut current = position.to_usize().unwrap();
        loop {
            if current == 0 {
                break;
            }
            let parent = (current + 1) / 2 - 1;
            if ComparatorT::compare(&heap_array[parent], &heap_array[current]) == Ordering::Greater {
                heap_array.swap(parent, current);
                position_array
                    .swap(heap_array[parent].0.to_usize().unwrap(), heap_array[current].0.to_usize().unwrap());
                current = parent;
            } else {
                break;
            }
        }
    }

    #[inline(never)]
    fn down_heap(&self, heap_array: &mut Vec<(IndexT, ValueT)>, position_array: &mut Vec<IndexT>, position: IndexT) {
        let mut current = position.to_usize().unwrap();
        loop {
            let left = (current + 1) * 2 - 1;
            let right = (current + 1) * 2;
            if left >= heap_array.len() {
                break;
            }
            let smaller_child = if right >= heap_array.len()
                || ComparatorT::compare(&heap_array[left], &heap_array[right]) == Ordering::Less
            {
                left
            } else {
                right
            };
            if ComparatorT::compare(&heap_array[current], &heap_array[smaller_child]) == Ordering::Greater {
                heap_array.swap(current, smaller_child);
                position_array
                    .swap(heap_array[current].0.to_usize().unwrap(), heap_array[smaller_child].0.to_usize().unwrap());
                current = smaller_child;
            } else {
                break;
            }
        }
    }
}

pub type FiniteHeapedMap<IndexT, ValueT, ComparatorT> = Base<IndexT, ValueT, CustomSort<IndexT, ValueT, ComparatorT>>;

pub type FiniteMap<IndexT, ValueT> = Base<IndexT, ValueT, NotSort<IndexT, ValueT>>;
