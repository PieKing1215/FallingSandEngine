use std::{
    collections::HashMap,
    hash::{BuildHasher, Hash},
};

use itertools::Itertools;

/// Workaround for <https://github.com/rust-lang/hashbrown/issues/332>
#[allow(clippy::missing_safety_doc)]
pub trait HashMapExt {
    type K;
    type V;

    unsafe fn get_many_var_unchecked_mut(&mut self, keys: &[Self::K]) -> Option<Vec<&mut Self::V>>;
    fn get_many_var_mut(&mut self, keys: &[Self::K]) -> Option<Vec<&mut Self::V>>;
}

impl<K: Eq + Hash, V, S: BuildHasher> HashMapExt for HashMap<K, V, S> {
    type K = K;
    type V = V;

    unsafe fn get_many_var_unchecked_mut(&mut self, keys: &[K]) -> Option<Vec<&mut V>> {
        let out = keys
            .iter()
            .map(|k| self.get_mut(k).map(|v| &mut *(v as &mut _ as *mut _)))
            .collect();
        out
    }

    fn get_many_var_mut(&mut self, keys: &[K]) -> Option<Vec<&mut V>> {
        let unique = keys.iter().duplicates().next().is_none();
        if unique {
            unsafe { self.get_many_var_unchecked_mut(keys) }
        } else {
            None
        }
    }
}
