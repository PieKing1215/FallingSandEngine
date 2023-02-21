use std::{
    borrow::Borrow,
    collections::{hash_map, HashMap},
    fmt::{Debug, Display},
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

impl<T> Display for RegistryID<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
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

impl<T> PartialOrd for RegistryID<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.value.partial_cmp(&other.value)
    }
}

impl<T> Ord for RegistryID<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.value.cmp(&other.value)
    }
}

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

pub struct Registry<V> {
    map: HashMap<RegistryID<V>, V, ahash::RandomState>,
}

impl<V> Registry<V> {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self { map: HashMap::default() }
    }

    pub fn register(&mut self, key: impl Into<RegistryID<V>>, value: V) {
        self.map.insert(key.into(), value);
    }

    #[inline]
    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        RegistryID<V>: Borrow<Q>,
        Q: std::hash::Hash + Eq + ?Sized,
    {
        self.map.get(key)
    }
}

impl<'a, V> IntoIterator for &'a Registry<V> {
    type Item = (&'a RegistryID<V>, &'a V);
    type IntoIter = hash_map::Iter<'a, RegistryID<V>, V>;

    #[inline]
    fn into_iter(self) -> hash_map::Iter<'a, RegistryID<V>, V> {
        self.map.iter()
    }
}
