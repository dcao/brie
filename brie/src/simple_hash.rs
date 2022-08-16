// TODO: data-oriented opts (later!)

use bumpalo::{boxed::Box, Bump};

use crate::{sorted::vec::BumpVec, Oneshot};

use std::hash::{Hash, Hasher};

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
                let l = vs.len();
                Some(&mut vs[Self::get_bucket::<H>(key, l)])
            }
        }
    }

    fn init(&mut self, buckets: usize, bump: &'b Bump) {
        match self {
            Trie::Empty => {
                *self = Trie::map(buckets, bump);
            }
            Trie::Data(_) => {}
            Trie::Map(_) => {}
        }
    }

    fn map(buckets: usize, bump: &'b Bump) -> Self {
        let mut bv = BumpVec::with_capacity_in(buckets, bump);
        for _i in 0..buckets {
            bv.push(Self::Empty, bump);
        }

        Self::Map(bv)
    }

    fn calc_buckets(iter_len: usize) -> usize {
        let sz = iter_len as f64;
        ((sz * 0.25) as usize).next_power_of_two()
    }

    fn get_bucket<H: Hasher + Default>(elem: &E, buckets: usize) -> usize {
        let mut h = H::default();
        elem.hash(&mut h);
        (h.finish() % buckets as u64).try_into().unwrap()
    }
}

impl<'b, E, const N: usize> Oneshot<'b, N> for Trie<'b, E, N>
where
    E: Clone + Hash,
{
    type Value = E;
    type KeyIter<const M: usize> = impl Iterator<Item = &'b E> where Self: 'b;

    fn from_iter<I: IntoIterator<Item = [Self::Value; N]>>(iter: I, bump: &'b Bump) -> Self {
        let iter = iter.into_iter();
        let buckets = Self::calc_buckets(iter.size_hint().1.unwrap());

        let mut res = Trie::map(buckets, bump);

        for tup in iter {
            let mut cur = &mut res;

            let mut it = tup.iter();
            while let Some(v) = it.next() {
                cur.init(buckets, bump);
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
