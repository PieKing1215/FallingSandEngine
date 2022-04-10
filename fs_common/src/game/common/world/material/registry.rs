use std::collections::HashMap;

pub struct Registry<K: Eq + std::hash::Hash + Copy + std::fmt::Debug, V: std::fmt::Debug> {
    map: HashMap<K, V>,
}

impl<K: Eq + std::hash::Hash + Copy + std::fmt::Debug, V: std::fmt::Debug> Registry<K, V> {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self { map: HashMap::new() }
    }

    pub fn register(&mut self, key: K, value: V) {
        self.map.insert(key, value);
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        self.map.get(key)
    }
}
