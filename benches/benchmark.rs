//! Benchmark the methods of `first-err`.
//!
//! What's the things should take into account:
//!
//! 1. Verify the performances of a series of functions with same algorithm which
//!    implemented by `first_err`, `loop` and `collect` approachs.
//! 2. Do not try to avoid compiler optimization, include inline, to align the usual use cases.
//! 3. Isolating the optimizations between the `*_approach` code and benchmarking harness.

use core::{hint::black_box, iter::FusedIterator};
use criterion::{criterion_group, criterion_main, Criterion};
use first_err::FirstErr;

mod l1res {
    use super::*;

    /// One layer iterator.
    struct L1Iter {
        curr: u64,
        err_at: Option<u64>,
    }

    impl L1Iter {
        fn new(err_at: Option<u64>) -> Self {
            Self { curr: 0, err_at }
        }
    }

    impl Iterator for L1Iter {
        type Item = Result<u64, u64>;

        fn next(&mut self) -> Option<Self::Item> {
            let tmp = self.curr;
            self.curr += 1;

            let res = if Some(tmp) != self.err_at {
                Some(Ok(tmp))
            } else {
                Some(Err(tmp))
            };

            // treat output of this iterator is a black box
            black_box(res)
        }
    }

    impl FusedIterator for L1Iter {}

    /// The code implemented by first_err.
    #[inline(never)]
    fn first_err_approach(iter: impl Iterator<Item = Result<u64, u64>>) -> Result<u64, u64> {
        iter.first_err_or_else(|iter1| iter1.sum::<u64>())
    }

    /// The code implemented by loop.
    #[inline(never)]
    fn loop_approach(iter: impl Iterator<Item = Result<u64, u64>>) -> Result<u64, u64> {
        let mut sum = 0;
        for res in iter {
            sum += res?;
        }

        Ok::<u64, u64>(sum)
    }

    /// The code implemented by `collect()`.
    #[inline(never)]
    fn collect_approach(iter: impl Iterator<Item = Result<u64, u64>>) -> Result<u64, u64> {
        let sum = iter
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .sum::<u64>();

        Ok(sum)
    }

    /// Set L1 benchmark group by given arguments.
    pub fn bench_setup(c: &mut Criterion, err_at: Option<u64>) {
        let length: usize = 100_000;

        let group_name = match err_at {
            Some(err_at) => format!("l1res::err_at_{err_at:_<7}"),
            None => format!("l1res::err_not_exists"),
        };

        // TEST: make sure answers are the same.
        {
            let collect_ans = black_box(collect_approach(black_box(
                L1Iter::new(err_at).take(length),
            )));

            assert_eq!(
                collect_ans,
                black_box(loop_approach(black_box(L1Iter::new(err_at).take(length)))),
                "loop approach test in: {group_name}",
            );
            assert_eq!(
                collect_ans,
                black_box(first_err_approach(black_box(
                    L1Iter::new(err_at).take(length)
                ))),
                "first_err approach test in: {group_name}",
            );
        }

        // benchmark conf
        {
            let mut group = c.benchmark_group(group_name);

            group.bench_function("__collect", |b| {
                b.iter(|| {
                    black_box(collect_approach(black_box(
                        L1Iter::new(err_at).take(length),
                    )))
                })
            });

            group.bench_function("_____loop", |b| {
                b.iter(|| black_box(loop_approach(black_box(L1Iter::new(err_at).take(length)))))
            });

            group.bench_function("first_err", |b| {
                b.iter(|| {
                    black_box(first_err_approach(black_box(
                        L1Iter::new(err_at).take(length),
                    )))
                })
            });

            group.finish();
        }
    }
}

mod l2res {
    use super::*;

    /// Two layer iterator.
    struct L2Iter {
        curr: u64,
        l1_err_at: Option<u64>,
        l2_err_at: Option<u64>,
    }

    impl L2Iter {
        fn new(l1_err_at: Option<u64>, l2_err_at: Option<u64>) -> Self {
            Self {
                curr: 0,
                l1_err_at,
                l2_err_at,
            }
        }
    }

    impl Iterator for L2Iter {
        type Item = Result<Result<u64, u64>, u64>;

