use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

pub struct SplitWrapper<'a, K: Eq + std::hash::Hash, V> {
    pub key: Option<K>,
    pub inner: Option<(V, &'a mut HashMap<K, V>)>,
}

impl<'a, K: Eq + std::hash::Hash, V> SplitWrapper<'a, K, V> {
    pub fn inner(&mut self) -> &mut (V, &'a mut HashMap<K, V>) {
        self.inner.as_mut().unwrap()
    }
}

impl<K: Eq + std::hash::Hash, V> Drop for SplitWrapper<'_, K, V> {
    fn drop(&mut self) {
        let inner = self.inner.take().unwrap();
        inner.1.insert(self.key.take().unwrap(), inner.0);
    }
}

impl<'a, K: Eq + std::hash::Hash, V> Deref for SplitWrapper<'a, K, V> {
    type Target = (V, &'a mut HashMap<K, V>);

    fn deref(&self) -> &Self::Target {
        self.inner.as_ref().unwrap()
    }
}

impl<'a, K: Eq + std::hash::Hash, V> DerefMut for SplitWrapper<'a, K, V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner.as_mut().unwrap()
    }
}

// TODO: docs
pub trait GetSplit<'a, K: Eq + std::hash::Hash + Clone, V> {
    fn split(&mut self, key: &K) -> Option<SplitWrapper<K, V>>;
}

#[allow(clippy::implicit_hasher)]
impl<'a, K: Clone + std::hash::Hash + Eq, V> GetSplit<'a, K, V> for HashMap<K, V> {
    fn split(&mut self, key: &K) -> Option<SplitWrapper<K, V>> {
        let m = self.remove(key)?;
        Some(SplitWrapper { key: Some(key.clone()), inner: Some((m, self)) })
    }
}
