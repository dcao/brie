#![feature(concat_idents)]

use brie::{hash, simple_hash, sorted, vanilla, Oneshot, simple_hash2};
use bumpalo::Bump;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};

use itertools::iproduct;

macro_rules! build_flat {
    ($g:expr, $ty:ty, $sz:expr) => {
        $g.bench_with_input(BenchmarkId::new(stringify!($ty), $sz), &$sz, |b, sz| {
            let a = Bump::new();
            b.iter(|| {
                let iter = (0..*sz).map(|x| [x]);
                <$ty>::from_iter(iter, &a)
            });
        });
    };
}

macro_rules! build_mid {
    ($g:expr, $ty:ty, $sz:expr) => {
        $g.bench_with_input(BenchmarkId::new(stringify!($ty), $sz), &$sz, |b, sz| {
            let a = Bump::new();
            b.iter(|| {
                let iter = iproduct!(0..*sz, 0..*sz, 0..*sz).map(|(x, y, z)| [x, y, z]);
                <$ty>::from_iter(iter, &a)
            });
        });
    };
}

macro_rules! build_nested {
    ($g:expr, $ty:ty, $sz:expr) => {
        $g.bench_with_input(BenchmarkId::new(stringify!($ty), $sz), &$sz, |b, sz| {
            let a = Bump::new();
            b.iter(|| {
                let iter = iproduct!(0..*sz, 0..*sz, 0..*sz, 0..*sz, 0..*sz)
                    .map(|(x, y, z, a, b)| [x, y, z, a, b]);
                <$ty>::from_iter(iter, &a)
            });
        });
    };
}

macro_rules! intersect_flat {
    ($g:expr, $ty:ty, $sz:expr) => {
        $g.bench_with_input(BenchmarkId::new(stringify!($ty), $sz), &$sz, |b, sz| {
            let a = Bump::new();
            let iter = (0..*sz).map(|x| [x]);
            let t1 = <$ty>::from_iter(iter.clone(), &a);
            let t2 = {
                let i = iter.clone().filter(|vs| vs[0] % 2 == 0);
                <$ty>::from_iter(i, &a)
            };
            let t3 = {
                let i = iter.clone().filter(|vs| vs[0] % 3 == 0);
                <$ty>::from_iter(i, &a)
            };
            let t4 = {
                let i = iter.clone().filter(|vs| vs[0] % 5 == 0);
                <$ty>::from_iter(i, &a)
            };

            b.iter(|| {
                for key in <$ty as Oneshot<1>>::intersect::<3>(&t1, [&t2, &t3, &t4]) {
                    // no-op
                    let _k = key;
                }
            });
        });
    };
}

macro_rules! intersect_nested {
    ($g:expr, $ty:ty, $sz:expr) => {
        $g.bench_with_input(BenchmarkId::new(stringify!($ty), $sz), &$sz, |b, sz| {
            let a = Bump::new();
            let iter = iproduct!(0..*sz, 0..*sz, 0..*sz, 0..*sz, 0..*sz)
                .map(|(x, y, z, a, b)| [x, y, z, a, b]);
            let t1 = { <$ty>::from_iter(iter.clone(), &a) };
            let t2 = {
                let i = iter.clone().filter(|vs| vs[0] % 2 == 0);
                <$ty>::from_iter(i, &a)
            };
            let t3 = {
                let i = iter.clone().filter(|vs| vs[0] % 3 == 0);
                <$ty>::from_iter(i, &a)
            };
            let t4 = {
                let i = iter.clone().filter(|vs| vs[0] % 5 == 0);
                <$ty>::from_iter(i, &a)
            };

            b.iter(|| {
                for key in <$ty as Oneshot<5>>::intersect::<3>(&t1, [&t2, &t3, &t4]) {
                    // no-op
                    let _k = key;
                }
            });
        });
    };
}

fn bench_build_flat(c: &mut Criterion) {
    let mut group = c.benchmark_group("trie, build flat (1 layer)");
    group.sample_size(10);

    for upper in [10, 50, 100, 500, 1_000, 10_000] {
        build_flat!(group, vanilla::Trie<_>, upper);
        build_flat!(group, vanilla::BumpTrie<_>, upper);
        build_flat!(group, sorted::Trie<_>, upper);
        build_flat!(group, simple_hash2::Trie<_, 1>, upper);
        // build_flat!(group, hash::ManagedTrie<_, 1>, upper);
    }
}

fn bench_build_mid(c: &mut Criterion) {
    let mut group = c.benchmark_group("trie, build mid (3 layers)");
    group.sample_size(10);

    for upper in [1, 5, 10, 25, 50, 100] {
        build_mid!(group, vanilla::Trie<_>, upper);
        build_mid!(group, vanilla::BumpTrie<_>, upper);
        build_mid!(group, sorted::Trie<_>, upper);
        build_mid!(group, simple_hash2::Trie<_, 3>, upper);
        // build_mid!(group, hash::ManagedTrie<_, 3>, upper);
    }
}

fn bench_build_nested(c: &mut Criterion) {
    let mut group = c.benchmark_group("trie, build nested (5 layers)");
    group.sample_size(10);

    for upper in [1, 5, 10, 15] {
        build_nested!(group, vanilla::Trie<_>, upper);
        build_nested!(group, vanilla::BumpTrie<_>, upper);
        build_nested!(group, sorted::Trie<_>, upper);
        build_nested!(group, simple_hash::Trie<_, 5>, upper);
        // build_nested!(group, hash::ManagedTrie<_, 5>, upper);
    }
}

fn bench_intersect_flat(c: &mut Criterion) {
    let mut group = c.benchmark_group("trie, intersect flat (1 layers)");
    group.sample_size(10);

    for upper in [1_000, 10_000, 100_000, 1_000_000] {
        intersect_flat!(group, vanilla::Trie<_>, upper);
        intersect_flat!(group, vanilla::BumpTrie<_>, upper);
        intersect_flat!(group, sorted::Trie<_>, upper);
        // intersect_flat!(group, simple_hash2::Trie<_, 1>, upper);
        // intersect_nested!(group, hash::ManagedTrie<_, 5>, upper);
    }
}

criterion_group! {
    name = benches;
    config = Criterion::default();
    targets = bench_build_flat, bench_build_mid, bench_build_nested//, bench_intersect_flat
    // targets = bench_intersect_flat
}
criterion_main!(benches);
