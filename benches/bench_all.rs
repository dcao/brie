use brie::{
    sorted::{self, Trie},
    vanilla, Trieish,
};
use bumpalo::Bump;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};

fn build_flat(c: &mut Criterion) {
    let mut group = c.benchmark_group("trie, build flat (1 layer)");

    for upper in [1_000, 10_000, 100_000] {
        group.bench_with_input(BenchmarkId::new("vanilla::Trie", upper), &upper, |b, sz| {
            b.iter(|| {
                let a = Bump::new();
                let mut t = vanilla::Trie::empty(&a);
                for x in 0..*sz {
                    t.insert(&[x], &a);
                }
            })
        });

        group.bench_with_input(
            BenchmarkId::new("vanilla::BumpTrie", upper),
            &upper,
            |b, sz| {
                b.iter(|| {
                    let a = Bump::new();
                    let mut t = vanilla::BumpTrie::empty(&a);
                    for i in 0..*sz {
                        t.insert(&[i], &a);
                    }
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("sorted::nested::Trie", upper),
            &upper,
            |b, sz| {
                b.iter(|| {
                    let a = Bump::new();
                    let mut t = sorted::nested::Trie::empty(&a);
                    for i in 0..*sz {
                        t.insert(&[i], &a);
                    }
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("sorted::flat::Trie from_iter", upper),
            &upper,
            |b, sz| {
                b.iter(|| {
                    let a = Bump::new();
                    let _t: sorted::flat::Trie<'_, usize, 1, sorted::flat::Write> =
                        sorted::flat::Trie::from_iter((0..*sz).map(|x| [x]), &a);
                })
            },
        );
    }
}

// fn build_mid(c: &mut Criterion) {
//     let mut group = c.benchmark_group("trie, build mid (3 layers)");

//     for upper in [1, 5, 10, 25, 50, 100] {
//         group.bench_with_input(BenchmarkId::new("vanilla::Trie", upper), &upper, |b, sz| {
//             b.iter(|| {
//                 let mut t = vanilla::Trie::new();
//                 for i in 0..*sz {
//                     for j in 0..*sz {
//                         for k in 0..*sz {
//                             t.insert_tuple(&[i, j, k]);
//                         }
//                     }
//                 }
//             })
//         });

//         group.bench_with_input(
//             BenchmarkId::new("vanilla::BumpTrie", upper),
//             &upper,
//             |b, sz| {
//                 b.iter(|| {
//                     let a = Bump::new();
//                     let mut t = vanilla::BumpTrie::new_in(&a);
//                     for i in 0..*sz {
//                         for j in 0..*sz {
//                             for k in 0..*sz {
//                                 t.insert_tuple(&a, &[i, j, k]);
//                             }
//                         }
//                     }
//                 })
//             },
//         );

//         group.bench_with_input(
//             BenchmarkId::new("sorted::Trie", upper),
//             &upper,
//             |b, sz| {
//                 b.iter(|| {
//                     let a = Bump::new();
//                     let mut t = sorted::Trie::new();
//                     for i in 0..*sz {
//                         for j in 0..*sz {
//                             for k in 0..*sz {
//                                 t.insert_tuple(&a, &[i, j, k]);
//                             }
//                         }
//                     }
//                 })
//             },
//         );

//         group.bench_with_input(
//             BenchmarkId::new("sorted::flat::Trie", upper),
//             &upper,
//             |b, sz| {
//                 b.iter(|| {
//                     let a = Bump::new();
//                     let mut t = sorted::flat::Trie::new();
//                     for i in 0..*sz {
//                         for j in 0..*sz {
//                             for k in 0..*sz {
//                                 t.insert_tuple(&a, [i, j, k]);
//                             }
//                         }
//                     }
//                 })
//             },
//         );
//     }
// }

// fn build_nested(c: &mut Criterion) {
//     let mut group = c.benchmark_group("trie, build nested (5 layers)");

//     for upper in [1, 5, 10, 15] {
//         group.bench_with_input(BenchmarkId::new("vanilla::Trie", upper), &upper, |b, sz| {
//             b.iter(|| {
//                 let mut t = vanilla::Trie::new();
//                 for i in 0..*sz {
//                     for j in 0..*sz {
//                         for k in 0..*sz {
//                             for l in 0..*sz {
//                                 for m in 0..*sz {
//                                     t.insert_tuple(&[i, j, k, l, m]);
//                                 }
//                             }
//                         }
//                     }
//                 }
//             })
//         });

//         group.bench_with_input(
//             BenchmarkId::new("vanilla::BumpTrie", upper),
//             &upper,
//             |b, sz| {
//                 b.iter(|| {
//                     let a = Bump::new();
//                     let mut t = vanilla::BumpTrie::new_in(&a);
//                     for i in 0..*sz {
//                         for j in 0..*sz {
//                             for k in 0..*sz {
//                                 for l in 0..*sz {
//                                     for m in 0..*sz {
//                                         t.insert_tuple(&a, &[i, j, k, l, m]);
//                                     }
//                                 }
//                             }
//                         }
//                     }
//                 })
//             },
//         );

//         group.bench_with_input(
//             BenchmarkId::new("sorted::Trie", upper),
//             &upper,
//             |b, sz| {
//                 b.iter(|| {
//                     let a = Bump::new();
//                     let mut t = sorted::Trie::new();
//                     for i in 0..*sz {
//                         for j in 0..*sz {
//                             for k in 0..*sz {
//                                 for l in 0..*sz {
//                                     for m in 0..*sz {
//                                         t.insert_tuple(&a, &[i, j, k, l, m]);
//                                     }
//                                 }
//                             }
//                         }
//                     }
//                 })
//             },
//         );

//         group.bench_with_input(
//             BenchmarkId::new("sorted::nested::Trie", upper),
//             &upper,
//             |b, sz| {
//                 b.iter(|| {
//                     let a = Bump::new();
//                     let mut t = sorted::flat::Trie::new();
//                     for i in 0..*sz {
//                         for j in 0..*sz {
//                             for k in 0..*sz {
//                                 for l in 0..*sz {
//                                     for m in 0..*sz {
//                                         t.insert_tuple(&a, [i, j, k, l, m]);
//                                     }
//                                 }
//                             }
//                         }
//                     }
//                 })
//             },
//         );
//     }
// }

criterion_group! {
    name = benches;
    config = Criterion::default();
    targets = build_flat
}
criterion_main!(benches);
