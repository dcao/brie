//! Experimental flattened hash trie impl

use std::{
    hash::{Hash, Hasher},
    iter::{self, FusedIterator},
    marker::PhantomData,
    mem::MaybeUninit,
};

use bumpalo::Bump;
use itertools::Itertools;

use crate::{sorted::vec::BumpVec, Oneshot};

// returns (cap, total_bits, hash_bits)
fn get_bit_sizes<const N: usize>(iter_len: usize) -> (usize, u32, u32) {
    let iter_len = iter_len * N;
    let (hash_bits, hash_cap) = {
        let size = iter_len as f64;
        let upsize = (size * 1.25).ceil() as usize;
        let v = upsize.next_power_of_two();
        (v.checked_log2().unwrap(), v)
    };
    let lvl_bits = {
        let v = N.next_power_of_two();
        v.checked_log2().unwrap()
    };
    let (capacity, total_bits) = (hash_cap << lvl_bits, hash_bits + lvl_bits);

    (capacity, total_bits, hash_bits)
}

/// A dumb Trie that also manages query advancing state
/// only for benching purposes!
pub struct ManagedTrie<'bump, T, const N: usize> {
    query: Vec<T>,
    trie: Trie<'bump, T, N>,
}

impl<'b, T: Clone + Hash + Ord + Eq + Default + std::fmt::Debug, const N: usize> Oneshot<'b, N>
    for ManagedTrie<'b, T, N>
{
    type Value = T;
    type IVal = &'b T;
    type KeyIter<const M: usize> = impl Iterator<Item = &'b T>;

    fn from_iter<I: IntoIterator<Item = [Self::Value; N]>>(iter: I, bump: &'b Bump) -> Self {
        let mut elems = iter.into_iter().collect::<Vec<_>>();
        elems.sort_unstable();
        Self {
            query: Vec::new(),
            trie: Trie::from_sorted::<ahash::AHasher, _>(elems.into_iter(), bump).unwrap(),
        }
    }

    fn advance(&'b self, v: &Self::Value) -> Option<&'b Self> {
        todo!()
        // self.query.push(v.clone());

        // Some(self)
    }

    fn intersect<'a, const M: usize>(&'b self, others: [&'b Self; M]) -> Self::KeyIter<M> {
        let from = self.trie.query_to_ix::<ahash::AHasher>(self.query.as_ref());
        let others: [(&'b Trie<_, N>, Ix); M] = unsafe {
            let mut arr: [_; M] = MaybeUninit::uninit().assume_init();
            for (item, mt) in (&mut arr[..]).into_iter().zip(others) {
                std::ptr::write(
                    item,
                    (
                        &mt.trie,
                        mt.trie.query_to_ix::<ahash::AHasher>(self.query.as_ref()),
                    ),
                );
            }
            arr
        };

        self.trie
            .intersect_unchecked::<ahash::AHasher, M>(from, others)
            .map(|x| x.0)
    }
}

pub struct Trie<'bump, T, const N: usize> {
    hash_keys: BumpVec<'bump, Key<T>>,
    extra_sibs: BumpVec<'bump, Key<T>>,
    pub root: Ix,
    data: BumpVec<'bump, [T; N]>,
}

impl<'bump, T, const N: usize> Trie<'bump, T, N>
where
    T: Clone + Hash + Default + PartialEq + Eq + Ord + std::fmt::Debug,
{
    pub fn from_sorted<H, I>(iter: I, bump: &'bump Bump) -> Option<Self>
    where
        I: IntoIterator<Item = [T; N]>,
        H: Hasher + Default,
    {
        let iter = iter.into_iter();
        let iter_len = if let Some(ub) = iter.size_hint().1 {
            if iter.size_hint().0 > 0 {
                ub
            } else {
                return None;
            }
        } else {
            return None;
        };

        let (capacity, _total_bits, hash_bits) = get_bit_sizes::<N>(iter_len);
        let mut hash_keys = BumpVec::with_capacity_in(capacity, bump);
        let mut extra_sibs: BumpVec<'bump, Key<T>> = BumpVec::with_capacity_in(capacity, bump);
        let mut data = BumpVec::with_capacity_in(capacity, bump);
        let mut root = Ix::none();

        let mut cur_sibs: [(T, Ix); N] = unsafe {
            let mut arr: [_; N] = MaybeUninit::uninit().assume_init();
            for item in &mut arr[..] {
                std::ptr::write(item, Default::default());
            }
            arr
        };

        // Zero-initialize keys
        for _i in 0..capacity {
            hash_keys.push(Key::default(), bump);
        }

        for v in iter {
            // For each tuple of values we need to create corresponding entries in the
            // keys list!
            let mut cur_ix = Ix::none();
            let mut sib_set = false;

            for (level, t) in v.iter().enumerate() {
                let ix = Self::calc_hash_keys_ix::<H>(cur_ix, t, level, hash_bits);
                if root.is_none() {
                    root = Ix::hashed(ix);
                }

                // Step 1
                // Find what key block this should correspond to
                let mut new_key: Result<usize, (Result<usize, Sibling>, usize)> = Ok(ix);
                let b = &hash_keys[ix];
                if !b.child.is_none() && (b.parent_ix != cur_ix || &b.data != t) {
                    let fst = if b.hash_sib.is_none() {
                        Ok(ix)
                    } else {
                        let mut z = b.hash_sib;
                        loop {
                            let zp = extra_sibs[z.0].hash_sib;
                            if zp.is_none() {
                                break;
                            } else {
                                z = zp;
                            }
                        }
                        Err(z)
                    };

                    let new_ix = extra_sibs.len();
                    new_key = Err((fst, new_ix));
                }

                // Step 2
                // We have our key block.
                // If we didn't need a sib, set the params appropriately
                // If we did need a sib, set the previous sibs attributes appropriately
                match new_key {
                    Ok(hash_ix) => {
                        let new_ix = Ix::hashed(hash_ix);

                        let this = &mut hash_keys[hash_ix];
                        this.data = t.clone();
                        this.parent_ix = cur_ix;

                        if cur_ix.is_hashed() {
                            hash_keys[cur_ix.0].child = Child::hashed(hash_ix);
                        } else if !cur_ix.is_none() {
                            extra_sibs[cur_ix.0 % (1 << (usize::BITS - 1))].child =
                                Child::hashed(hash_ix);
                        }

                        cur_ix = new_ix;
                    }
                    Err((sib_at, new_ix)) => {
                        extra_sibs.push(
                            Key {
                                data: t.clone(),
                                parent_ix: cur_ix,
                                ..Key::default()
                            },
                            bump,
                        );
                        let prev = match sib_at {
                            Ok(hash_ix) => &mut hash_keys[hash_ix],
                            Err(sib_ix) => &mut extra_sibs[sib_ix.0],
                        };
                        prev.hash_sib = Sibling::sibbed(new_ix);

                        if cur_ix.is_hashed() {
                            hash_keys[cur_ix.0].child = Child::sibbed(new_ix);
                        } else if !cur_ix.is_none() {
                            extra_sibs[cur_ix.0 % (1 << (usize::BITS - 1))].child =
                                Child::sibbed(new_ix);
                        }

                        cur_ix = Ix::sibbed(new_ix);
                    }
                }

                // Step 3
                // Set sibs appropriately
                let (prev_val, prev_ix) = &cur_sibs[level];

                if !prev_ix.is_none() && prev_val != t && !sib_set {
                    if prev_ix.is_hashed() {
                        hash_keys[prev_ix.0].tuple_sib = cur_ix;
                    } else {
                        extra_sibs[prev_ix.0 % (1 << (usize::BITS - 1))].tuple_sib = cur_ix;
                    }

                    sib_set = true;
                }

                cur_sibs[level] = (t.clone(), cur_ix);
            }

            data.push(v, bump);
            match cur_ix.as_enum().unwrap() {
                Ok(ix) => {
                    hash_keys[ix].child = Child::data(data.len() - 1);
                }
                Err(ix) => {
                    extra_sibs[ix].child = Child::data(data.len() - 1);
                }
            }
        }

        Some(Self {
            root,
            hash_keys,
            extra_sibs,
            data,
        })
    }

    // Assumes Ix is valid
    fn get_data_ix_unchecked<'a>(&self, ix: Ix) -> usize {
        // Our backing array is in sorted order so all we need to do is actually find the
        // correct start to begin with.
        // Basically, we go to hash_ix in the hash_keys array.
        // From there, we descend recursively into the child until we hit a data pointer.
        // Once we do this, we check if the first few elems actually match the query.
        // If not, we go to the next sib of the hash_ix and do the same thing
        // If none of the sibs check out, we can't materialize (the query doesn't exist
        // and there's some sort of mismatch between hash_ix and query)
        let mut block = if ix.is_none() {
            panic!("can't use null ix");
        } else if ix.is_hashed() {
            &self.hash_keys[ix.0]
        } else {
            &self.extra_sibs[ix.0 % (1 << (usize::BITS - 1))]
        };

        // Descend into children
        loop {
            // With the block reference, we start descending into the children
            // Get the block child and figure out what ix it's pointing to
            if block.child.is_none() {
                panic!("all children should be initialized");
            }

            let child_top = block.child.top_bits();
            if child_top == 0b00 {
                // Child is a hashed ix
                block = &self.hash_keys[block.child.0 % (1 << (usize::BITS - 2))];
            } else if child_top == 0b01 {
                // Child is a sibbed ix
                block = &self.extra_sibs[block.child.0 % (1 << (usize::BITS - 2))];
            } else {
                // Child is a data ptr
                // Return the data we get
                let data_ix = block.child.0 % (1 << (usize::BITS - 1));
                break data_ix;
            }
        }
    }

    // Return iterator
    // Assumes Ix exists in arrays
    pub fn materialize_unchecked<'a, 't>(
        &'t self,
        query: &'a [T],
        ix: Ix,
    ) -> Materialize<'a, 'bump, 't, T, N> {
        if ix.is_none() {
            Materialize {
                query: &[],
                data: &self.data,
                idx: 0,
                end: self.data.len(),
                _marker: PhantomData,
            }
        } else {
            let idx = self.get_data_ix_unchecked(ix);

            Materialize {
                query,
                data: &self.data,
                idx,
                end: self.data.len(),
                _marker: PhantomData,
            }
        }
    }

    /// Intersects the keys of M + 1 tries given a query.
    /// Performs this by materializing all tries and going through the elements
    /// of all tries at once, finding points where the keys match up.
    /// Assumes Ix is valid
    pub fn intersect_unchecked<'a, 't: 'bump, H: Hasher + Default, const M: usize>(
        &'t self,
        from: Ix,
        mut others: [(&'t Trie<'bump, T, N>, Ix); M],
    ) -> impl Iterator<Item = (&'bump T, Ix)> + 'a
    where
        'bump: 'a,
        't: 'a,
    {
        let mut cur_ix = if from.is_none() {
            self.root
        } else if from.is_hashed() {
            self.hash_keys[from.0]
                .child
                .as_ix()
                .expect("can't use data ix")
        } else {
            self.extra_sibs[from.0 % (1 << (usize::BITS - 1))]
                .child
                .as_ix()
                .expect("can't use data ix")
        };

        for (other_trie, other_ix) in others.iter_mut() {
            *other_ix = if other_ix.is_none() {
                other_trie.root
            } else if other_ix.is_hashed() {
                other_trie.hash_keys[other_ix.0]
                    .child
                    .as_ix()
                    .expect("can't use data ix")
            } else {
                other_trie.extra_sibs[other_ix.0 % (1 << (usize::BITS - 1))]
                    .child
                    .as_ix()
                    .expect("can't use data ix")
            };
        }

        let mut cur_max = None;

        iter::from_fn(move || {
            // Let's look at the current element
            'outer: loop {
                let xk = if cur_ix.is_hashed() {
                    &self.hash_keys[cur_ix.0]
                } else if !cur_ix.is_none() {
                    &self.extra_sibs[cur_ix.0 % (1 << (usize::BITS - 1))]
                } else {
                    return None;
                };

                if let Some(prev_max) = cur_max {
                    if prev_max > &xk.data {
                        cur_ix = xk.tuple_sib;
                        continue 'outer;
                    }
                }

                cur_max = Some(&xk.data);

                // For each of the other tries
                for (other_trie, other_ix) in others.iter_mut() {
                    'inner: loop {
                        let yk = if other_ix.is_hashed() {
                            &other_trie.hash_keys[other_ix.0]
                        } else if !other_ix.is_none() {
                            &other_trie.extra_sibs[other_ix.0 % (1 << (usize::BITS - 1))]
                        } else {
                            return None;
                        };

                        if let Some(prev_max) = cur_max {
                            if prev_max > &yk.data {
                                *other_ix = yk.tuple_sib;
                                continue 'inner;
                            }
                        }

                        if &yk.data > &xk.data {
                            cur_max = Some(&yk.data);
                            // Advancing xk taken care of above
                            continue 'outer;
                        } else if &yk.data == &xk.data {
                            *other_ix = yk.tuple_sib;
                            break 'inner;
                        } else {
                            *other_ix = yk.tuple_sib;
                            continue 'inner;
                        }
                    }
                }

                // We've passed the gauntlet
                let res = Some((&xk.data, cur_ix));
                // Advance xk
                cur_ix = xk.tuple_sib;
                return res;
            }
        })
        .fuse()
    }

    fn calc_hash_keys_ix<H: Hasher + Default>(
        prev: Ix,
        value: &T,
        level: usize,
        hash_bits: u32,
    ) -> usize {
        let mut hasher = H::default();
        prev.hash(&mut hasher);
        value.hash(&mut hasher);

        // index:
        //    hash % (2 ^ hash_bits)
        //    -----
        // xxxyyyyy
        // ---
        // level
        let hash_mask = (1 << hash_bits) - 1;
        let hv = (hasher.finish() & hash_mask) as usize;
        let lv = level << hash_bits;

        let ix = hv | lv;

        // println!();
        // println!("calc_hash_keys_ix");
        // println!("hash_bits: {}", hash_bits);
        // println!("hf {:#066b}", hasher.finish());
        // println!("hm {:#066b}", hash_mask);
        // println!("hv {:#066b}", hv);
        // println!("lv {:#066b}", lv);
        // println!("ix {:#066b}", ix);

        ix
    }

    pub fn query_to_ix<H: Hasher + Default>(&self, query: &[T]) -> Ix {
        let (_capacity, _total_bits, hash_bits) = get_bit_sizes::<N>(self.data.len());
        let mut cur = Ix::none();

        for (l, q) in query.iter().enumerate() {
            let hk = Self::calc_hash_keys_ix::<H>(cur, q, l, hash_bits);
            cur = Ix::hashed(hk);

            // Look at block.
            // If parent value matches and key value matches, we're good
            // Otherwise, go to hash sib
            loop {
                let cur_block = if cur.is_none() {
                    return Ix::none();
                } else if cur.is_hashed() {
                    &self.hash_keys[cur.0]
                } else {
                    &self.extra_sibs[cur.0 % (1 << (usize::BITS - 1))]
                };

                if cur_block.parent_ix == cur && &cur_block.data == q {
                    break;
                } else {
                    cur = cur_block.hash_sib.as_ix();
                }
            }
        }

        cur
    }
}

