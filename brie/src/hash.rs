//! Experimental flattened hash trie impl, influenced by
//! the bytell hash map

use std::hash::{Hash, Hasher};

use bumpalo::{boxed::Box, Bump};

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

struct Trie<'bump, T, const N: usize> {
    hash_keys: BumpVec<'bump, Key>,
    extra_sibs: BumpVec<'bump, Key>,
    data: BumpVec<'bump, [T; N]>,
}

impl<'bump, T, const N: usize> Trie<'bump, T, N>
where
    T: Clone + Hash + Default + PartialEq + Eq,
{
    pub fn from_sorted<I, H>(iter: I, bump: &'bump Bump) -> Self
    where
        I: ExactSizeIterator<Item = [T; N]>,
        H: Hasher + Default,
    {
        let iter = iter.into_iter();
        let (capacity, total_bits, hash_bits) = get_bit_sizes::<N>(iter.len());
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

                    cur_key = Some(new_key);
                }

                prev_hash = Some(ix);
            }

            data.push(v, bump);
            match cur_key.unwrap() {
                Ok(ix) => {
                    unsafe { hash_keys.get_unchecked_mut(ix) }.child =
                        Child::data(data.len() - 1);
                }
                Err(ix) => {
                    unsafe { extra_sibs.get_unchecked_mut(ix) }.child =
                        Child::data(data.len() - 1);
                }
            }
        }

        Self {
            hash_keys,
            extra_sibs,
            data,
        }
    }

    pub fn advance<H: Hasher + Default>(&self, prev: Option<usize>, value: &T, level: usize) -> usize {
        let (_, _, hash_bits) = get_bit_sizes::<N>(self.data.len());
        Self::calc_hash_keys_ix::<H>(prev, value, level, hash_bits)
    }

    // Return iterator
    pub fn materialize(&self, hash_ix: usize) {
        todo!()
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
}

impl Default for Child {
    fn default() -> Self {
        Self::none()
    }
}