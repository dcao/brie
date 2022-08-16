//! A flat list of tuples, backed by a RawVec
//! Adapted from https://github.com/frankmcsherry/blog/blob/master/posts/2018-05-19.md

use std::marker::PhantomData;

use bumpalo::Bump;

use crate::Oneshot;

use super::vec::BumpVec;

pub struct Read;
pub struct Write;

pub struct Trie<'bump, V: 'bump + Ord, const N: usize, M> {
    vec: BumpVec<'bump, [V; N]>,
    _rw: PhantomData<M>,
}

impl<'bump, V, const N: usize> Trie<'bump, V, N, Write>
where
    V: Ord + 'bump,
{
    pub fn finalize(mut self) -> Trie<'bump, V, N, Read> {
        self.vec.sort_unstable();
        self.vec.dedup();

        Trie {
            vec: self.vec,
            _rw: PhantomData,
        }
    }
}

// impl<'bump, V, const N: usize> Oneshot<'bump, N> for Trie<'bump, V, N, Write>
// where
//     V: Ord + 'bump,
// {
//     type Value = V;
//     type KeyIter<const M: usize> = impl Iterator<Item = &'bump Self::Value>
//     where Self: 'bump;

//     fn from_iter<I: IntoIterator<Item = [Self::Value; N]>>(iter: I, bump: &'bump Bump) -> Self {
//         todo!()
//     }

//     fn advance(self, v: &Self::Value) -> Option<Self> {
//         todo!()
//     }

//     fn intersect<'a, 't: 'bump, const M: usize>(
//         &'t self,
//         query: &'a [Self::Value],
//         others: [&'t Self; M],
//     ) -> Self::KeyIter<M> {
//         todo!()
//     }
// }

// impl<'bump, V, const N: usize> Trieish<'bump> for Trie<'bump, V, N, Write>
// where
//     V: Ord + 'bump,
// {
//     type Value = V;
//     type Tuple<'a> = [V; N] where V: 'a;

//     fn empty(_bump: &'bump Bump) -> Self {
//         Trie {
//             vec: BumpVec::new(),
//             _rw: PhantomData,
//         }
//     }

//     fn insert<'a>(&mut self, tuple: Self::Tuple<'a>, bump: &'bump Bump)
//     where
//         V: 'a,
//     {
//         self.vec.push(tuple, bump);
//     }

//     fn query(&self, _v: &Self::Value) -> bool {
//         panic!("can't query while in write mode!")
//     }
// }

// impl<'bump, V, const N: usize> Trieish<'bump> for Trie<'bump, V, N, Read>
// where
//     V: Ord + 'bump,
// {
//     type Value = V;
//     type Tuple<'a> = [V; N] where V: 'a;

//     fn empty(_bump: &'bump Bump) -> Self {
//         Trie {
//             vec: BumpVec::new(),
//             _rw: PhantomData,
//         }
//     }

//     fn insert<'a>(&mut self, _tuple: Self::Tuple<'a>, _bump: &'bump Bump)
//     where
//         V: 'a,
//     {
//         panic!("can't insert while in read mode!")
//     }

//     fn query(&self, _v: &Self::Value) -> bool {
//         todo!()
//     }

//     fn from_iter<'a, I: IntoIterator<Item = Self::Tuple<'a>>>(iter: I, bump: &'bump Bump) -> Self
//     where
//         Self: Sized,
//         Self::Value: 'a,
//     {
//         let mut vec: BumpVec<[V; N]> = BumpVec::from_iter(iter.into_iter(), bump);
//         vec.sort_unstable();
//         vec.dedup();
//         Self {
//             vec,
//             _rw: PhantomData,
//         }
//     }
// }
