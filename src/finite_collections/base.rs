use std::{cmp::Ordering, marker::PhantomData};

pub trait Comparator<V> {
    fn compare(lhs: &(usize, V), rhs: &(usize, V)) -> Ordering;
}

pub trait Sort<V>: Default {
    fn update(&self, heap_array: &mut Vec<(usize, V)>, position_array: &mut Vec<usize>, position: usize);
    fn up_heap(&self, heap_array: &mut Vec<(usize, V)>, position_array: &mut Vec<usize>, position: usize);
    fn down_heap(&self, heap_array: &mut Vec<(usize, V)>, position_array: &mut Vec<usize>, position: usize);
}

pub struct Base<V, S>
where
    S: Sort<V>,
{
    heap_array: Vec<(usize, V)>,
    position_array: Vec<usize>,
    sort: S,
}

impl<V, S> Default for Base<V, S>
where
    S: Sort<V>,
{
    #[inline(never)]
    fn default() -> Self {
        Base { heap_array: Vec::default(), position_array: Vec::default(), sort: S::default() }
    }
}

impl<V, S> Base<V, S>
where
    S: Sort<V>,
{
    const NULL_POSITION: usize = usize::MAX;

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
    pub fn first_key_value(&self) -> Option<&(usize, V)> {
        self.heap_array.first()
    }

    #[inline(always)]
    pub fn contains_key(&self, index: usize) -> bool {
        self.position_array[index] != Self::NULL_POSITION
    }

    #[inline(always)]
    pub fn get(&self, index: usize) -> Option<&V> {
        let p = self.position_array[index];
        if p != Self::NULL_POSITION {
            Some(&self.heap_array[p].1)
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn iter(&self) -> impl Iterator<Item = &(usize, V)> {
        self.heap_array.iter()
    }

    #[inline(never)]
    pub fn reserve(&mut self, additional: usize) {
        self.heap_array.reserve(additional);
        self.position_array.extend(std::iter::repeat(Self::NULL_POSITION).take(additional));
    }

    #[inline(never)]
    pub fn clear(&mut self) {
        for (p, (i, _)) in self.heap_array.iter().enumerate() {
            assert!(self.position_array[*i] == p);
            self.position_array[*i] = Self::NULL_POSITION;
        }
        self.heap_array.clear();
    }

    #[inline(always)]
    pub fn insert(&mut self, index: usize, value: V) {
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
    pub fn pop_first(&mut self) -> Option<(usize, V)> {
        if self.heap_array.len() == 0 {
            None
        } else {
            let (i, v) = self.heap_array.swap_remove(0);
            assert!(self.position_array[i] == 0);
            self.position_array[i] = Self::NULL_POSITION;
            if self.heap_array.len() != 0 {
                self.position_array[self.heap_array[0].0] = 0;
                self.sort.down_heap(&mut self.heap_array, &mut self.position_array, 0);
            }
            Some((i, v))
        }
    }

    #[inline(always)]
    pub fn remove(&mut self, index: usize) -> Option<V> {
        if self.position_array[index] == Self::NULL_POSITION {
            None
        } else {
            assert!(self.heap_array.len() != 0);
            let position = self.position_array[index];
            let (i, value) = self.heap_array.swap_remove(position);
            assert!(i == index);
            self.position_array[index] = Self::NULL_POSITION;
            if position < self.heap_array.len() {
                self.position_array[self.heap_array[position].0] = position;
                self.sort.update(&mut self.heap_array, &mut self.position_array, position);
            }
            Some(value)
        }
    }
}

pub struct NotSort<V> {
    phantom: PhantomData<V>,
}

impl<V> Default for NotSort<V> {
    fn default() -> Self {
        NotSort { phantom: PhantomData }
    }
}

impl<V> Sort<V> for NotSort<V> {
    #[inline(always)]
    fn update(&self, _: &mut Vec<(usize, V)>, _: &mut Vec<usize>, _: usize) {}

    #[inline(always)]
    fn up_heap(&self, _: &mut Vec<(usize, V)>, _: &mut Vec<usize>, _: usize) {}

    #[inline(always)]
    fn down_heap(&self, _: &mut Vec<(usize, V)>, _: &mut Vec<usize>, _: usize) {}
}

pub struct CustomSort<V, C>
where
    C: Comparator<V>,
{
    phantom_v: PhantomData<V>,
    phantom_c: PhantomData<C>,
}

impl<V, C> Default for CustomSort<V, C>
where
    C: Comparator<V>,
{
    fn default() -> Self {
        CustomSort { phantom_v: PhantomData, phantom_c: PhantomData }
    }
}

impl<V, C> Sort<V> for CustomSort<V, C>
where
    C: Comparator<V>,
{
    #[inline(always)]
    fn update(&self, heap_array: &mut Vec<(usize, V)>, position_array: &mut Vec<usize>, position: usize) {
        if position != 0 && C::compare(&heap_array[(position + 1) / 2 - 1], &heap_array[position]) == Ordering::Greater
        {
            self.up_heap(heap_array, position_array, position);
        } else {
            self.down_heap(heap_array, position_array, position);
        }
    }

    #[inline(never)]
    fn up_heap(&self, heap_array: &mut Vec<(usize, V)>, position_array: &mut Vec<usize>, position: usize) {
        let mut current = position;
        loop {
            if current == 0 {
                break;
            }
            let parent = (current + 1) / 2 - 1;
            if C::compare(&heap_array[parent], &heap_array[current]) == Ordering::Greater {
                heap_array.swap(parent, current);
                position_array.swap(heap_array[parent].0, heap_array[current].0);
                current = parent;
            } else {
                break;
            }
        }
    }

    #[inline(never)]
    fn down_heap(&self, heap_array: &mut Vec<(usize, V)>, position_array: &mut Vec<usize>, position: usize) {
        let mut current = position;
        loop {
            let left = (current + 1) * 2 - 1;
            let right = (current + 1) * 2;
            if left >= heap_array.len() {
                break;
            }
            let smaller_child =
                if right >= heap_array.len() || C::compare(&heap_array[left], &heap_array[right]) == Ordering::Less {
                    left
                } else {
                    right
                };
            if C::compare(&heap_array[current], &heap_array[smaller_child]) == Ordering::Greater {
                heap_array.swap(current, smaller_child);
                position_array.swap(heap_array[current].0, heap_array[smaller_child].0);
                current = smaller_child;
            } else {
                break;
            }
        }
    }
}

pub type FiniteHeapedMap<V, C> = Base<V, CustomSort<V, C>>;

pub type FiniteMap<V> = Base<V, NotSort<V>>;
