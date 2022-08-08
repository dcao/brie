//! Implements basic, non-custom tries.
//! These are mainly used in benchmark comparisons against the
//! other trie varieties so that we have a baseline we can
//! compare against.
//!
//! vanilla-flavored :^)

use std::hash::Hash;

use bumpalo::Bump;
use hashbrown::{hash_map::DefaultHashBuilder, BumpWrapper, HashMap};

/// A vanilla hash trie!
/// Nothing special, just a bunch of nested HashMaps.
#[derive(Debug, Clone)]
pub struct Trie<T>(HashMap<T, Self>);

impl<T> Default for Trie<T> {
    fn default() -> Self {
        Self(HashMap::default())
    }
}

impl<T> Trie<T>
where
    T: Eq + Hash + Clone,
{
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, shuffle: &[usize], tuple: &[T]) {
        // debug_assert_eq!(shuffle.len(), tuple.len());
        debug_assert!(shuffle.len() <= tuple.len());
        let mut trie = self;
        for i in shuffle {
            trie = trie.0.entry(tuple[*i].clone()).or_default()
        }
    }

    pub fn insert_tuple(&mut self, tuple: &[T]) {
        let mut trie = self;
        for v in tuple {
            trie = trie.0.entry(v.clone()).or_default()
        }
    }
}

/// A hash trie allocated on a bump allocator.
#[derive(Debug, Clone)]
pub struct BumpTrie<'a, T>(HashMap<T, Self, DefaultHashBuilder, BumpWrapper<'a>>);

impl<'a, T> BumpTrie<'a, T>
where
    T: Eq + Hash + Clone,
{
    pub fn new_in(arena: &'a Bump) -> Self {
        Self(HashMap::new_in(BumpWrapper(arena)))
    }

    pub fn insert(&mut self, arena: &'a Bump, shuffle: &[usize], tuple: &[T]) {
        // debug_assert_eq!(shuffle.len(), tuple.len());
        debug_assert!(shuffle.len() <= tuple.len());
        let mut trie = self;
        for i in shuffle {
            trie = trie
                .0
                .entry(tuple[*i].clone())
                .or_insert_with(|| Self::new_in(arena));
        }
    }

    pub fn insert_tuple(&mut self, arena: &'a Bump, tuple: &[T]) {
        let mut trie = self;
        for v in tuple {
            trie = trie
                .0
                .entry(v.clone())
                .or_insert_with(|| Self::new_in(arena))
        }
    }
}
