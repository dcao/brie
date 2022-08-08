//! Sorted maps/tries, backed by a RawVec

use bumpalo::Bump;

use core::{ptr, slice, fmt};
use std::{cmp::Ordering, ops};

use crate::raw::RawVec;

// TODO: bench SoA approach:
//       two bufs, one for key, one for val
pub struct Map<'bump, K: 'bump, V: 'bump> {
    buf: RawVec<'bump, (K, V)>,
    len: usize,
}

impl<'bump, K, V> Map<'bump, K, V>
where
    K: 'bump + Ord + Eq,
    V: 'bump,
{
    #[inline]
    pub fn new() -> Self {
        Self {
            buf: RawVec::new(),
            len: 0,
        }
    }

    #[inline]
    pub fn with_capacity(cap: usize, bump: &'bump Bump) -> Self {
        Self {
            buf: RawVec::with_capacity_in(cap, bump),
            len: 0,
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    pub fn reserve(&mut self, additional: usize, bump: &'bump Bump) {
        self.buf.reserve(self.len, additional, bump);
    }

    #[inline]
    pub unsafe fn set_len(&mut self, new_len: usize) {
        self.len = new_len;
    }

    fn write_at_ix(&mut self, index: usize, element: (K, V)) -> (K, V) {
        let len = self.len();
        debug_assert!(index <= len);

        unsafe {
            // infallible
            // The spot to put the new value
            let p = self.as_mut_ptr().add(index);
            let v = ptr::read(p);
            // Write it in!
            ptr::write(p, element);

            v
        }
    }

    fn insert_at_ix(&mut self, index: usize, element: (K, V), bump: &'bump Bump) -> &mut (K, V) {
        let len = self.len();
        debug_assert!(index <= len);

        // space for the new element
        if len == self.buf.cap() {
            self.reserve(1, bump);
        }

        unsafe {
            // infallible
            // The spot to put the new value
            let p = self.as_mut_ptr().add(index);
            // Shift everything over to make space. (Duplicating the
            // `index`th element into two consecutive places.)
            ptr::copy(p, p.offset(1), len - index);
            // Write it in, overwriting the first copy of the `index`th
            // element.
            ptr::write(p, element);
            
            self.set_len(len + 1);

            &mut *p
        }
    }

    pub fn get(&self, k: &K) -> Option<&V> {
        self.binary_search_by(|v| k.cmp(v))
            .ok()
            // SAFETY: binary_search_by guarantees x < len
            .map(|x| &unsafe { self.get_unchecked(x) }.1)
    }

    pub fn insert(&mut self, k: K, v: V, bump: &'bump Bump) -> Option<V> {
        match self.binary_search_by(|v| k.cmp(v)) {
            Ok(found) => Some(self.write_at_ix(found, (k, v)).1),
            Err(none) => {
                self.insert_at_ix(none, (k, v), bump);
                None
            }
        }
    }

    // TODO: maybe actually impl the entry api
    pub fn get_or_insert<F>(&mut self, k: K, mut vf: F, bump: &'bump Bump) -> &mut V
    where
        F: FnMut() -> V
    {
        match self.binary_search_by(|v| k.cmp(v)) {
            // SAFETY: binary_search_by guarantees found < len
            Ok(found) => unsafe { &mut self.get_unchecked_mut(found).1 }
            Err(none) => {
                &mut self.insert_at_ix(none, (k, vf()), bump).1
            }
        }
    }


    #[inline]
    pub fn binary_search_by<'a, F>(&'a self, mut f: F) -> Result<usize, usize>
    where
        F: FnMut(&'a K) -> Ordering,
    {
        let mut size = self.len();
        let mut left = 0;
        let mut right = size;
        while left < right {
            let mid = left + size / 2;

            // SAFETY: the call is made safe by the following invariants:
            // - `mid >= 0`
            // - `mid < size`: `mid` is limited by `[left; right)` bound.
            let cmp = f(&unsafe { self.get_unchecked(mid) }.0);

            // The reason why we use if/else control flow rather than match
            // is because match reorders comparison operations, which is perf sensitive.
            // This is x86 asm for u8: https://rust.godbolt.org/z/8Y8Pra.
            if cmp == Ordering::Less {
                left = mid + 1;
            } else if cmp == Ordering::Greater {
                right = mid;
            } else {
                // SAFETY: same as the `get_unchecked` above
                unsafe { core::intrinsics::assume(mid < self.len()) };
                return Ok(mid);
            }

            size = right - left;
        }
        Err(left)
    }
}

impl<'bump, K: 'bump, V: 'bump> ops::Deref for Map<'bump, K, V> {
    type Target = [(K, V)];

    fn deref(&self) -> &[(K, V)] {
        unsafe {
            let p = self.buf.ptr();
            // assume(!p.is_null());
            slice::from_raw_parts(p, self.len)
        }
    }
}

impl<'bump, K: 'bump, V: 'bump> ops::DerefMut for Map<'bump, K, V> {
    fn deref_mut(&mut self) -> &mut [(K, V)] {
        unsafe {
            let ptr = self.buf.ptr();
            // assume(!ptr.is_null());
            slice::from_raw_parts_mut(ptr, self.len)
        }
    }
}

impl<'bump, K: 'bump + fmt::Debug, V: 'bump + fmt::Debug> fmt::Debug for Map<'bump, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

pub struct Trie<'a, T>(Map<'a, T, Self>);

impl<'a, T> Trie<'a, T>
where
    T: Ord + Eq + Clone
{
    pub fn new() -> Self {
        Self(Map::new())
    }

    pub fn insert(&mut self, arena: &'a Bump, shuffle: &[usize], tuple: &[T]) {
        // debug_assert_eq!(shuffle.len(), tuple.len());
        debug_assert!(shuffle.len() <= tuple.len());
        let mut trie = self;
        for i in shuffle {
            trie = trie
                .0
                .get_or_insert(tuple[*i].clone(), Self::new, arena);
        }
    }

    pub fn insert_tuple(&mut self, arena: &'a Bump, tuple: &[T]) {
        let mut trie = self;
        for v in tuple {
            trie = trie
                .0
                .get_or_insert(v.clone(), Self::new, arena);
        }
    }
}