        fn next(&mut self) -> Option<Self::Item> {
            let tmp = self.curr;
            self.curr += 1;

            // build inner Result<u64, u64>.
            let l2_res = if Some(tmp) != self.l2_err_at {
                Ok(tmp)
            } else {
                Err(tmp)
            };

            // build outer Result<Result<u64, u64>, u64>.
            let l1_res = if Some(tmp) != self.l1_err_at {
                Some(Ok(l2_res))
            } else {
                Some(Err(tmp))
            };

            // treat output of this iterator is a black box
            black_box(l1_res)
        }
    }

    impl FusedIterator for L2Iter {}

    /// The code implemented by first_err.
    #[inline(never)]
    fn first_err_approach(
        iter: impl Iterator<Item = Result<Result<u64, u64>, u64>>,
    ) -> Result<u64, u64> {
        iter.first_err_or_else(|iter1| iter1.first_err_or_else(|iter2| iter2.sum::<u64>()))
            .and_then(|res| res)
    }

    /// The code implemented by loop.
    #[inline(never)]
    fn loop_approach(
        mut iter: impl Iterator<Item = Result<Result<u64, u64>, u64>>,
    ) -> Result<u64, u64> {
        let mut sum = 0;
        let mut inner_first_err: Option<u64> = None;

        while let Some(outer_res) = iter.next() {
            let inner_res = outer_res?; // return immediately when outer hit a `Err`.

            match inner_res {
                // no `Err` found for now (both inner and outer layer)
                Ok(v) => {
                    sum += v;
                }

                // this is inner's first `Err`.
                Err(e) => {
                    inner_first_err = Some(e);

                    // inner_first_err already exists, we don't care anything further,
                    // just verify all outer_res ASAP.
                    for outer_res in iter {
                        let _ = outer_res?;
                    }

                    break;
                }
            }
        }

        // At this point, we're known no outer `Err` in iter.
        if let Some(e) = inner_first_err {
            return Err(e);
        }

        // no any `Err` (both inner and outer).
        Ok::<u64, u64>(sum)
    }

    /// The code implemented by `collect()`.
    #[inline(never)]
    fn collect_approach(
        iter: impl Iterator<Item = Result<Result<u64, u64>, u64>>,
    ) -> Result<u64, u64> {
        let sum = iter
            .collect::<Result<Vec<Result<u64, u64>>, u64>>()?
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .sum::<u64>();

        Ok::<u64, u64>(sum)
    }

    /// Set L2 benchmark group by given arguments.
    pub fn bench_setup(c: &mut Criterion, l1_err_at: Option<u64>, l2_err_at: Option<u64>) {
        let length: usize = 100_000;

        let group_name = match (l1_err_at, l2_err_at) {
            (Some(l1_err_at), Some(l2_err_at)) => {
                format!("l2res::l1_err_at_{l1_err_at:_<7}_l2_err_at_{l2_err_at:_<7}")
            }
            (Some(l1_err_at), None) => {
                format!("l2res::l1_err_at_{l1_err_at:_<7}_l2_err_not_exists")
            }
            (None, Some(l2_err_at)) => {
                format!("l2res::l1_err_not_exists_l2_err_at_{l2_err_at:_<7}")
            }
            (None, None) => format!("l2res::l1_err_not_exists_l2_err_not_exists"),
        };

        // TEST: make sure answers are the same.
        {
            let collect_ans = black_box(collect_approach(black_box(
                L2Iter::new(l1_err_at, l2_err_at).take(length),
            )));

            assert_eq!(
                collect_ans,
                black_box(loop_approach(black_box(
                    L2Iter::new(l1_err_at, l2_err_at).take(length)
                ))),
                "loop approach test in: {group_name}",
            );
            assert_eq!(
                collect_ans,
                black_box(first_err_approach(black_box(
                    L2Iter::new(l1_err_at, l2_err_at).take(length)
                ))),
                "first_err approach test in: {group_name}",
            );
        }

        // benchmark conf
        {
            let mut group = c.benchmark_group(group_name);

            group.bench_function("__collect", |b| {
                b.iter(|| {
                    black_box(collect_approach(black_box(
                        L2Iter::new(l1_err_at, l2_err_at).take(length),
                    )))
                })
            });

            group.bench_function("_____loop", |b| {
                b.iter(|| {
                    black_box(loop_approach(black_box(
                        L2Iter::new(l1_err_at, l2_err_at).take(length),
                    )))
                })
            });

            group.bench_function("first_err", |b| {
                b.iter(|| {
                    black_box(first_err_approach(black_box(
                        L2Iter::new(l1_err_at, l2_err_at).take(length),
                    )))
                })
            });

            group.finish();
        }
    }
}

