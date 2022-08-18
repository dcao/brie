//! Implements basic, non-custom tries.
//! These are mainly used in benchmark comparisons against the
//! other trie varieties so that we have a baseline we can
//! compare against.
//!
//! vanilla-flavored :^)

use std::{
    hash::{Hash, Hasher},
    marker::PhantomData,
    ptr::NonNull,
    task::RawWakerVTable,
};

use bumpalo::{boxed::Box, Bump};
use hashbrown::{hash_map::DefaultHashBuilder, raw::RawTable, BumpWrapper, HashMap};

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
    type IVal = &'bump T;
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

    fn intersect<'a, const M: usize>(&'bump self, others: [&'bump Self; M]) -> Self::KeyIter<M> {
        self.0
            .keys()
            .filter(move |k| others.iter().all(|idx| idx.0.contains_key(k)))
    }

    fn advance(&'bump self, v: &Self::Value) -> Option<&'bump Self> {
        self.0.get(v)
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
    type IVal = &'bump T;
    type KeyIter<const M: usize> = impl Iterator<Item = &'bump T>;

    fn from_iter<I: IntoIterator<Item = [T; N]>>(iter: I, bump: &'bump Bump) -> Self {
        let mut res = Self::new_in(bump);

        for tuple in iter.into_iter() {
            let mut trie = &mut res;
            for v in tuple {
                trie = trie
                    .0
                    .entry(v.clone())
                    .or_insert_with(|| Self::new_in(bump))
            }
        }

        res
    }

    fn intersect<'a, const M: usize>(&'bump self, others: [&'bump Self; M]) -> Self::KeyIter<M> {
        self.0
            .keys()
            .filter(move |k| others.iter().all(|idx| idx.0.contains_key(k)))
    }

    fn advance(&'bump self, v: &Self::Value) -> Option<&'bump Self> {
        self.0.get(v)
    }
}

pub struct FancyTrie<'a, T>(RawTable<Entry<'a, T>, BumpWrapper<'a>>);

pub struct Entry<'a, T> {
    hash: u64,
    ptr: Ptr<'a, T>,
}

pub enum Ptr<'a, T> {
    Data(NonNull<Data<'a, T>>),
    Trie(FancyTrie<'a, T>),
}

impl<'a, T> Ptr<'a, T> {
    fn get_trie(&mut self) -> Option<&mut FancyTrie<'a, T>> {
        match self {
            Ptr::Data(_) => None,
            Ptr::Trie(ft) => Some(ft),
        }
    }

    fn get_data(&mut self) -> Option<&mut Data<'a, T>> {
        match self {
            Ptr::Data(d) => Some(unsafe { d.as_mut() }),
            Ptr::Trie(_) => None,
        }
    }
}

pub struct Data<'a, T> {
    data: T,
    next: Option<NonNull<Data<'a, T>>>,
    _p: PhantomData<&'a T>,
    // next: Option<&'a usize>,
}

impl<'bump, T, const N: usize> Oneshot<'bump, N> for FancyTrie<'bump, [T; N]>
where
    T: Eq + Hash + Clone,
    T: 'bump,
{
    type Value = T;
    type IVal = &'bump Entry<'bump, [T; N]>;
    type KeyIter<const M: usize> = impl Iterator<Item = Self::IVal>;

    fn from_iter<I: IntoIterator<Item = [T; N]>>(iter: I, bump: &'bump Bump) -> Self {
        let mut res = Self(RawTable::new_in(BumpWrapper(bump)));

        for tuple in iter.into_iter() {
            let mut trie = &mut res;
            for (i, v) in tuple.iter().enumerate() {
                let hash = {
                    let mut hasher = wyhash::WyHash::default();
                    v.hash(&mut hasher);
                    hasher.finish()
                };
                if i == N - 1 {
                    let d = Box::new_in(
                        Data {
                            data: tuple.clone(),
                            next: None,
                            _p: PhantomData,
                        },
                        bump,
                    );

                    if let Some(v) = trie.0.get_mut(hash, |x| x.hash == hash) {
                        let mut t = v.ptr.get_data().unwrap();

                        while t.next.is_some() {
                            // We can't use while let here bc of borrowck limits
                            // with linked lists
                            // fuckin rust smfh
                            t = unsafe { t.next.unwrap().as_mut() };
                        }
                        t.next = Some(NonNull::from(Box::leak(d)));
                    } else {
                        let value = Entry {
                            hash,
                            ptr: Ptr::Data(NonNull::from(Box::leak(d))),
                        };
                        trie.0
                            .insert_entry(hash, value, |v| v.hash)
                            .ptr
                            .get_data()
                            .unwrap();
                    };
                } else {
                    trie = if let Some(b) = trie.0.find(hash, |x| x.hash == hash) {
                        unsafe { b.as_mut().ptr.get_trie().unwrap() }
                    } else {
                        let value = Entry {
                            hash,
                            ptr: Ptr::Trie(Self(RawTable::new_in(BumpWrapper(bump)))),
                        };
                        trie.0
                            .insert_entry(hash, value, |v| v.hash)
                            .ptr
                            .get_trie()
                            .unwrap()
                    };
                }
            }
        }

        res
    }

    fn intersect<'a, const M: usize>(&'bump self, others: [&'bump Self; M]) -> Self::KeyIter<M> {
        unsafe {
            self.0
                .iter()
                .filter(move |v| {
                    let v = v.as_ptr();
                    let hash = {
                        let mut hasher = wyhash::WyHash::default();
                        v.hash(&mut hasher);
                        hasher.finish()
                    };
                    others
                        .iter()
                        .all(|idx| idx.0.get(hash, |o| o.hash == hash).is_some())
                })
                .map(|x| x.as_ref())
        }
    }

    fn advance(&'bump self, v: &Self::Value) -> Option<&'bump Self> {
        todo!()
    }
}
