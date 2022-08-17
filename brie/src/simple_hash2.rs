// TODO: data-oriented opts
//       reduce size of everything wayyyyy down (entries should just be u64 and ptr)
//       pack data into array (partition according to paper)
//       building optimizations from paper (singleton, lazy)

use bumpalo::{boxed::Box, Bump};
use hyperloglogplus::{HyperLogLog, HyperLogLogPF};

use crate::{sorted::vec::BumpVec, Oneshot};

use std::{
    hash::{Hash, Hasher},
    mem::MaybeUninit, hint::unreachable_unchecked,
};

// TODO: restructure?
//       Make Trie always a BumpVec.
//       Entry has ptr field which becomes new enum type, Ptr
//       Ptr can be Data or Trie.
//       Removes some unnecessary matching I think?
#[derive(Debug)]
pub enum Trie<'b, E, const N: usize> {
    Empty,
    Data(Box<'b, Data<'b, [E; N]>>),
    // Data(E),
    Map(BumpVec<'b, Entry<'b, E, N>>),
}

impl<'b, E, const N: usize> Default for Trie<'b, E, N> {
    fn default() -> Self {
        Trie::Empty
    }
}

#[derive(Debug)]
pub struct Entry<'b, E, const N: usize> {
    hash: u64,
    // TODO: Could replace this with Option<Trie<'b, E, N>> and
    //       remove Trie::Empty
    //       Depends on if this makes the enum smaller
    ptr: Trie<'b, E, N>,
}

impl<'b, E, const N: usize> Default for Entry<'b, E, N> {
    fn default() -> Self {
        Entry {
            hash: 0,
            ptr: Trie::Empty,
        }
    }
}

#[derive(Debug)]
pub struct Data<'b, T> {
    data: T,
    next: Option<Box<'b, Self>>,
}

impl<'b, E, const N: usize> Trie<'b, E, N>
where
    E: Hash,
{
    fn init_and_get<H: Hasher + Default>(
        &mut self,
        key: &E,
        bits: u32,
        bump: &'b Bump,
    ) -> Option<&mut Self> {
        match self {
            Trie::Empty => {
                // Turn this into a map
                *self = Self::map(bits, bump);

                let (hash, bucket_ix) = Self::get_bucket::<H>(key, bits);
                let bv = match self {
                    Trie::Map(bv) => bv,
                    _ => unsafe { unreachable_unchecked() },
                };
                let res = &mut bv[bucket_ix];
                res.hash = hash;

                Some(&mut res.ptr)
            }
            Trie::Map(bv) => {
                // Linear probe
                let (hash, mut bucket_ix) = Self::get_bucket::<H>(key, bits);
                let max_ix = bv.len() - 1;

                loop {
                    let cur = &mut bv[bucket_ix];

                    if bucket_ix == max_ix {
                        // This ix is ok if we have no other options lmao
                        break;
                    } else if let Trie::Empty = cur.ptr {
                        // This ix is ok if it's unoccupied
                        break;
                    } else if cur.hash == hash {
                        // This ix is ok if it's the same hash
                        break;
                    }

                    bucket_ix += 1;
                }

                bv[bucket_ix].hash = hash;
                Some(&mut bv[bucket_ix].ptr)
            }
            // TODO: with singleton opt, this would mean something i think
            //       let's say we have this map:
            //       h(1) -> (1, 2)
            //       then we try to insert (1, 3)
            //       this means we need to do the following:
            //       1. set self to a map
            //       2. reinsert the data ptr that was here to the correct position on the new map
            //          corresponding to the hash val of the next level (we can get the val from the data
            //          ptr and then hash it)
            //       3. hash whatever we're tryna insert
            //       idk how this is perf wise
            Trie::Data(_) => None,
        }
    }

    fn print_util(&self) {
        match self {
            Trie::Map(vs) => {
                let occ = vs.iter().filter(|x| if let Trie::Empty = x.ptr { false } else { true }).count();
                println!("{} / {}", occ, vs.len());
            },
            _ => {},
            
        }
    }

    fn map(bits: u32, bump: &'b Bump) -> Self {
        let buckets = 2_usize.pow(bits);
        let mut bv = BumpVec::with_capacity_in(buckets, bump);
        for _i in 0..buckets {
            bv.push(Entry::default(), bump);
        }

        Self::Map(bv)
    }

    fn calc_bits(iter_len: usize) -> u32 {
        let sz = iter_len as f64;
        (sz * 1.25).log2().ceil() as u32
    }

    fn get_bucket<H: Hasher + Default>(elem: &E, bits: u32) -> (u64, usize) {
        let mut h = H::default();
        elem.hash(&mut h);
        let v = h.finish();
        (
            v,
            (v >> (usize::BITS - bits as u32) as u64)
                .try_into()
                .unwrap(),
        )
    }
}