mod l1opt {
    use super::*;

    /// One layer iterator.
    struct L1Iter {
        curr: u64,
        none_at: Option<u64>,
    }

    impl L1Iter {
        fn new(none_at: Option<u64>) -> Self {
            Self { curr: 0, none_at }
        }
    }

    impl Iterator for L1Iter {
        type Item = Option<u64>;

        fn next(&mut self) -> Option<Self::Item> {
            let tmp = self.curr;
            self.curr += 1;

            let res = if Some(tmp) != self.none_at {
                Some(Some(tmp))
            } else {
                Some(None)
            };

            // treat output of this iterator is a black box
            black_box(res)
        }
    }

    impl FusedIterator for L1Iter {}

    /// The code implemented by first_err.
    #[inline(never)]
    fn first_err_approach(iter: impl Iterator<Item = Option<u64>>) -> Option<u64> {
        iter.first_none_or_else(|iter1| iter1.sum::<u64>())
    }

    /// The code implemented by loop.
    #[inline(never)]
    fn loop_approach(iter: impl Iterator<Item = Option<u64>>) -> Option<u64> {
        let mut sum = 0;
        for opt in iter {
            sum += opt?;
        }

        Some(sum)
    }

    /// The code implemented by `collect()`.
    #[inline(never)]
    fn collect_approach(iter: impl Iterator<Item = Option<u64>>) -> Option<u64> {
        let sum = iter.collect::<Option<Vec<u64>>>()?.into_iter().sum::<u64>();

        Some(sum)
    }

    /// Set L1 benchmark group by given arguments.
    pub fn bench_setup(c: &mut Criterion, none_at: Option<u64>) {
        let length: usize = 100_000;

        let group_name = match none_at {
            Some(none_at) => format!("l1opt::none_at_{none_at:_<7}"),
            None => format!("l1opt::none_not_exists"),
        };

        // TEST: make sure answers are the same.
        {
            let collect_ans = black_box(collect_approach(black_box(
                L1Iter::new(none_at).take(length),
            )));

            assert_eq!(
                collect_ans,
                black_box(loop_approach(black_box(L1Iter::new(none_at).take(length)))),
                "loop approach test in: {group_name}",
            );
            assert_eq!(
                collect_ans,
                black_box(first_err_approach(black_box(
                    L1Iter::new(none_at).take(length)
                ))),
                "first_err approach test in: {group_name}",
            );
        }

        // benchmark conf
        {
            let mut group = c.benchmark_group(group_name);

            group.bench_function("__collect", |b| {
                b.iter(|| {
                    black_box(collect_approach(black_box(
                        L1Iter::new(none_at).take(length),
                    )))
                })
            });

            group.bench_function("_____loop", |b| {
                b.iter(|| black_box(loop_approach(black_box(L1Iter::new(none_at).take(length)))))
            });

            group.bench_function("first_err", |b| {
                b.iter(|| {
                    black_box(first_err_approach(black_box(
                        L1Iter::new(none_at).take(length),
                    )))
                })
            });

            group.finish();
        }
    }
}

mod l2opt {
    use super::*;

    /// Two layer iterator.
    struct L2Iter {
        curr: u64,
        l1_none_at: Option<u64>,
        l2_none_at: Option<u64>,
    }

    impl L2Iter {
        fn new(l1_none_at: Option<u64>, l2_none_at: Option<u64>) -> Self {
            Self {
                curr: 0,
                l1_none_at,
                l2_none_at,
            }
        }
    }

    impl Iterator for L2Iter {
        type Item = Result<Result<u64, u64>, u64>;

