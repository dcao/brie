// TODO: data-oriented opts (later!)

use bumpalo::{boxed::Box, Bump};
use hyperloglogplus::{HyperLogLogPF, HyperLogLog};

use crate::{sorted::vec::BumpVec, Oneshot};

use std::{hash::{Hash, Hasher}, mem::MaybeUninit};

// #[derive(Debug)]
// pub enum Trie<'b, E, const N: usize> {
//     Empty,
//     Data(Box<'b, Data<'b, [E; N]>>),
//     Map(BumpVec<'b, Entry<'b, E, N>>),
// }

// impl<'b, E, const N: usize> Default for Trie<'b, E, N> {
//     fn default() -> Self {
//         Trie::Empty
//     }
// }

// #[derive(Debug, Default)]
// pub struct Entry<'b, E, const N: usize> {
//     hash: usize,
//     ptr: Trie<'b, E, N>,
// }

pub enum Trie<'b, E, const N: usize> {
    Empty,
    Data(Box<'b, Data<'b, [E; N]>>),
    Map(BumpVec<'b, Self>),
}

pub struct Data<'b, T> {
    data: T,
    next: Option<Box<'b, Self>>,
}

impl<'b, E, const N: usize> Trie<'b, E, N>
where
    E: Hash,
{
    fn get<H: Hasher + Default>(&mut self, key: &E) -> Option<&mut Self> {
        match self {
            Trie::Empty => None,
            Trie::Data(_) => None,
            Trie::Map(vs) => {
                let l = vs.len().log2(); // TODO: perf
                Some(&mut vs[Self::get_bucket::<H>(key, l)])
            }
        }
    }

    fn init(&mut self, bits: u32, bump: &'b Bump) {
        match self {
            Trie::Empty => {
                *self = Trie::map(bits, bump);
            }
            Trie::Data(_) => {}
            Trie::Map(_) => {}
        }
    }

    fn map(bits: u32, bump: &'b Bump) -> Self {
        let buckets = 2_usize.pow(bits);
        let mut bv = BumpVec::with_capacity_in(buckets, bump);
        for _i in 0..buckets {
            bv.push(Self::Empty, bump);
        }

        Self::Map(bv)
    }

    fn calc_bits(iter_len: usize) -> u32 {
        let sz = iter_len as f64;
        (sz * 1.25).log2().ceil() as u32
    }

    fn get_bucket<H: Hasher + Default>(elem: &E, bits: u32) -> usize {
        let mut h = H::default();
        elem.hash(&mut h);
        (h.finish() >> (usize::BITS - bits as u32) as u64).try_into().unwrap()
    }
}

impl<'b, E, const N: usize> Oneshot<'b, N> for Trie<'b, E, N>
where
    E: Clone + Hash,
{
    type Value = E;
    type KeyIter<const M: usize> = impl Iterator<Item = &'b E> where Self: 'b;

    fn from_iter<I: IntoIterator<Item = [Self::Value; N]>>(iter: I, bump: &'b Bump) -> Self {
        // Collect into a vec first (lmao)
        let iter = iter.into_iter().collect::<Vec<_>>();

        // HLL++ cardinality estimation
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
                std::ptr::write(
                    item,
                    hll.count().trunc() as usize,
                );
            }
            arr
        };

        let mut res = Trie::Empty;

        for tup in iter {
            let mut cur = &mut res;

            let mut it = tup.iter().zip(cardinalities.iter());
            while let Some((v, buckets)) = it.next() {
                cur.init(Self::calc_bits(*buckets), bump);
                cur = cur.get::<ahash::AHasher>(v).unwrap();
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
                },
                Trie::Data(pd) => {
                    let mut t = &mut **pd;
                    while t.next.as_mut().is_some() {
                        // We can't use while let here bc of borrowck limits
                        // with linked lists
                        // fuckin rust smfh
                        t = t.next.as_mut().unwrap();
                    }
                    t.next = Some(d);
                },
                Trie::Map(_) => unreachable!(),
            }
        }

        res
    }

    fn advance(&'b self, v: &Self::Value) -> Option<&'b Self> {
        todo!()
    }

    fn intersect<'a, 't: 'b, const M: usize>(&'t self, others: [&'t Self; M]) -> Self::KeyIter<M> {
        std::iter::from_fn(|| todo!())
    }
}
