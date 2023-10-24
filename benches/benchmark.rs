use core::iter::FusedIterator;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use first_err::FirstErr;

mod l1 {
    //! Tools for benchmark one layer `Result` iterator (`impl Iterator<Item = Result<_, _>>`)

    use super::*;

    /// One layer iterator.
    struct L1Iter {
        curr: u32,
        err_at: Option<u32>,
    }

    impl L1Iter {
        fn new(err_at: Option<u32>) -> Self {
            Self { curr: 0, err_at }
        }
    }

    impl Iterator for L1Iter {
        type Item = Result<u32, u32>;

        fn next(&mut self) -> Option<Self::Item> {
            let tmp = self.curr;
            self.curr += 1;

            if Some(tmp) != self.err_at {
                Some(Ok(tmp))
            } else {
                Some(Err(tmp))
            }
        }
    }

    impl FusedIterator for L1Iter {}

    /// first_err algorithm.
    fn first_err_approach(iter: impl Iterator<Item = Result<u32, u32>>) -> Result<u32, u32> {
        iter.first_err_or_else(|iter1| iter1.sum::<u32>())
    }

    /// first_err algorithm implemented by loop.
    fn loop_approach(iter: impl Iterator<Item = Result<u32, u32>>) -> Result<u32, u32> {
        let mut sum = 0;
        for res in iter {
            sum += res?;
        }

        Ok::<u32, u32>(sum)
    }

    /// first_err algorithm implemented by `collect()`.
    fn collect_approach(iter: impl Iterator<Item = Result<u32, u32>>) -> Result<u32, u32> {
        let sum = iter
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .sum::<u32>();

        Ok(sum)
    }

    /// Set L1 benchmark group by given arguments.
    pub fn bench_setup(c: &mut Criterion, err_at: Option<u32>) {
        let length: usize = 100_000;

        let group_name = match err_at {
            Some(err_at) => format!("bench_{length}_err_at_{err_at:_<7}"),
            None => format!("bench_{length}_err_not_exists"),
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
                    let input = black_box(L1Iter::new(err_at).take(length));
                    black_box(collect_approach(input))
                })
            });

            group.bench_function("_____loop", |b| {
                b.iter(|| {
                    let input = black_box(L1Iter::new(err_at).take(length));
                    black_box(loop_approach(input))
                })
            });

            group.bench_function("first_err", |b| {
                b.iter(|| {
                    let input = black_box(L1Iter::new(err_at).take(length));
                    black_box(first_err_approach(input))
                })
            });

            group.finish();
        }
    }
}

mod l2 {
    //! Tools for benchmark tow layer `Result` iterator (`impl Iterator<Item = Result<Result<_, _>, _>>`)

    use super::*;

    /// Two layer iterator.
    struct L2Iter {
        curr: u32,
        l1_err_at: Option<u32>,
        l2_err_at: Option<u32>,
    }

    impl L2Iter {
        fn new(l1_err_at: Option<u32>, l2_err_at: Option<u32>) -> Self {
            Self {
                curr: 0,
                l1_err_at,
                l2_err_at,
            }
        }
    }

    impl Iterator for L2Iter {
        type Item = Result<Result<u32, u32>, u32>;

        fn next(&mut self) -> Option<Self::Item> {
            let tmp = self.curr;
            self.curr += 1;

            // build inner Result<u32, u32>.
            let l2_res = if Some(tmp) != self.l2_err_at {
                Ok(tmp)
            } else {
                Err(tmp)
            };

            // build outer Result<Result<u32, u32>, u32>.
            let l1_res = if Some(tmp) != self.l1_err_at {
                Some(Ok(l2_res))
            } else {
                Some(Err(tmp))
            };

            l1_res
        }
    }

    impl FusedIterator for L2Iter {}

    /// first_err algorithm.
    fn first_err_approach(
        iter: impl Iterator<Item = Result<Result<u32, u32>, u32>>,
    ) -> Result<u32, u32> {
        iter.first_err_or_else(|iter1| iter1.first_err_or_else(|iter2| iter2.sum::<u32>()))
            .and_then(|res| res)
    }

    /// first_err algorithm implemented by loop.
    fn loop_approach(
        mut iter: impl Iterator<Item = Result<Result<u32, u32>, u32>>,
    ) -> Result<u32, u32> {
        let mut sum = 0;
        let mut inner_first_err: Option<u32> = None;

        for outer_res in iter {
            let inner_res = outer_res?; // return immediately when outer hit a `Err`.

            // if inner_first_err already exists, we don't care anything further, just verify
            // all outer_res ASAP.
            if inner_first_err.is_some() {
                continue;
            }

            match inner_res {
                // no `Err` found for now (both inner and outer layer)
                Ok(v) => {
                    sum += v;
                }

                // this is inner's first `Err`.
                Err(e) => {
                    inner_first_err = Some(e);
                }
            }
        }

        // At this point, we're known no outer `Err` in iter.
        if let Some(e) = inner_first_err {
            return Err(e);
        }

        // no any `Err` (both inner and outer).
        Ok::<u32, u32>(sum)
    }