        fn next(&mut self) -> Option<Self::Item> {
            let tmp = self.curr;
            self.curr += 1;

            // build inner Result<u64, u64>.
            let l2_res = if Some(tmp) != self.l2_none_at {
                Ok(tmp)
            } else {
                Err(tmp)
            };

            // build outer Result<Result<u64, u64>, u64>.
            let l1_res = if Some(tmp) != self.l1_none_at {
                Some(Ok(l2_res))
            } else {
                Some(Err(tmp))
            };

            // treat output of this iterator is a black box
            black_box(l1_res)
        }
    }

    impl FusedIterator for L2Iter {}

    /// The code implemented by first_err.
    #[inline(never)]
    fn first_err_approach(
        iter: impl Iterator<Item = Result<Result<u64, u64>, u64>>,
    ) -> Result<u64, u64> {
        iter.first_err_or_else(|iter1| iter1.first_err_or_else(|iter2| iter2.sum::<u64>()))
            .and_then(|res| res)
    }

    /// The code implemented by loop.
    #[inline(never)]
    fn loop_approach(
        mut iter: impl Iterator<Item = Result<Result<u64, u64>, u64>>,
    ) -> Result<u64, u64> {
        let mut sum = 0;
        let mut inner_first_err: Option<u64> = None;

        while let Some(outer_res) = iter.next() {
            let inner_res = outer_res?; // return immediately when outer hit a `Err`.

            match inner_res {
                // no `Err` found for now (both inner and outer layer)
                Ok(v) => {
                    sum += v;
                }

                // this is inner's first `Err`.
                Err(e) => {
                    inner_first_err = Some(e);

                    // inner_first_err already exists, we don't care anything further,
                    // just verify all outer_res ASAP.
                    for outer_res in iter {
                        let _ = outer_res?;
                    }

                    break;
                }
            }
        }

        // At this point, we're known no outer `Err` in iter.
        if let Some(e) = inner_first_err {
            return Err(e);
        }

        // no any `Err` (both inner and outer).
        Ok::<u64, u64>(sum)
    }

    /// The code implemented by `collect()`.
    #[inline(never)]
    fn collect_approach(
        iter: impl Iterator<Item = Result<Result<u64, u64>, u64>>,
    ) -> Result<u64, u64> {
        let sum = iter
            .collect::<Result<Vec<Result<u64, u64>>, u64>>()?
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .sum::<u64>();

        Ok::<u64, u64>(sum)
    }

    /// Set L2 benchmark group by given arguments.
    pub fn bench_setup(c: &mut Criterion, l1_none_at: Option<u64>, l2_none_at: Option<u64>) {
        let length: usize = 100_000;

        let group_name = match (l1_none_at, l2_none_at) {
            (Some(l1_none_at), Some(l2_none_at)) => {
                format!("l2opt::l1_none_at_{l1_none_at:_<7}_l2_none_at_{l2_none_at:_<7}")
            }
            (Some(l1_none_at), None) => {
                format!("l2opt::l1_none_at_{l1_none_at:_<7}_l2_none_not_exists")
            }
            (None, Some(l2_none_at)) => {
                format!("l2opt::l1_none_not_exists_l2_none_at_{l2_none_at:_<7}")
            }
            (None, None) => format!("l2opt::l1_none_not_exists_l2_none_not_exists"),
        };

        // TEST: make sure answers are the same.
        {
            let collect_ans = black_box(collect_approach(black_box(
                L2Iter::new(l1_none_at, l2_none_at).take(length),
            )));

            assert_eq!(
                collect_ans,
                black_box(loop_approach(black_box(
                    L2Iter::new(l1_none_at, l2_none_at).take(length)
                ))),
                "loop approach test in: {group_name}",
            );
            assert_eq!(
                collect_ans,
                black_box(first_err_approach(black_box(
                    L2Iter::new(l1_none_at, l2_none_at).take(length)
                ))),
                "first_err approach test in: {group_name}",
            );
        }

        // benchmark conf
        {
            let mut group = c.benchmark_group(group_name);

            group.bench_function("__collect", |b| {
                b.iter(|| {
                    black_box(collect_approach(black_box(
                        L2Iter::new(l1_none_at, l2_none_at).take(length),
                    )))
                })
            });

            group.bench_function("_____loop", |b| {
                b.iter(|| {
                    black_box(loop_approach(black_box(
                        L2Iter::new(l1_none_at, l2_none_at).take(length),
                    )))
                })
            });

            group.bench_function("first_err", |b| {
                b.iter(|| {
                    black_box(first_err_approach(black_box(
                        L2Iter::new(l1_none_at, l2_none_at).take(length),
                    )))
                })
            });

            group.finish();
        }
    }
}

