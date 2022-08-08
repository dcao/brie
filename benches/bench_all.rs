use brie::{vanilla, sorted};
use bumpalo::Bump;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};

// macro_rules! odometer {
//     ([$($e:ident),*], $f:expr) => {
//         odometer!(@recur [$($e),*], $f, [])
//     };
//     (@recur [$e1:ident, $($e:ident),+], $f:expr, [$($r:ident),*]) => {
//         for $e1 in 0..*$e1 {
//             odometer!(@recur [$($e),*], $f, [$($r,)* $e1])
//         }
//     };
//     (@recur [$e1:ident], $f:expr, [$($r:ident),*]) => {
//         for $e1 in 0..*$e1 {
//             odometer!(@recur [], $f, [$($r,)* $e1])
//         }
//     };
//     (@recur [], $f:expr, [$($r:ident),*]) => {
//         $f(&[$($r),*])
//     };
// }
fn build_flat(c: &mut Criterion) {
    let mut group = c.benchmark_group("trie, build flat (1 layer)");

    for upper in [1, 10, 100, 1000, 10000] {
        group.bench_with_input(BenchmarkId::new("vanilla::Trie", upper), &upper, |b, sz| {
            b.iter(|| {
                let mut t = vanilla::Trie::new();
                for i in 0..*sz {
                    t.insert_tuple(&[i]);
                }
            })
        });

        group.bench_with_input(
            BenchmarkId::new("vanilla::BumpTrie", upper),
            &upper,
            |b, sz| {
                b.iter(|| {
                    let a = Bump::new();
                    let mut t = vanilla::BumpTrie::new_in(&a);
                    for i in 0..*sz {
                        t.insert_tuple(&a, &[i]);
                    }
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("sorted::Trie", upper),
            &upper,
            |b, sz| {
                b.iter(|| {
                    let a = Bump::new();
                    let mut t = sorted::Trie::new();
                    for i in 0..*sz {
                        t.insert_tuple(&a, &[i]);
                    }
                })
            },
        );
    }
}

fn build_mid(c: &mut Criterion) {
    let mut group = c.benchmark_group("trie, build mid (3 layers)");

    for upper in [1, 5, 10, 25, 50, 100] {
        group.bench_with_input(BenchmarkId::new("vanilla::Trie", upper), &upper, |b, sz| {
            b.iter(|| {
                let mut t = vanilla::Trie::new();
                for i in 0..*sz {
                    for j in 0..*sz {
                        for k in 0..*sz {
                            t.insert_tuple(&[i, j, k]);
                        }
                    }
                }
            })
        });

        group.bench_with_input(
            BenchmarkId::new("vanilla::BumpTrie", upper),
            &upper,
            |b, sz| {
                b.iter(|| {
                    let a = Bump::new();
                    let mut t = vanilla::BumpTrie::new_in(&a);
                    for i in 0..*sz {
                        for j in 0..*sz {
                            for k in 0..*sz {
                                t.insert_tuple(&a, &[i, j, k]);
                            }
                        }
                    }
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("sorted::Trie", upper),
            &upper,
            |b, sz| {
                b.iter(|| {
                    let a = Bump::new();
                    let mut t = sorted::Trie::new();
                    for i in 0..*sz {
                        for j in 0..*sz {
                            for k in 0..*sz {
                                t.insert_tuple(&a, &[i, j, k]);
                            }
                        }
                    }
                })
            },
        );
    }
}

fn build_nested(c: &mut Criterion) {
    let mut group = c.benchmark_group("trie, build nested (5 layers)");

    for upper in [1, 5, 10, 15] {
        group.bench_with_input(BenchmarkId::new("vanilla::Trie", upper), &upper, |b, sz| {
            b.iter(|| {
                let mut t = vanilla::Trie::new();
                for i in 0..*sz {
                    for j in 0..*sz {
                        for k in 0..*sz {
                            for l in 0..*sz {
                                for m in 0..*sz {
                                    t.insert_tuple(&[i, j, k, l, m]);
                                }
                            }
                        }
                    }
                }
            })
        });

        group.bench_with_input(
            BenchmarkId::new("vanilla::BumpTrie", upper),
            &upper,
            |b, sz| {
                b.iter(|| {
                    let a = Bump::new();
                    let mut t = vanilla::BumpTrie::new_in(&a);
                    for i in 0..*sz {
                        for j in 0..*sz {
                            for k in 0..*sz {
                                for l in 0..*sz {
                                    for m in 0..*sz {
                                        t.insert_tuple(&a, &[i, j, k, l, m]);
                                    }
                                }
                            }
                        }
                    }
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("sorted::Trie", upper),
            &upper,
            |b, sz| {
                b.iter(|| {
                    let a = Bump::new();
                    let mut t = sorted::Trie::new();
                    for i in 0..*sz {
                        for j in 0..*sz {
                            for k in 0..*sz {
                                for l in 0..*sz {
                                    for m in 0..*sz {
                                        t.insert_tuple(&a, &[i, j, k, l, m]);
                                    }
                                }
                            }
                        }
                    }
                })
            },
        );
    }
}

criterion_group! {
    name = benches;
    config = Criterion::default();
    targets = build_flat, build_mid, build_nested
}
criterion_main!(benches);