impl<'b, E, const N: usize> Oneshot<'b, N> for Trie<'b, E, N>
where
    E: Clone + Hash,
{
    type Value = E;
    type IVal = usize;
    type KeyIter<const M: usize> = impl Iterator<Item = usize> where Self: 'b;

    fn from_iter<I: IntoIterator<Item = [Self::Value; N]>>(iter: I, bump: &'b Bump) -> Self {
        // Collect into a vec first (lmao)
        let iter = iter.into_iter().collect::<Vec<_>>();

        // HLL cardinality estimation
        let mut hlls: [HyperLogLogPF<E, ahash::RandomState>; N] = unsafe {
            let mut arr: [_; N] = MaybeUninit::uninit().assume_init();
            for item in &mut arr[..] {
                std::ptr::write(
                    item,
                    HyperLogLogPF::new(4, ahash::RandomState::new()).unwrap(),
                );
            }
            arr
        };

        for tup in iter.iter() {
            for (v, c) in tup.iter().zip(hlls.iter_mut()) {
                c.insert(v);
            }
        }

        let cardinalities: [usize; N] = unsafe {
            let mut arr: [_; N] = MaybeUninit::uninit().assume_init();
            for (item, hll) in (&mut arr[..]).into_iter().zip(hlls.iter_mut()) {
                std::ptr::write(item, hll.count().trunc() as usize);
            }
            arr
        };

        let mut res = Trie::Empty;

        for tup in iter {
            let mut cur = &mut res;

            let mut it = tup.iter().zip(cardinalities.iter());
            while let Some((v, buckets)) = it.next() {
                cur = cur
                    .init_and_get::<ahash::AHasher>(v, Self::calc_bits(*buckets), bump)
                    .unwrap();
            }

            // At this point cur is pointing to an entry
            // that should become a data ptr.
            let d = Box::new_in(
                Data {
                    data: tup,
                    next: None,
                },
                bump,
            );
            match cur {
                Trie::Empty => {
                    *cur = Trie::Data(d);
                }
                Trie::Data(pd) => {
                    let mut t = &mut **pd;
                    while t.next.as_mut().is_some() {
                        // We can't use while let here bc of borrowck limits
                        // with linked lists
                        // fuckin rust smfh
                        t = t.next.as_mut().unwrap();
                    }
                    t.next = Some(d);
                }
                Trie::Map(_) => unsafe { unreachable_unchecked() },
            }
        }

        res
    }

    fn advance(&'b self, v: &Self::Value) -> Option<&'b Self> {
        todo!()
    }

    fn intersect<'a, const M: usize>(&'b self, others: [&'b Self; M]) -> Self::KeyIter<M> {
        let mut vals = match self {
            Trie::Empty => todo!(),
            Trie::Data(_) => todo!(),
            Trie::Map(vs) => vs.iter().enumerate(),
        };

        std::iter::from_fn(move || {
            'outer: loop {
                if let Some((ix, v)) = vals.next() {
                    if let Trie::Empty = v.ptr {
                        continue 'outer;
                    }

                    for other in others.iter() {
                        let other = match other {
                            Trie::Empty => todo!(),
                            Trie::Data(_) => todo!(),
                            Trie::Map(os) => os,
                        };
                        let other_bits = other.len().log2();
                        let mut other_ix = (v.hash >> (usize::BITS - other_bits)) as usize;

                        // Start linear probe search
                        let max_ix = other.len() - 1;

                        loop {
                            let cur = &other[other_ix];

                            if other_ix == max_ix {
                                // Couldn't find before end
                                continue 'outer;
                            } else if let Trie::Empty = cur.ptr {
                                // Couldn't find before empty
                                continue 'outer;
                            } else if cur.hash == v.hash {
                                // This ix is ok if it's the same hash
                                break;
                            }

                            other_ix += 1;
                        }
                    }

                    return Some(ix)
                } else {
                    return None;
                }
            }
        })
    }
}
