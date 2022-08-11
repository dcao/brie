//! Experimental flattened hash trie impl, influenced by
//! the bytell hash map

use std::{
    hash::{Hash, Hasher},
    marker::PhantomData,
    ptr::NonNull,
};

use bumpalo::Bump;

use crate::sorted::vec::BumpVec;

// returns (cap, total_bits, hash_bits)
fn get_bit_sizes<const N: usize>(iter_len: usize) -> (usize, u32, u32) {
    let (capacity, total_bits) = {
        let size = iter_len as f64;
        let upsize = (size * 1.25).ceil() as usize;
        let v = upsize.next_power_of_two();
        (v, v.checked_log2().unwrap())
    };
    let hash_bits = {
        let v = N.next_power_of_two();
        total_bits - v.checked_log2().unwrap()
    };

    (capacity, total_bits, hash_bits)
}

// TODO: a StatefulTrie which contains query/hash_key state
//       so that it'll work with the Trieish trait

pub struct Trie<'bump, T, const N: usize> {
    hash_keys: BumpVec<'bump, Key>,
    extra_sibs: BumpVec<'bump, Key>,
    data: BumpVec<'bump, [T; N]>,
}

impl<'bump, T, const N: usize> Trie<'bump, T, N>
where
    T: Clone + Hash + Default + PartialEq + Eq,
{
    pub fn from_sorted<H, I>(iter: I, bump: &'bump Bump) -> Self
    where
        I: ExactSizeIterator<Item = [T; N]>,
        H: Hasher + Default,
    {
        let iter = iter.into_iter();
        let (capacity, _total_bits, hash_bits) = get_bit_sizes::<N>(iter.len());
        let mut hash_keys = BumpVec::with_capacity_in(capacity, bump);
        let mut extra_sibs: BumpVec<'bump, Key> = BumpVec::with_capacity_in(capacity, bump);
        let mut data = BumpVec::with_capacity_in(capacity, bump);

        // Zero-initialize keys
        for _i in 0..capacity {
            hash_keys.push(Key::default(), bump);
        }

        for v in iter {
            // For each tuple of values we need to create corresponding entries in the
            // keys list!
            let mut prev_hash = None;
            let mut cur_key: Option<Result<usize, usize>> = None;

            for (level, t) in v.iter().enumerate() {
                let ix = Self::calc_hash_keys_ix::<H>(prev_hash, t, level, hash_bits);

                // Go next?
                // either ix in hashed or ix in sibbed
                let mut new_key: Result<usize, usize> = Ok(ix);
                let mut z = unsafe { hash_keys.get_unchecked(ix) }.sibling;
                if !z.is_none() {
                    while !z.is_none() {
                        z = unsafe { extra_sibs.get_unchecked(z.0) }.sibling;
                    }

                    let new_ix = extra_sibs.len();
                    new_key = Err(new_ix);
                }

                if let Err(new_ix) = new_key {
                    extra_sibs.push(Key::default(), bump);
                    unsafe { extra_sibs.get_unchecked_mut(z.0) }.sibling = Sibling::sibbed(new_ix);
                }

                if let Some(prev_key) = cur_key {
                    let prev = match prev_key {
                        Ok(ix) => unsafe { hash_keys.get_unchecked_mut(ix) },
                        Err(ix) => unsafe { extra_sibs.get_unchecked_mut(ix) },
                    };

                    match new_key {
                        Ok(ix) => {
                            if prev.child.is_none() {
                                prev.child = Child::hashed(ix);
                            }
                        }
                        Err(ix) => {
                            if prev.child.is_none() {
                                prev.child = Child::sibbed(ix);
                            }
                        }
                    }
                }

                cur_key = Some(new_key);
                prev_hash = Some(ix);
            }

            data.push(v, bump);
            match cur_key.unwrap() {
                Ok(ix) => {
                    unsafe { hash_keys.get_unchecked_mut(ix) }.child = Child::data(data.len() - 1);
                }
                Err(ix) => {
                    unsafe { extra_sibs.get_unchecked_mut(ix) }.child = Child::data(data.len() - 1);
                }
            }
        }

        Self {
            hash_keys,
            extra_sibs,
            data,
        }
    }

    pub fn advance<H: Hasher + Default>(
        &self,
        prev: Option<usize>,
        value: &T,
        level: usize,
    ) -> usize {
        let (_, _, hash_bits) = get_bit_sizes::<N>(self.data.len());
        Self::calc_hash_keys_ix::<H>(prev, value, level, hash_bits)
    }

    // Return iterator
    pub fn materialize<'a>(
        &self,
        hash_ix: usize,
        query: &'a [T],
    ) -> Option<Materialize<'a, 'bump, T, N>> {
        // Given a query so far, we want to materialize an iterator which goes through all
        // items starting with that query.
        // Our backing array is in sorted order so all we need to do is actually find the
        // correct start to begin with.
        // Basically, we go to hash_ix in the hash_keys array.
        // From there, we descend recursively into the child until we hit a data pointer.
        // Once we do this, we check if the first few elems actually match the query.
        // If not, we go to the next sib of the hash_ix and do the same thing
        // If none of the sibs check out, we can't materialize (the query doesn't exist
        // and there's some sort of mismatch between hash_ix and query)
        let mut cur: Result<usize, usize> = Ok(hash_ix);

        // Outer loop: check cur block if it works
        loop {
            // Inner loop: descend into children
            let mut block = match cur {
                Ok(hash_ix) => unsafe { self.hash_keys.get_unchecked(hash_ix) },
                Err(sib_ix) => unsafe { self.extra_sibs.get_unchecked(sib_ix) },
            };

            let (data_ix, data) = loop {
                // With the block reference, we start descending into the children
                // Get the block child and figure out what ix it's pointing to
                if block.child.is_none() {
                    panic!("all children should be initialized");
                }

                let child_top = block.child.top_bits();
                if child_top == 0b00 {
                    // Child is a hashed ix
                    block = unsafe {
                        self.hash_keys
                            .get_unchecked(block.child.0 % (1 << (usize::BITS - 2)))
                    };
                } else if child_top == 0b01 {
                    // Child is a sibbed ix
                    block = unsafe {
                        self.extra_sibs
                            .get_unchecked(block.child.0 % (1 << (usize::BITS - 2)))
                    };
                } else {
                    // Child is a data ptr
                    // Return the data we get
                    let data_ix = block.child.0 % (1 << (usize::BITS - 1));
                    break (data_ix, unsafe { self.data.get_unchecked(data_ix) });
                }
            };

            // Compare query to data
            if query.iter().zip(data.iter()).all(|(x, y)| x == y) {
                // This shit is valid!
                // We get a NonNull ptr to this element in the data arr
                let ptr = NonNull::from(unsafe { self.data.get_unchecked(data_ix) });
                // Get a pointer to the very end of the data arr, so the iter knows
                // when to stop
                let end = unsafe { self.data.as_ptr().add(self.data.len()) };

                break Some(Materialize {
                    ptr,
                    end,
                    query,
                    _marker: PhantomData,
                });
            } else {
                // This isn't valid.
                // There are two cases: either the current block has a sibling
                // and we go to that, or it doesn't and we give up
                let this = match cur {
                    Ok(hash_ix) => unsafe { self.hash_keys.get_unchecked(hash_ix) },
                    Err(sib_ix) => unsafe { self.extra_sibs.get_unchecked(sib_ix) },
                };

                if this.sibling.is_none() {
                    // Give up D:
                    break None;
                } else {
                    // Try with sibling
                    cur = Err(this.sibling.0);
                }
            }
        }
    }

    fn calc_hash_keys_ix<H: Hasher + Default>(
        prev: Option<usize>,
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
        let hash_mask = 1 << hash_bits - 1;
        let hv = (hasher.finish() & hash_mask) as usize;
        let lv = level << hash_bits;

        let ix = hv | lv;

        ix
    }
}

