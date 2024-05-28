use super::base;
use super::size::Size;

pub struct Set<SizeT>
where
    SizeT: Size,
{
    map: base::Base<SizeT, (), base::NotSort<SizeT, ()>>,
}

impl<SizeT> Default for Set<SizeT>
where
    SizeT: Size,
{
    fn default() -> Self {
        Set { map: base::Base::default() }
    }
}

impl<SizeT> Set<SizeT>
where
    SizeT: Size,
{
    pub fn capacity(&self) -> SizeT {
        self.map.capacity()
    }

    pub fn len(&self) -> SizeT {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn contains_key(&self, index: SizeT) -> bool {
        self.map.contains_key(index)
    }

    pub fn iter(&self) -> impl Iterator<Item = &SizeT> {
        self.map.iter().map(|(index, ..)| index)
    }

    pub fn reserve(&mut self, additional: SizeT) {
        self.map.reserve(additional);
    }

    pub fn clear(&mut self) {
        self.map.clear();
    }

    pub fn insert(&mut self, index: SizeT) {
        self.map.insert(index, ());
    }

    pub fn remove(&mut self, index: SizeT) {
        self.map.remove(index);
    }
}
