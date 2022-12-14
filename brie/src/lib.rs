#![feature(allocator_api)]
#![feature(core_intrinsics)]
#![feature(alloc_layout_extra)]
#![feature(slice_ptr_get)]
#![feature(generic_associated_types)]
#![feature(int_log)]
#![feature(type_alias_impl_trait)]

use bumpalo::Bump;

pub mod hash;
pub mod simple_hash;
pub mod skip_list;
pub mod sorted;
pub mod vanilla;

pub trait Oneshot<'bump, const N: usize>
where
    Self: Sized,
{
    type Value;
    type IVal;
    type KeyIter<const M: usize>: Iterator<Item = Self::IVal> + 'bump
    where
        Self: 'bump;

    fn from_iter<I: IntoIterator<Item = [Self::Value; N]>>(iter: I, bump: &'bump Bump) -> Self;
    fn advance(&'bump self, v: &Self::Value) -> Option<&'bump Self>;
    fn intersect<'a, const M: usize>(
        &'bump self,
        others: [&'bump Self; M],
    ) -> Self::KeyIter<M>;
    // fn materialize(&self, query: [T; M]) -> impl Iterator<Item = [T; M + 1]>;
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
