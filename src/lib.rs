#![feature(allocator_api)]
#![feature(alloc_layout_extra)]
#![feature(slice_ptr_get)]

mod raw;
pub mod vanilla;

// TODO
// binary heap trie?
// hashbrown but without the alloc field
// sorted trie
// custom hash trie (optimized for size and no delete!)

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