pub struct Materialize<'a, 'b, T, const N: usize> {
    query: &'a [T],
    ptr: NonNull<[T; N]>,
    end: *const [T; N],
    _marker: PhantomData<&'b [T; N]>,
}

impl<'a, 'b, T: PartialEq, const N: usize> Iterator for Materialize<'a, 'b, T, N> {
    type Item = &'b [T; N];

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        // We have two break conditions:
        // - We're at the end of the of the array (ptr == end)
        // - The query no longer matches the data
        unsafe {
            let raw = self.ptr.as_ptr();
            if raw as *const _ == self.end {
                None
            } else {
                let val = self.ptr.as_ref();
                if val.iter().zip(self.query.iter()).any(|(x, y)| x != y) {
                    None
                } else {
                    self.ptr = NonNull::new_unchecked(raw.add(1));
                    Some(val)
                }
            }
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        // TODO: if slice len = N we know it's max 1 upper bound
        // TODO: upper bound end - ptr
        let upper = if self.query.len() == N {
            Some(1)
        } else {
            Some(unsafe { self.end.offset_from(self.ptr.as_ptr() as *const _) } as usize)
        };

        (0, upper)
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
struct Key {
    // index into extra_sibs
    // all 1s is None
    sibling: Sibling,
    // 00xx...xxx => hashed ix
    // 01yy...yyy => sibbed ix
    // 1zzz...zzz => data ix
    // 1111...111 => no child/uninit
    child: Child,
    // both usize changes reduce size of Key from 32 to 16.
    // storing offsets instead of ixs might reduce even more
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
}

impl Default for Child {
    fn default() -> Self {
        Self::none()
    }
}