    /// first_err algorithm implemented by `collect()`.
    fn collect_approach(
        iter: impl Iterator<Item = Result<Result<u32, u32>, u32>>,
    ) -> Result<u32, u32> {
        let sum = iter
            .collect::<Result<Vec<Result<u32, u32>>, u32>>()?
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .sum::<u32>();

        Ok::<u32, u32>(sum)
    }

    /// Set L2 benchmark group by given arguments.
    pub fn bench_setup(c: &mut Criterion, l1_err_at: Option<u32>, l2_err_at: Option<u32>) {
        let length: usize = 100_000;

        let group_name = match (l1_err_at, l2_err_at) {
            (Some(l1_err_at), Some(l2_err_at)) => {
                format!("bench_{length}_l1_err_at_{l1_err_at:_<7}_l2_err_at_{l2_err_at:_<7}")
            }
            (Some(l1_err_at), None) => {
                format!("bench_{length}_l1_err_at_{l1_err_at:_<7}_l2_err_at_none___")
            }
            (None, Some(l2_err_at)) => {
                format!("bench_{length}_l1_err_at_none____l2_err_at_{l2_err_at:_<7}")
            }
            (None, None) => format!("bench_{length}_err_not_exists_____________________"),
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
                    let input = black_box(L2Iter::new(l1_err_at, l2_err_at).take(length));
                    black_box(collect_approach(input))
                })
            });

            group.bench_function("_____loop", |b| {
                b.iter(|| {
                    let input = black_box(L2Iter::new(l1_err_at, l2_err_at).take(length));
                    black_box(loop_approach(input))
                })
            });

            group.bench_function("first_err", |b| {
                b.iter(|| {
                    let input = black_box(L2Iter::new(l1_err_at, l2_err_at).take(length));
                    black_box(first_err_approach(input))
                })
            });

            group.finish();
        }
    }
}

fn benchmarks(c: &mut Criterion) {
    l1::bench_setup(c, Some(0));
    l1::bench_setup(c, Some(1));
    l1::bench_setup(c, Some(10));
    l1::bench_setup(c, Some(100));
    l1::bench_setup(c, Some(1000));
    l1::bench_setup(c, Some(10000));
    l1::bench_setup(c, Some(99999));
    l1::bench_setup(c, Some(100000));
    l1::bench_setup(c, None);

    l2::bench_setup(c, Some(0), Some(1000));
    l2::bench_setup(c, Some(1), Some(1000));
    l2::bench_setup(c, Some(10), Some(1000));
    l2::bench_setup(c, Some(100), Some(1000));
    l2::bench_setup(c, Some(1000), Some(1000));
    l2::bench_setup(c, Some(10000), Some(1000));
    l2::bench_setup(c, Some(99999), Some(1000));
    l2::bench_setup(c, Some(100000), Some(1000));

    l2::bench_setup(c, Some(1000), Some(0));
    l2::bench_setup(c, Some(1000), Some(1));
    l2::bench_setup(c, Some(1000), Some(10));
    l2::bench_setup(c, Some(1000), Some(100));
    l2::bench_setup(c, Some(1000), Some(1000));
    l2::bench_setup(c, Some(1000), Some(10000));
    l2::bench_setup(c, Some(1000), Some(99999));
    l2::bench_setup(c, Some(1000), Some(100000));
    l2::bench_setup(c, Some(1000), None);

    l2::bench_setup(c, Some(0), None);
    l2::bench_setup(c, Some(1), None);
    l2::bench_setup(c, Some(10), None);
    l2::bench_setup(c, Some(100), None);
    l2::bench_setup(c, Some(1000), None);
    l2::bench_setup(c, Some(10000), None);
    l2::bench_setup(c, Some(99999), None);
    l2::bench_setup(c, Some(100000), None);

    l2::bench_setup(c, None, Some(0));
    l2::bench_setup(c, None, Some(1));
    l2::bench_setup(c, None, Some(10));
    l2::bench_setup(c, None, Some(100));
    l2::bench_setup(c, None, Some(1000));
    l2::bench_setup(c, None, Some(10000));
    l2::bench_setup(c, None, Some(99999));
    l2::bench_setup(c, None, Some(100000));
}

criterion_group!(benches, benchmarks);
criterion_main!(benches);