pub struct Materialize<'a, 'b, 't, T, const N: usize> {
    query: &'a [T],
    data: &'t BumpVec<'b, [T; N]>,
    idx: usize,
    end: usize,
    _marker: PhantomData<&'b [T; N]>,
}

impl<'a, 'b, 't, T: PartialEq, const N: usize> Iterator for Materialize<'a, 'b, 't, T, N>
where
    't: 'b,
{
    type Item = &'b [T; N];

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        // We have two break conditions:
        // - We're at the end of the of the array (ptr == end)
        // - The query no longer matches the data
        if self.idx == self.end {
            None
        } else {
            unsafe {
                let idx = self.idx;
                self.idx += 1;
                Some(self.data.get_unchecked(idx))
            }
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let upper = if self.query.len() == N {
            Some(1)
        } else {
            Some(self.end - self.idx)
        };

        (0, upper)
    }
}

impl<'a, 'b, 't, T: PartialEq, const N: usize> FusedIterator for Materialize<'a, 'b, 't, T, N> where
    't: 'b
{
}

#[derive(Debug, Default, PartialEq, Eq)]
struct Key<T> {
    parent_ix: Ix,
    tuple_sib: Ix,
    data: T,
    // index into extra_sibs
    // all 1s is None
    hash_sib: Sibling,
    // 00xx...xxx => hashed ix
    // 01yy...yyy => sibbed ix
    // 1zzz...zzz => data ix
    // 1111...111 => no child/uninit
    child: Child,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct Sibling(pub usize);

impl Sibling {
    pub fn sibbed(ix: usize) -> Self {
        Self(ix)
    }