fn benchmarks(c: &mut Criterion) {
    // result

    l1res::bench_setup(c, Some(0));
    l1res::bench_setup(c, Some(10));
    l1res::bench_setup(c, Some(100));
    l1res::bench_setup(c, Some(1000));
    l1res::bench_setup(c, Some(10000));
    l1res::bench_setup(c, Some(99999));
    l1res::bench_setup(c, None);

    l2res::bench_setup(c, Some(0), Some(1000));
    l2res::bench_setup(c, Some(10), Some(1000));
    l2res::bench_setup(c, Some(100), Some(1000));
    l2res::bench_setup(c, Some(1000), Some(1000));
    l2res::bench_setup(c, Some(10000), Some(1000));
    l2res::bench_setup(c, Some(99999), Some(1000));
    l2res::bench_setup(c, None, Some(1000));

    l2res::bench_setup(c, Some(1000), Some(0));
    l2res::bench_setup(c, Some(1000), Some(10));
    l2res::bench_setup(c, Some(1000), Some(100));
    l2res::bench_setup(c, Some(1000), Some(1000));
    l2res::bench_setup(c, Some(1000), Some(10000));
    l2res::bench_setup(c, Some(1000), Some(99999));
    l2res::bench_setup(c, Some(1000), None);

    l2res::bench_setup(c, None, Some(0));
    l2res::bench_setup(c, None, Some(10));
    l2res::bench_setup(c, None, Some(100));
    l2res::bench_setup(c, None, Some(1000));
    l2res::bench_setup(c, None, Some(10000));
    l2res::bench_setup(c, None, Some(99999));

    l2res::bench_setup(c, Some(0), None);
    l2res::bench_setup(c, Some(10), None);
    l2res::bench_setup(c, Some(100), None);
    l2res::bench_setup(c, Some(1000), None);
    l2res::bench_setup(c, Some(10000), None);
    l2res::bench_setup(c, Some(99999), None);

    l2res::bench_setup(c, None, None);

    // option

    l1opt::bench_setup(c, Some(0));
    l1opt::bench_setup(c, Some(10));
    l1opt::bench_setup(c, Some(100));
    l1opt::bench_setup(c, Some(1000));
    l1opt::bench_setup(c, Some(10000));
    l1opt::bench_setup(c, Some(99999));
    l1opt::bench_setup(c, None);

    l2opt::bench_setup(c, Some(0), Some(1000));
    l2opt::bench_setup(c, Some(10), Some(1000));
    l2opt::bench_setup(c, Some(100), Some(1000));
    l2opt::bench_setup(c, Some(1000), Some(1000));
    l2opt::bench_setup(c, Some(10000), Some(1000));
    l2opt::bench_setup(c, Some(99999), Some(1000));
    l2opt::bench_setup(c, None, Some(1000));

    l2opt::bench_setup(c, Some(1000), Some(0));
    l2opt::bench_setup(c, Some(1000), Some(10));
    l2opt::bench_setup(c, Some(1000), Some(100));
    l2opt::bench_setup(c, Some(1000), Some(1000));
    l2opt::bench_setup(c, Some(1000), Some(10000));
    l2opt::bench_setup(c, Some(1000), Some(99999));
    l2opt::bench_setup(c, Some(1000), None);

    l2opt::bench_setup(c, None, Some(0));
    l2opt::bench_setup(c, None, Some(10));
    l2opt::bench_setup(c, None, Some(100));
    l2opt::bench_setup(c, None, Some(1000));
    l2opt::bench_setup(c, None, Some(10000));
    l2opt::bench_setup(c, None, Some(99999));

    l2opt::bench_setup(c, Some(0), None);
    l2opt::bench_setup(c, Some(10), None);
    l2opt::bench_setup(c, Some(100), None);
    l2opt::bench_setup(c, Some(1000), None);
    l2opt::bench_setup(c, Some(10000), None);
    l2opt::bench_setup(c, Some(99999), None);

    l2opt::bench_setup(c, None, None);
}

criterion_group!(benches, benchmarks);
criterion_main!(benches);
