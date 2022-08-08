# brie

(b)inary heap t(rie)s!

## what

in database applications, often times we need to build *tries* as indices
in order to perform efficient joins on a conjunction of queries.
these tries are usually implemented as recursive hashmaps:

```rust
struct Trie<T>(HashMap<T, Self>);
```

the maps used in tries thus have a few characteristics:

- they are often pretty small
- there are *a lot* of them
- they all share the same lifetime - that of the index

this crate provides implementations of maps and tries which
optimize for this use case. in particular we implement a
few optimizations:

- **keep the data structure as small as possible**:
  rust data structures normally store a reference to the
  allocator used in the data structure.
  instead, we require every method of a data structure which can
  allocate to take an allocator argument.
  if you have a ton of data structure instances, this space
  savings can add up.
  this is similar to the "Unmanaged" idiom that zig uses.

- **bump allocation only**: the only supported allocator is the
  `bumpalo::Bump` allocator, which makes things much faster

- **deleting isn't supported**: when constructing indices for a
  database, we have no need to ever delete entries from a map.
  as such, for e.g. hashmap impls, we don't need to worry about
  deletion and the complexity this entails (tombstones, etc.)

- **different trie types**: sorted trie. binary heap trie. hash trie.
  we got it all!