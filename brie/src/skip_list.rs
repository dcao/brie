use std::mem;

use bumpalo::Bump;

use crate::sorted::vec::BumpVec;

struct SkipList<'bump, T, const N: usize> {
    skips: [BumpVec<'bump, usize>; N],
    data: BumpVec<'bump, T>,
}

impl<'bump, T, const N: usize> SkipList<'bump, T, N>
where
    T: Clone + Ord + PartialEq,
{
    pub fn from_sorted<I>(mut iter: I, bump: &'bump Bump) -> Self
    where
        I: ExactSizeIterator<Item = [T; N]>,
    {
        assert!(iter.len() >= 1);

        let mut data = BumpVec::with_capacity_in(iter.len(), bump);
        let mut skips = unsafe {
            let mut arr: [BumpVec<'bump, usize>; N] = mem::MaybeUninit::uninit().assume_init();
            for item in &mut arr[..] {
                std::ptr::write(item, BumpVec::with_capacity_in(iter.len(), bump));
            }
            arr
        };

        let prev: [T; N] = iter.next().unwrap();
        let mut cur_skips: [usize; N] = [1; N];

        for tup in iter {
            for (i, v) in tup.iter().enumerate() {
                if &prev[i] == v {
                    cur_skips[i] += 1;
                } else {
                    // Don't increment.
                    // Push this val into cur skip level
                    skips[i].push(cur_skips[i], bump);
                    cur_skips[i] = 0;

                    // For all next levels, push 0
                    for l in i + 1..N {
                        skips[l].push(0, bump);
                        cur_skips[l] = 0;
                    }
                }
            }
        }

        for i in 0..N {
            skips[i].push(0, bump);
        }

        Self { skips, data }
    }
}
