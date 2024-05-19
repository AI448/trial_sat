use std::{cmp::Ordering, marker::PhantomData};

pub trait Comparator<V> {
    fn compare(lhs: &(usize, V), rhs: &(usize, V)) -> Ordering;
}

pub struct FiniteHeapedMap<V, F>
where
    F: Comparator<V>,
{
    heap_array: Vec<(usize, V)>,
    position_array: Vec<usize>,
    _phantom: PhantomData<F>,
}

impl<V, F> Default for FiniteHeapedMap<V, F>
where
    F: Comparator<V>,
{
    #[inline(never)]
    fn default() -> Self {
        FiniteHeapedMap { heap_array: Vec::default(), position_array: Vec::default(), _phantom: PhantomData }
    }
}

impl<V, F> FiniteHeapedMap<V, F>
where
    F: Comparator<V>, // TODO: Comparator ではなく GetSortKey とかの方がいいかも(MEMO: その場合弱順序対応に注意)
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

    #[inline(never)]
    pub fn insert(&mut self, index: usize, value: V) {
        let p = self.position_array[index];
        if self.position_array[index] == Self::NULL_POSITION {
            let p = self.heap_array.len();
            self.position_array[index] = p;
            self.heap_array.push((index, value));
            self.up_heap(p);
        } else {
            let (i, v) = &mut self.heap_array[p];
            assert!(*i == index);
            *v = value;
            self.update(p);
        }
    }

    #[inline(never)]
    pub fn pop_first(&mut self) -> Option<(usize, V)> {
        if self.heap_array.len() == 0 {
            None
        } else {
            let (i, v) = self.heap_array.swap_remove(0);
            assert!(self.position_array[i] == 0);
            self.position_array[i] = Self::NULL_POSITION;
            if self.heap_array.len() != 0 {
                self.position_array[self.heap_array[0].0] = 0;
                self.down_heap(0);
            }
            Some((i, v))
        }
    }

    #[inline(never)]
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
                self.update(position);
            }
            Some(value)
        }
    }

    // private:

    #[inline(never)]
    fn update(&mut self, position: usize) {
        if position != 0
            && F::compare(&self.heap_array[(position + 1) / 2 - 1], &self.heap_array[position]) == Ordering::Greater
        {
            self.up_heap(position);
        } else {
            self.down_heap(position);
        }
    }

    #[inline(never)]
    fn up_heap(&mut self, position: usize) {
        let mut current = position;
        loop {
            if current == 0 {
                break;
            }
            let parent = (current + 1) / 2 - 1;
            if F::compare(&self.heap_array[parent], &self.heap_array[current]) == Ordering::Greater {
                self.heap_array.swap(parent, current);
                self.position_array.swap(self.heap_array[parent].0, self.heap_array[current].0);
                current = parent;
            } else {
                break;
            }
        }
    }

    #[inline(never)]
    fn down_heap(&mut self, position: usize) {
        let mut current = position;
        loop {
            let left = (current + 1) * 2 - 1;
            let right = (current + 1) * 2;
            if left >= self.heap_array.len() {
                break;
            }
            let smaller_child;
            if right >= self.heap_array.len()
                || F::compare(&self.heap_array[left], &self.heap_array[right]) == Ordering::Less
            {
                smaller_child = left;
            } else {
                smaller_child = right
            }
            if F::compare(&self.heap_array[current], &self.heap_array[smaller_child]) == Ordering::Greater {
                self.heap_array.swap(current, smaller_child);
                self.position_array.swap(self.heap_array[current].0, self.heap_array[smaller_child].0);
                current = smaller_child;
            } else {
                break;
            }
        }
    }
}
