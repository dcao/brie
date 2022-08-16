//! Implements basic, non-custom tries.
//! These are mainly used in benchmark comparisons against the
//! other trie varieties so that we have a baseline we can
//! compare against.
//!
//! vanilla-flavored :^)

use std::hash::Hash;

use bumpalo::Bump;
use hashbrown::{hash_map::DefaultHashBuilder, BumpWrapper, HashMap};

use crate::Oneshot;

/// A vanilla hash trie!
/// Nothing special, just a bunch of nested HashMaps.
#[derive(Debug, Clone)]
pub struct Trie<T>(pub HashMap<T, Self>);

impl<T> Default for Trie<T> {
    fn default() -> Self {
        Self(HashMap::default())
    }
}

impl<'bump, T, const N: usize> Oneshot<'bump, N> for Trie<T>
where
    T: Eq + Hash + Clone,
    T: 'bump,
{
    type Value = T;
    type KeyIter<const M: usize> = impl Iterator<Item = &'bump T>;

    fn from_iter<I: IntoIterator<Item = [T; N]>>(iter: I, bump: &'bump Bump) -> Self {
        let mut res = Self::default();

        for tuple in iter.into_iter() {
            let mut trie = &mut res;
            for v in tuple {
                trie = trie.0.entry(v.clone()).or_default()
            }
        }

        res
    }

    fn intersect<'a, 't: 'bump, const M: usize>(
        &'t self,
        others: [&'t Self; M],
    ) -> Self::KeyIter<M> {
        self.0
            .keys()
            .filter(move |k| others.iter().all(|idx| idx.0.contains_key(k)))
    }

    fn advance(mut self, v: &Self::Value) -> Option<Self> {
        // TODO: technically not optimal but...
        self.0.remove(v)
    }
}

impl<'b, T> BumpTrie<'b, T>
where
    T: Eq + Hash + Clone,
{
    fn new_in(bump: &'b Bump) -> Self {
        Self(HashMap::new_in(BumpWrapper(bump)))
    }
}

/// A hash trie allocated on a bump allocator.
#[derive(Debug, Clone)]
pub struct BumpTrie<'a, T>(pub HashMap<T, Self, DefaultHashBuilder, BumpWrapper<'a>>);

impl<'bump, T, const N: usize> Oneshot<'bump, N> for BumpTrie<'bump, T>
where
    T: Eq + Hash + Clone,
    T: 'bump,
{
    type Value = T;
    type KeyIter<const M: usize> = impl Iterator<Item = &'bump T>;

    fn from_iter<I: IntoIterator<Item = [T; N]>>(iter: I, bump: &'bump Bump) -> Self {
        let mut res = Self::new_in(bump);

        for tuple in iter.into_iter() {
            let mut trie = &mut res;
            for v in tuple {
                trie = trie.0.entry(v.clone()).or_insert_with(|| Self::new_in(bump))
            }
        }

        res
    }

    fn intersect<'a, 't: 'bump, const M: usize>(
        &'t self,
        others: [&'t Self; M],
    ) -> Self::KeyIter<M> {
        self.0
            .keys()
            .filter(move |k| others.iter().all(|idx| idx.0.contains_key(k)))
    }

    fn advance(mut self, v: &Self::Value) -> Option<Self> {
        // TODO: technically not optimal but...
        self.0.remove(v)
    }
}