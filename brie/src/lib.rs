#![feature(allocator_api)]
#![feature(core_intrinsics)]
#![feature(alloc_layout_extra)]
#![feature(slice_ptr_get)]
#![feature(generic_associated_types)]
#![feature(generic_const_exprs)]
#![feature(int_log)]

use bumpalo::Bump;

pub mod sorted;
pub mod vanilla;
pub mod hash;
pub mod skip_list;

// TODO
// binary heap trie?
// hashbrown but without the alloc field
// sorted trie
// custom hash trie (optimized for size and no delete!)

pub trait Trieish<'bump> {
    type Value;
    type Tuple<'a>: AsRef<[Self::Value]> where Self::Value: 'a; // should be [V; N] or &[V]

    fn empty(bump: &'bump Bump) -> Self;
    fn insert<'a>(&mut self, tuple: Self::Tuple<'a>, bump: &'bump Bump) where Self::Value: 'a;
    // TODO: what if this changes the type of self?
    // fn advance(&mut self, v: &Self::Value) -> bool;
    fn query(&self, v: &Self::Value) -> bool;

    fn advance(&self, _v: &Self::Value) -> Option<&Self> {
        todo!()
    }

    fn from_iter<'a, I: IntoIterator<Item = Self::Tuple<'a>>>(iter: I, bump: &'bump Bump) -> Self
    where
        Self: Sized,
        Self::Value: 'a,
    {
        let mut res = Self::empty(bump);

        for x in iter.into_iter() {
            res.insert(x, bump);
        }

        res
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
