//! Implements basic, non-custom tries.
//! These are mainly used in benchmark comparisons against the
//! other trie varieties so that we have a baseline we can
//! compare against.
//!
//! vanilla-flavored :^)

use std::hash::Hash;

use bumpalo::Bump;
use hashbrown::{hash_map::DefaultHashBuilder, BumpWrapper, HashMap};

use crate::Trieish;

/// A vanilla hash trie!
/// Nothing special, just a bunch of nested HashMaps.
#[derive(Debug, Clone)]
pub struct Trie<T>(HashMap<T, Self>);

impl<T> Default for Trie<T> {
    fn default() -> Self {
        Self(HashMap::default())
    }
}

impl<'bump, T> Trieish<'bump> for Trie<T>
where
    T: Eq + Hash + Clone,
{
    type Value = T;
    type Tuple<'a> = &'a [T] where T: 'a;

    fn empty(_bump: &'bump Bump) -> Self {
        Self::default()
    }

    fn insert<'a>(&mut self, tuple: &'a [T], _arena: &'bump Bump) where T: 'a {
        let mut trie = self;
        for v in tuple {
            trie = trie.0.entry(v.clone()).or_default()
        }
    }

    fn query(&self, v: &Self::Value) -> bool {
        self.0.contains_key(v)
    }

    fn advance(&self, v: &Self::Value) -> Option<&Self> {
        self.0.get(v)
    }
}

/// A hash trie allocated on a bump allocator.
#[derive(Debug, Clone)]
pub struct BumpTrie<'a, T>(HashMap<T, Self, DefaultHashBuilder, BumpWrapper<'a>>);

impl<'b, T> Trieish<'b> for BumpTrie<'b, T>
where
    T: Eq + Hash + Clone,
{
    type Value = T;
    type Tuple<'a> = &'a [T] where T: 'a;

    fn empty(bump: &'b Bump) -> Self {
        Self(HashMap::new_in(BumpWrapper(bump)))
    }

    fn insert<'a>(&mut self, tuple: &'a [T], arena: &'b Bump) where T: 'a {
        let mut trie = self;
        for v in tuple {
            trie = trie.0.entry(v.clone()).or_insert_with(|| Self::empty(arena))
        }
    }

    fn query(&self, v: &Self::Value) -> bool {
        self.0.contains_key(v)
    }

    fn advance(&self, v: &Self::Value) -> Option<&Self> {
        self.0.get(v)
    }
}
