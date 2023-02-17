use std::collections::{hash_map, HashMap};

pub struct Registry<K: Eq + std::hash::Hash + Copy, V> {
    map: HashMap<K, V, ahash::RandomState>,
}

impl<K: Eq + std::hash::Hash + Copy, V> Registry<K, V> {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self { map: HashMap::default() }
    }

    pub fn register(&mut self, key: K, value: V) {
        self.map.insert(key, value);
    }

    #[inline]
    pub fn get(&self, key: &K) -> Option<&V> {
        self.map.get(key)
    }
}

impl<'a, K: Eq + std::hash::Hash + Copy, V> IntoIterator for &'a Registry<K, V> {
    type Item = (&'a K, &'a V);
    type IntoIter = hash_map::Iter<'a, K, V>;

    #[inline]
    fn into_iter(self) -> hash_map::Iter<'a, K, V> {
        self.map.iter()
    }
}
