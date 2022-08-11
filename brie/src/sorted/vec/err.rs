use super::alloc::{AllocError, LayoutError};

/// Augments `AllocErr` with a `CapacityOverflow` variant.
#[derive(Clone, PartialEq, Eq, Debug)]
// #[unstable(feature = "try_reserve", reason = "new API", issue="48043")]
pub enum CollectionAllocError {
    /// Error due to the computed capacity exceeding the collection's maximum
    /// (usually `isize::MAX` bytes).
    CapacityOverflow,
    /// Error due to the allocator (see the documentation for the [`AllocError`] type).
    AllocError,
}

// #[unstable(feature = "try_reserve", reason = "new API", issue="48043")]
impl From<AllocError> for CollectionAllocError {
    #[inline]
    fn from(AllocError: AllocError) -> Self {
        CollectionAllocError::AllocError
    }
}

// #[unstable(feature = "try_reserve", reason = "new API", issue="48043")]
impl From<LayoutError> for CollectionAllocError {
    #[inline]
    fn from(_: LayoutError) -> Self {
        CollectionAllocError::CapacityOverflow
    }
}

// /// An intermediate trait for specialization of `Extend`.
// #[doc(hidden)]
// trait SpecExtend<I: IntoIterator> {
//     /// Extends `self` with the contents of the given iterator.
//     fn spec_extend(&mut self, iter: I);
// }