    pub fn none() -> Self {
        Self(usize::MAX)
    }

    pub fn is_none(&self) -> bool {
        self.0 == usize::MAX
    }

    pub fn as_ix(&self) -> Ix {
        Ix(self.0 | 1 << (usize::BITS - 1))
    }
}

impl Default for Sibling {
    fn default() -> Self {
        Self::none()
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct Child(pub usize);

impl Child {
    pub fn hashed(ix: usize) -> Self {
        let bits = usize::BITS;
        Child(ix % (1 << (bits - 2)))
    }

    pub fn sibbed(ix: usize) -> Self {
        let bits = usize::BITS;
        Child((ix % (1 << (bits - 2))) | (0b01 << (bits - 2)))
    }

    pub fn data(ix: usize) -> Self {
        let bits = usize::BITS;
        Child((ix % (1 << (bits - 1))) | (1 << (bits - 1)))
    }

    pub fn none() -> Self {
        Child(usize::MAX)
    }

    pub fn is_none(&self) -> bool {
        self.0 == usize::MAX
    }

    pub fn top_bits(&self) -> usize {
        self.0 >> (usize::BITS - 2)
    }

    pub fn is_data(&self) -> bool {
        self.top_bits() != 0b00 && self.top_bits() != 0b01
    }

    pub fn is_hashed(&self) -> bool {
        self.top_bits() == 0b00
    }

    pub fn as_ix(&self) -> Option<Ix> {
        if self.is_none() {
            Some(Ix::none())
        } else if self.is_data() {
            None
        } else if self.is_hashed() {
            Some(Ix(self.0))
        } else {
            let mut val = self.0;
            val = val ^ (1 << (usize::BITS - 2));
            val = val ^ (1 << (usize::BITS - 1));
            Some(Ix(val))
        }
    }
}

impl Default for Child {
    fn default() -> Self {
        Self::none()
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Ix(pub usize);

impl Ix {
    pub fn hashed(ix: usize) -> Self {
        let bits = usize::BITS;
        Self(ix % (1 << (bits - 1)))
    }

    pub fn sibbed(ix: usize) -> Self {
        let bits = usize::BITS;
        Self((ix % (1 << (bits - 1))) | (0b1 << (bits - 1)))
    }

    pub fn none() -> Self {
        Self(usize::MAX)
    }

    pub fn is_none(&self) -> bool {
        self.0 == usize::MAX
    }

    pub fn is_hashed(&self) -> bool {
        self.top_bit() == 0
    }

    pub fn top_bit(&self) -> usize {
        self.0 >> (usize::BITS - 1)
    }

    pub fn as_enum(&self) -> Option<Result<usize, usize>> {
        if self.is_none() {
            None
        } else {
            if self.is_hashed() {
                Some(Ok(self.0))
            } else {
                Some(Err(self.0 % (1 << (usize::BITS - 1))))
            }
        }
    }
}

impl Default for Ix {
    fn default() -> Self {
        Self::none()
    }
}

#[cfg(test)]
mod test {
    use bumpalo::Bump;
    use itertools::iproduct;

    use super::{Ix, Trie};

    #[test]
    fn iter_keys() {
        let a = Bump::new();
        let t = Trie::from_sorted::<ahash::AHasher, _>((0..10).map(|x| [x]), &a).unwrap();

        let v: Vec<_> = t
            .intersect_unchecked::<ahash::AHasher, 0>(Ix::none(), [])
            .map(|x| *x.0)
            .collect();
        assert_eq!(v, (0..10).collect::<Vec<_>>());
    }

    #[test]
    fn iter_nested() {
        let sz = &10;
        let iter = iproduct!(0..*sz, 0..*sz, 0..*sz, 0..*sz, 0..*sz)
            .map(|(x, y, z, a, b)| [x, y, z, a, b]);
        let a = Bump::new();
        let t = Trie::from_sorted::<ahash::AHasher, _>(iter, &a).unwrap();

        println!("{:?}", t.hash_keys[t.root.0]);

        let v: Vec<_> = t
            .intersect_unchecked::<ahash::AHasher, 0>(Ix::none(), [])
            .map(|x| *x.0)
            .collect();
        assert_eq!(v, (0..10).collect::<Vec<_>>());
    }
}
