use super::base;



pub struct Set {
    map: base::Base<(), base::NotSort<()>>,
}

impl Default for Set {

    fn default() -> Self {
        Set{map: base::Base::default()}
    }
}

impl Set {

    pub fn capacity(&self) -> usize {
        self.map.capacity()
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn contains_key(&self, index: usize) -> bool {
        self.map.contains_key(index)
    }

    pub fn iter(&self) -> impl Iterator<Item = &usize> {
        self.map.iter().map(|(index, ..)| index)
    }

    pub fn reserve(&mut self, additional: usize) {
        self.map.reserve(additional);
    }

    pub fn clear(&mut self) {
        self.map.clear();
    }

    pub fn insert(&mut self, index: usize) {
        self.map.insert(index, ());
    }

    pub fn remove(&mut self, index: usize) {
        self.map.remove(index);
    }


}
