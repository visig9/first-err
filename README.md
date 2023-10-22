- [Api Document](https://docs.rs/first-err)



# `first-err`

Find the first `Err` in `Iterator<Result<T, E>>` and allow continuous iteration.

This crate is specifically designed to replace the following pattern without allocation:

```txt
// iter: impl Iterator<Result<T, E>>
iter.collect::<Result<Vec<T>, E>>().map(|vec| vec.into_iter().foo() );
```



## Features

- Find first `Err` in `Iterator<Result<T, E>>` and allow to iterating continuously.
- Speed: Roughly on par with a hand-written loop, using lazy evaluation and without allocation.
- Minimized: no `std`, no `alloc`, no dependency.



## Getting Started

```rust
use first_err::FirstErr;

// Everything is Ok.
let result = [Ok::<u8, u8>(0), Ok(1), Ok(2)]
    .into_iter()
    .first_err_or_else(|iter| iter.sum::<u8>());
assert_eq!(result, Ok(3));

// Contains some `Err` values.
let result = [Ok::<u8, u8>(0), Err(1), Err(2)]
    .into_iter()
    .first_err_or_else(|iter| iter.sum::<u8>());
assert_eq!(result, Err(1));
```



## Why

In Rust, I frequently encounter a pattern where I need to perform actions on all
items within an iterator, and halt immediately if any error is detected in the layer
I'm working on. But if no error found, the iterator should able to run continuously
and allow me to do further transform.

The pattern typically looks as follows:

```rust
let array: [Result<u8, u8>; 3] = [Ok(0), Err(1), Err(2)];

fn fallible_sum(iter: impl IntoIterator<Item = Result<u8, u8>>) -> Result<u8, u8> {
    let sum = iter
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?    // early return (and a vec alloc in here)
        .into_iter()                        // continue iterate next layer ...
        .sum();

    Ok(sum)
}

let result = fallible_sum(array);
assert_eq!(result, Err(1));
```

In theory, this allocation is not necessary. We can just write that code as an old good
loop:

```rust
let array: [Result<u8, u8>; 3] = [Ok(0), Err(1), Err(2)];

fn fallible_sum(iter: impl IntoIterator<Item = Result<u8, u8>>) -> Result<u8, u8> {
    let mut sum = 0;
    for res in iter {
        let val = res?;                     // early return, no alloc
        sum += val;
    }

    Ok(sum)
}

let result = fallible_sum(array);
assert_eq!(result, Err(1))
```

Using a loop is not bad at all. However, in some situations, maintaining a chainable iterator
is preferable.

Furthermore, some scenarios may not be as simple as the previous example. Consider this one:

```rust
// The second layer `Result` is usually created by further transform after the first layer
// `Result` be processed. But for the sake of simplicity, we've just use pre-defined values.
let array: [Result<Result<u8, u8>, u8>; 3] = [Ok(Ok(0)), Ok(Err(1)), Err(2)];

fn fallible_sum(
    iter: impl IntoIterator<Item = Result<Result<u8, u8>, u8>>
) -> Result<u8, u8> {
    // take "first `Err`" layer by layer, or the sum value.
    let sum = iter
        .into_iter()
        .collect::<Result<Vec<Result<u8, u8>>, u8>>()?
        .into_iter()
        .collect::<Result<Vec<u8>, u8>>()?
        .into_iter()
        .sum();

    Ok(sum)
}

let result = fallible_sum(array);
assert_eq!(result, Err(2));
```

Implementing the above logic in a loop without allocation may be error-prone and complicated.
This crate simplifies that for you:

```rust
let array: [Result<Result<u8, u8>, u8>; 3] = [Ok(Ok(0)), Ok(Err(1)), Err(2)];

fn fallible_sum(
    iter: impl IntoIterator<Item = Result<Result<u8, u8>, u8>>
) -> Result<u8, u8> {
    iter
        .into_iter()
        .first_err_or_else(|iter1| { // iter1 = impl Iterator<Item = Result<u8, u8>>
            iter1.first_err_or_else(|iter2| { // iter2 = impl Iterator<Item = u8>
                iter2.sum::<u8>()
            })
        })
        .and_then(|res_res| res_res)
}

let result = fallible_sum(array);
assert_eq!(result, Err(2));
```



## Benchmark

The performance of this crate is designed to be roughly on par with hand-written loops.
However, the compiler might apply different optimizations in various situations, and favoring
one approach over the others.

If you want to to do a benchmark by yourself, use the following command:

```sh
cargo bench --bench benchmark -- --output-format bencher
```

Also, don't forget to check the actual code that is used for benchmarking, which is in the
`benches` folder.



<details>
  <summary>Click to see benchmark results</summary>
  <p>

  ### Environment

  - cpu: AMD Ryzen 5 3600 6-Core Processor
  - os: Debian GNU/Linux 12 (bookworm)
  - kernel: Linux 6.1.0-10-amd64 #1 SMP PREEMPT_DYNAMIC Debian 6.1.38-1 (2023-07-14)
  - rustc: 1.72.0 (5680fa18f 2023-08-23)
  - cargo: 1.72.0 (103a7ff2e 2023-08-15)
  - first-err: v0.1.0
  - date: 2023-10-21

  ### Results

  ```txt
  test bench_100000_err_at_0______/__collect ... bench:          13 ns/iter (+/- 0)
  test bench_100000_err_at_0______/_____loop ... bench:           1 ns/iter (+/- 0)
  test bench_100000_err_at_0______/first_err ... bench:           1 ns/iter (+/- 0)

  test bench_100000_err_at_1______/__collect ... bench:          19 ns/iter (+/- 0)
  test bench_100000_err_at_1______/_____loop ... bench:           2 ns/iter (+/- 0)
  test bench_100000_err_at_1______/first_err ... bench:           2 ns/iter (+/- 0)

  test bench_100000_err_at_10_____/__collect ... bench:          82 ns/iter (+/- 1)
  test bench_100000_err_at_10_____/_____loop ... bench:           3 ns/iter (+/- 0)
  test bench_100000_err_at_10_____/first_err ... bench:           3 ns/iter (+/- 0)

  test bench_100000_err_at_100____/__collect ... bench:         276 ns/iter (+/- 3)
  test bench_100000_err_at_100____/_____loop ... bench:           9 ns/iter (+/- 0)
  test bench_100000_err_at_100____/first_err ... bench:           7 ns/iter (+/- 0)

  test bench_100000_err_at_1000___/__collect ... bench:        1027 ns/iter (+/- 39)
  test bench_100000_err_at_1000___/_____loop ... bench:          66 ns/iter (+/- 1)
  test bench_100000_err_at_1000___/first_err ... bench:          60 ns/iter (+/- 0)

  test bench_100000_err_at_10000__/__collect ... bench:        6168 ns/iter (+/- 149)
  test bench_100000_err_at_10000__/_____loop ... bench:         604 ns/iter (+/- 10)
  test bench_100000_err_at_10000__/first_err ... bench:         605 ns/iter (+/- 1)

  test bench_100000_err_at_99999__/__collect ... bench:       57075 ns/iter (+/- 344)
  test bench_100000_err_at_99999__/_____loop ... bench:        5985 ns/iter (+/- 19)
  test bench_100000_err_at_99999__/first_err ... bench:        5980 ns/iter (+/- 11)

  test bench_100000_err_at_100000_/__collect ... bench:       60171 ns/iter (+/- 4611)
  test bench_100000_err_at_100000_/_____loop ... bench:        5982 ns/iter (+/- 8)
  test bench_100000_err_at_100000_/first_err ... bench:        5987 ns/iter (+/- 33)

  test bench_100000_err_not_exists/__collect ... bench:       58460 ns/iter (+/- 343)
  test bench_100000_err_not_exists/_____loop ... bench:           1 ns/iter (+/- 0)
  test bench_100000_err_not_exists/first_err ... bench:           1 ns/iter (+/- 0)

  test bench_100000_l1_err_at_0_______l2_err_at_1000___/__collect ... bench:          14 ns/iter (+/- 0)
  test bench_100000_l1_err_at_0_______l2_err_at_1000___/_____loop ... bench:           3 ns/iter (+/- 0)
  test bench_100000_l1_err_at_0_______l2_err_at_1000___/first_err ... bench:           6 ns/iter (+/- 0)

  test bench_100000_l1_err_at_1_______l2_err_at_1000___/__collect ... bench:          21 ns/iter (+/- 0)
  test bench_100000_l1_err_at_1_______l2_err_at_1000___/_____loop ... bench:           3 ns/iter (+/- 0)
  test bench_100000_l1_err_at_1_______l2_err_at_1000___/first_err ... bench:           6 ns/iter (+/- 0)

  test bench_100000_l1_err_at_10______l2_err_at_1000___/__collect ... bench:          95 ns/iter (+/- 1)
  test bench_100000_l1_err_at_10______l2_err_at_1000___/_____loop ... bench:          13 ns/iter (+/- 0)
  test bench_100000_l1_err_at_10______l2_err_at_1000___/first_err ... bench:          10 ns/iter (+/- 0)

  test bench_100000_l1_err_at_100_____l2_err_at_1000___/__collect ... bench:         362 ns/iter (+/- 3)
  test bench_100000_l1_err_at_100_____l2_err_at_1000___/_____loop ... bench:         110 ns/iter (+/- 2)
  test bench_100000_l1_err_at_100_____l2_err_at_1000___/first_err ... bench:          72 ns/iter (+/- 0)

  test bench_100000_l1_err_at_1000____l2_err_at_1000___/__collect ... bench:        1319 ns/iter (+/- 14)
  test bench_100000_l1_err_at_1000____l2_err_at_1000___/_____loop ... bench:        1020 ns/iter (+/- 15)
  test bench_100000_l1_err_at_1000____l2_err_at_1000___/first_err ... bench:         626 ns/iter (+/- 6)

  test bench_100000_l1_err_at_10000___l2_err_at_1000___/__collect ... bench:        9883 ns/iter (+/- 633)
  test bench_100000_l1_err_at_10000___l2_err_at_1000___/_____loop ... bench:        1115 ns/iter (+/- 5)
  test bench_100000_l1_err_at_10000___l2_err_at_1000___/first_err ... bench:         774 ns/iter (+/- 52)

  test bench_100000_l1_err_at_99999___l2_err_at_1000___/__collect ... bench:       94787 ns/iter (+/- 330)
  test bench_100000_l1_err_at_99999___l2_err_at_1000___/_____loop ... bench:        1780 ns/iter (+/- 4)
  test bench_100000_l1_err_at_99999___l2_err_at_1000___/first_err ... bench:        2123 ns/iter (+/- 5)

  test bench_100000_l1_err_at_100000__l2_err_at_1000___/__collect ... bench:       96160 ns/iter (+/- 161)
  test bench_100000_l1_err_at_100000__l2_err_at_1000___/_____loop ... bench:        1787 ns/iter (+/- 3)
  test bench_100000_l1_err_at_100000__l2_err_at_1000___/first_err ... bench:        2118 ns/iter (+/- 123)

  test bench_100000_l1_err_at_none____l2_err_at_0______/__collect ... bench:       89359 ns/iter (+/- 309)
  test bench_100000_l1_err_at_none____l2_err_at_0______/_____loop ... bench:           3 ns/iter (+/- 0)
  test bench_100000_l1_err_at_none____l2_err_at_0______/first_err ... bench:           6 ns/iter (+/- 0)

  test bench_100000_l1_err_at_none____l2_err_at_1______/__collect ... bench:       89247 ns/iter (+/- 211)
  test bench_100000_l1_err_at_none____l2_err_at_1______/_____loop ... bench:           4 ns/iter (+/- 0)
  test bench_100000_l1_err_at_none____l2_err_at_1______/first_err ... bench:           6 ns/iter (+/- 0)

  test bench_100000_l1_err_at_none____l2_err_at_10_____/__collect ... bench:       89375 ns/iter (+/- 131)
  test bench_100000_l1_err_at_none____l2_err_at_10_____/_____loop ... bench:          13 ns/iter (+/- 0)
  test bench_100000_l1_err_at_none____l2_err_at_10_____/first_err ... bench:          11 ns/iter (+/- 0)

  test bench_100000_l1_err_at_none____l2_err_at_100____/__collect ... bench:       89231 ns/iter (+/- 161)
  test bench_100000_l1_err_at_none____l2_err_at_100____/_____loop ... bench:         106 ns/iter (+/- 0)
  test bench_100000_l1_err_at_none____l2_err_at_100____/first_err ... bench:          73 ns/iter (+/- 2)

  test bench_100000_l1_err_at_none____l2_err_at_1000___/__collect ... bench:       89982 ns/iter (+/- 84)
  test bench_100000_l1_err_at_none____l2_err_at_1000___/_____loop ... bench:         966 ns/iter (+/- 0)
  test bench_100000_l1_err_at_none____l2_err_at_1000___/first_err ... bench:         616 ns/iter (+/- 3)

  test bench_100000_l1_err_at_none____l2_err_at_10000__/__collect ... bench:       96578 ns/iter (+/- 352)
  test bench_100000_l1_err_at_none____l2_err_at_10000__/_____loop ... bench:        9569 ns/iter (+/- 47)
  test bench_100000_l1_err_at_none____l2_err_at_10000__/first_err ... bench:        6058 ns/iter (+/- 8)

  test bench_100000_l1_err_at_none____l2_err_at_99999__/__collect ... bench:      173763 ns/iter (+/- 646)
  test bench_100000_l1_err_at_none____l2_err_at_99999__/_____loop ... bench:       95541 ns/iter (+/- 490)
  test bench_100000_l1_err_at_none____l2_err_at_99999__/first_err ... bench:       60196 ns/iter (+/- 539)

  test bench_100000_l1_err_at_none____l2_err_at_100000_/__collect ... bench:      167568 ns/iter (+/- 1418)
  test bench_100000_l1_err_at_none____l2_err_at_100000_/_____loop ... bench:       95596 ns/iter (+/- 178)
  test bench_100000_l1_err_at_none____l2_err_at_100000_/first_err ... bench:       50250 ns/iter (+/- 3401)

  test bench_100000_err_not_exists_____________________/__collect ... bench:      182434 ns/iter (+/- 1666)
  test bench_100000_err_not_exists_____________________/_____loop ... bench:       95634 ns/iter (+/- 82)
  test bench_100000_err_not_exists_____________________/first_err ... bench:           5 ns/iter (+/- 0)
  ```

  </p>
</details>



## Licence

MIT
