- https://doc.rust-lang.org/nomicon/vec/vec.html  
- https://github.com/fitzgen/bumpalo/blob/main/src/collections/raw_vec.rs

todos:
- [x] implement a simple hash trie (`HashMap<T, Self>`)
- [x] benchmark harness (criterion, iai, etc.)
- [ ] add a vanilla::SortedTrie?
- [ ] api for sorted: arena as first param instead of last?

https://bheisler.github.io/criterion.rs/book/user_guide/comparing_functions.html

benches:
- building a trie
  - flat and a lot: many siblings, few levels
  - deeply nested: not many siblings, many levels
  - both - a lot and deeply nested
- querying a trie