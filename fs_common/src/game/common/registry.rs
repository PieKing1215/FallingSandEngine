use std::{
    borrow::Borrow,
    collections::{hash_map, HashMap},
    fmt::Debug,
    marker::PhantomData,
    sync::Arc,
};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct RegistryID<T> {
    value: Arc<String>,
    _phantom: PhantomData<T>,
}

impl<T> Debug for RegistryID<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("RegistryID").field(&self.value).finish()
    }
}

// allows calling `Registry<RegistryID<_>, _>::get` with a `&str` as argument
impl<T> Borrow<str> for RegistryID<T> {
    fn borrow(&self) -> &str {
        let s: &String = self.value.borrow();
        s.as_str()
    }
}

// need to do these impls manually since the PhantomData messes up derive

impl<T> PartialEq for RegistryID<T> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<T> std::cmp::Eq for RegistryID<T> {}

impl<T> std::hash::Hash for RegistryID<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.value.hash(state);
    }
}

impl<T> Clone for RegistryID<T> {
    fn clone(&self) -> Self {
        Self { value: self.value.clone(), _phantom: PhantomData }
    }
}

impl<S: Into<String>, T> From<S> for RegistryID<T> {
    fn from(value: S) -> Self {
        Self {
            value: Arc::new(value.into()),
            _phantom: PhantomData,
        }
    }
}

pub struct Registry<K: Eq + std::hash::Hash, V> {
    map: HashMap<K, V, ahash::RandomState>,
}

impl<K: Eq + std::hash::Hash, V> Registry<K, V> {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self { map: HashMap::default() }
    }

    pub fn register(&mut self, key: K, value: V) {
        self.map.insert(key, value);
    }

    #[inline]
    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: std::hash::Hash + Eq + ?Sized,
    {
        self.map.get(key)
    }
}

impl<'a, K: Eq + std::hash::Hash, V> IntoIterator for &'a Registry<K, V> {
    type Item = (&'a K, &'a V);
    type IntoIter = hash_map::Iter<'a, K, V>;

    #[inline]
    fn into_iter(self) -> hash_map::Iter<'a, K, V> {
        self.map.iter()
    }
}
