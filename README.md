- [API Document](https://docs.rs/first-err)



# `first-err`

Find the first `Err` in `Iterator<Item = Result<T, E>>` and allow iterating continuously.

This crate is specifically designed to replace the following pattern without allocation:

```txt
// iter: impl Iterator<Item = Result<T, E>>
iter.collect::<Result<Vec<T>, E>>().map(|vec| vec.into_iter().foo() );
```



## Features

- Easy-to-use: simple and no way to using wrong.
- Minimized: no `std`, no `alloc`, zero dependency.
- Fast: Roughly on par with a hand-written loop, using lazy evaluation and no allocation.
- Nestable: `T` in `Iterator<Item = Result<T, E>>` can lazily produce more `Result`s.



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

Implementing the above logic in a loop without allocation may be
[error-prone and complicated](https://github.com/visig9/first-err/blob/f666ef16c0b72174e7862468f1dbda382ebe6b68/benches/benchmark.rs#L194-L231).
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



## Performance

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
  - first-err: v0.2.0
  - date: 2023-10-28

  ### Results

  ```txt
  test l1res::err_at_0______/__collect ... bench:          11 ns/iter (+/- 0)
  test l1res::err_at_0______/_____loop ... bench:           1 ns/iter (+/- 0)
  test l1res::err_at_0______/first_err ... bench:           2 ns/iter (+/- 0)

  test l1res::err_at_10_____/__collect ... bench:         103 ns/iter (+/- 1)
  test l1res::err_at_10_____/_____loop ... bench:          10 ns/iter (+/- 0)
  test l1res::err_at_10_____/first_err ... bench:          11 ns/iter (+/- 6)

  test l1res::err_at_100____/__collect ... bench:         314 ns/iter (+/- 3)
  test l1res::err_at_100____/_____loop ... bench:          89 ns/iter (+/- 38)
  test l1res::err_at_100____/first_err ... bench:          89 ns/iter (+/- 3)

  test l1res::err_at_1000___/__collect ... bench:        1542 ns/iter (+/- 531)
  test l1res::err_at_1000___/_____loop ... bench:         879 ns/iter (+/- 714)
  test l1res::err_at_1000___/first_err ... bench:         900 ns/iter (+/- 785)

  test l1res::err_at_10000__/__collect ... bench:       24843 ns/iter (+/- 6165)
  test l1res::err_at_10000__/_____loop ... bench:       24066 ns/iter (+/- 8005)
  test l1res::err_at_10000__/first_err ... bench:       10712 ns/iter (+/- 7917)

  test l1res::err_at_99999__/__collect ... bench:      246043 ns/iter (+/- 10544)
  test l1res::err_at_99999__/_____loop ... bench:       86958 ns/iter (+/- 77491)
  test l1res::err_at_99999__/first_err ... bench:       87586 ns/iter (+/- 81596)

  test l1res::err_not_exists/__collect ... bench:      256112 ns/iter (+/- 6170)
  test l1res::err_not_exists/_____loop ... bench:       86596 ns/iter (+/- 60184)
  test l1res::err_not_exists/first_err ... bench:       86703 ns/iter (+/- 71470)

  test l2res::l1_err_at_0_______l2_err_at_1000___/__collect ... bench:          18 ns/iter (+/- 0)
  test l2res::l1_err_at_0_______l2_err_at_1000___/_____loop ... bench:          11 ns/iter (+/- 0)
  test l2res::l1_err_at_0_______l2_err_at_1000___/first_err ... bench:          10 ns/iter (+/- 0)

  test l2res::l1_err_at_10______l2_err_at_1000___/__collect ... bench:         107 ns/iter (+/- 4)
  test l2res::l1_err_at_10______l2_err_at_1000___/_____loop ... bench:          21 ns/iter (+/- 0)
  test l2res::l1_err_at_10______l2_err_at_1000___/first_err ... bench:          20 ns/iter (+/- 0)

  test l2res::l1_err_at_100_____l2_err_at_1000___/__collect ... bench:         379 ns/iter (+/- 8)
  test l2res::l1_err_at_100_____l2_err_at_1000___/_____loop ... bench:         163 ns/iter (+/- 5)
  test l2res::l1_err_at_100_____l2_err_at_1000___/first_err ... bench:         156 ns/iter (+/- 3)

  test l2res::l1_err_at_1000____l2_err_at_1000___/__collect ... bench:        1924 ns/iter (+/- 7)
  test l2res::l1_err_at_1000____l2_err_at_1000___/_____loop ... bench:        1446 ns/iter (+/- 12)
  test l2res::l1_err_at_1000____l2_err_at_1000___/first_err ... bench:        1453 ns/iter (+/- 29)

  test l2res::l1_err_at_10000___l2_err_at_1000___/__collect ... bench:       16222 ns/iter (+/- 555)
  test l2res::l1_err_at_10000___l2_err_at_1000___/_____loop ... bench:       14432 ns/iter (+/- 5748)
  test l2res::l1_err_at_10000___l2_err_at_1000___/first_err ... bench:       14363 ns/iter (+/- 5538)

  test l2res::l1_err_at_99999___l2_err_at_1000___/__collect ... bench:      183241 ns/iter (+/- 2231)
  test l2res::l1_err_at_99999___l2_err_at_1000___/_____loop ... bench:      143188 ns/iter (+/- 48487)
  test l2res::l1_err_at_99999___l2_err_at_1000___/first_err ... bench:      139212 ns/iter (+/- 26714)

  test l2res::l1_err_not_exists_l2_err_at_1000___/__collect ... bench:      180766 ns/iter (+/- 2712)
  test l2res::l1_err_not_exists_l2_err_at_1000___/_____loop ... bench:      143152 ns/iter (+/- 680)
  test l2res::l1_err_not_exists_l2_err_at_1000___/first_err ... bench:      140688 ns/iter (+/- 75539)

  test l2res::l1_err_at_1000____l2_err_at_0______/__collect ... bench:        1921 ns/iter (+/- 22)
  test l2res::l1_err_at_1000____l2_err_at_0______/_____loop ... bench:        1354 ns/iter (+/- 595)
  test l2res::l1_err_at_1000____l2_err_at_0______/first_err ... bench:        1446 ns/iter (+/- 639)

  test l2res::l1_err_at_1000____l2_err_at_10_____/__collect ... bench:        1892 ns/iter (+/- 29)
  test l2res::l1_err_at_1000____l2_err_at_10_____/_____loop ... bench:        1668 ns/iter (+/- 762)
  test l2res::l1_err_at_1000____l2_err_at_10_____/first_err ... bench:        1458 ns/iter (+/- 354)

  test l2res::l1_err_at_1000____l2_err_at_100____/__collect ... bench:        1878 ns/iter (+/- 53)
  test l2res::l1_err_at_1000____l2_err_at_100____/_____loop ... bench:        2784 ns/iter (+/- 537)
  test l2res::l1_err_at_1000____l2_err_at_100____/first_err ... bench:        2948 ns/iter (+/- 715)

  test l2res::l1_err_at_1000____l2_err_at_1000___/__collect #2 ... bench:        1884 ns/iter (+/- 68)
  test l2res::l1_err_at_1000____l2_err_at_1000___/_____loop #2 ... bench:        1516 ns/iter (+/- 59)
  test l2res::l1_err_at_1000____l2_err_at_1000___/first_err #2 ... bench:        1472 ns/iter (+/- 71)

  test l2res::l1_err_at_1000____l2_err_at_10000__/__collect ... bench:        1937 ns/iter (+/- 22)
  test l2res::l1_err_at_1000____l2_err_at_10000__/_____loop ... bench:        1514 ns/iter (+/- 43)
  test l2res::l1_err_at_1000____l2_err_at_10000__/first_err ... bench:        1486 ns/iter (+/- 33)

  test l2res::l1_err_at_1000____l2_err_at_99999__/__collect ... bench:        1912 ns/iter (+/- 36)
  test l2res::l1_err_at_1000____l2_err_at_99999__/_____loop ... bench:        1446 ns/iter (+/- 9)
  test l2res::l1_err_at_1000____l2_err_at_99999__/first_err ... bench:        1454 ns/iter (+/- 45)

  test l2res::l1_err_at_1000____l2_err_not_exists/__collect ... bench:        1936 ns/iter (+/- 52)
  test l2res::l1_err_at_1000____l2_err_not_exists/_____loop ... bench:        1512 ns/iter (+/- 83)
  test l2res::l1_err_at_1000____l2_err_not_exists/first_err ... bench:        1450 ns/iter (+/- 8)

  test l2res::l1_err_not_exists_l2_err_at_0______/__collect ... bench:      181537 ns/iter (+/- 3474)
  test l2res::l1_err_not_exists_l2_err_at_0______/_____loop ... bench:      134772 ns/iter (+/- 39332)
  test l2res::l1_err_not_exists_l2_err_at_0______/first_err ... bench:      142383 ns/iter (+/- 58778)

  test l2res::l1_err_not_exists_l2_err_at_10_____/__collect ... bench:      185324 ns/iter (+/- 14065)
  test l2res::l1_err_not_exists_l2_err_at_10_____/_____loop ... bench:      134675 ns/iter (+/- 39927)
  test l2res::l1_err_not_exists_l2_err_at_10_____/first_err ... bench:      142835 ns/iter (+/- 58236)

  test l2res::l1_err_not_exists_l2_err_at_100____/__collect ... bench:      179775 ns/iter (+/- 3277)
  test l2res::l1_err_not_exists_l2_err_at_100____/_____loop ... bench:      143182 ns/iter (+/- 37429)
  test l2res::l1_err_not_exists_l2_err_at_100____/first_err ... bench:      140019 ns/iter (+/- 1241)

  test l2res::l1_err_not_exists_l2_err_at_1000___/__collect #2 ... bench:      180570 ns/iter (+/- 680)
  test l2res::l1_err_not_exists_l2_err_at_1000___/_____loop #2 ... bench:      143217 ns/iter (+/- 2567)
  test l2res::l1_err_not_exists_l2_err_at_1000___/first_err #2 ... bench:      140436 ns/iter (+/- 63962)

  test l2res::l1_err_not_exists_l2_err_at_10000__/__collect ... bench:      184348 ns/iter (+/- 4317)
  test l2res::l1_err_not_exists_l2_err_at_10000__/_____loop ... bench:      143244 ns/iter (+/- 241)
  test l2res::l1_err_not_exists_l2_err_at_10000__/first_err ... bench:      141813 ns/iter (+/- 15499)

  test l2res::l1_err_not_exists_l2_err_at_99999__/__collect ... bench:      260050 ns/iter (+/- 3608)
  test l2res::l1_err_not_exists_l2_err_at_99999__/_____loop ... bench:      143346 ns/iter (+/- 4882)
  test l2res::l1_err_not_exists_l2_err_at_99999__/first_err ... bench:      143413 ns/iter (+/- 3107)

  test l2res::l1_err_at_0_______l2_err_not_exists/__collect ... bench:          18 ns/iter (+/- 0)
  test l2res::l1_err_at_0_______l2_err_not_exists/_____loop ... bench:          11 ns/iter (+/- 0)
  test l2res::l1_err_at_0_______l2_err_not_exists/first_err ... bench:          10 ns/iter (+/- 0)

  test l2res::l1_err_at_10______l2_err_not_exists/__collect ... bench:         129 ns/iter (+/- 2)
  test l2res::l1_err_at_10______l2_err_not_exists/_____loop ... bench:          20 ns/iter (+/- 0)
  test l2res::l1_err_at_10______l2_err_not_exists/first_err ... bench:          20 ns/iter (+/- 0)

  test l2res::l1_err_at_100_____l2_err_not_exists/__collect ... bench:         413 ns/iter (+/- 2)
  test l2res::l1_err_at_100_____l2_err_not_exists/_____loop ... bench:         163 ns/iter (+/- 3)
  test l2res::l1_err_at_100_____l2_err_not_exists/first_err ... bench:         154 ns/iter (+/- 1)

  test l2res::l1_err_at_1000____l2_err_not_exists/__collect #2 ... bench:        1914 ns/iter (+/- 18)
  test l2res::l1_err_at_1000____l2_err_not_exists/_____loop #2 ... bench:        1444 ns/iter (+/- 71)
  test l2res::l1_err_at_1000____l2_err_not_exists/first_err #2 ... bench:        1484 ns/iter (+/- 20)

  test l2res::l1_err_at_10000___l2_err_not_exists/__collect ... bench:       16071 ns/iter (+/- 188)
  test l2res::l1_err_at_10000___l2_err_not_exists/_____loop ... bench:       14350 ns/iter (+/- 389)
  test l2res::l1_err_at_10000___l2_err_not_exists/first_err ... bench:       14355 ns/iter (+/- 24)

  test l2res::l1_err_at_99999___l2_err_not_exists/__collect ... bench:      179823 ns/iter (+/- 530)
  test l2res::l1_err_at_99999___l2_err_not_exists/_____loop ... bench:      143386 ns/iter (+/- 2464)
  test l2res::l1_err_at_99999___l2_err_not_exists/first_err ... bench:      143362 ns/iter (+/- 117)

  test l2res::l1_err_not_exists_l2_err_not_exists/__collect ... bench:      269990 ns/iter (+/- 5142)
  test l2res::l1_err_not_exists_l2_err_not_exists/_____loop ... bench:      143353 ns/iter (+/- 12659)
  test l2res::l1_err_not_exists_l2_err_not_exists/first_err ... bench:      143382 ns/iter (+/- 150)

  test l1opt::none_at_0______/__collect ... bench:          11 ns/iter (+/- 0)
  test l1opt::none_at_0______/_____loop ... bench:           1 ns/iter (+/- 0)
  test l1opt::none_at_0______/first_err ... bench:           2 ns/iter (+/- 0)

  test l1opt::none_at_10_____/__collect ... bench:         112 ns/iter (+/- 1)
  test l1opt::none_at_10_____/_____loop ... bench:          10 ns/iter (+/- 3)
  test l1opt::none_at_10_____/first_err ... bench:          25 ns/iter (+/- 6)

  test l1opt::none_at_100____/__collect ... bench:         364 ns/iter (+/- 3)
  test l1opt::none_at_100____/_____loop ... bench:         263 ns/iter (+/- 70)
  test l1opt::none_at_100____/first_err ... bench:          93 ns/iter (+/- 39)

  test l1opt::none_at_1000___/__collect ... bench:        1566 ns/iter (+/- 216)
  test l1opt::none_at_1000___/_____loop ... bench:        1927 ns/iter (+/- 772)
  test l1opt::none_at_1000___/first_err ... bench:        1280 ns/iter (+/- 791)

  test l1opt::none_at_10000__/__collect ... bench:       11772 ns/iter (+/- 5020)
  test l1opt::none_at_10000__/_____loop ... bench:        8357 ns/iter (+/- 3201)
  test l1opt::none_at_10000__/first_err ... bench:        7937 ns/iter (+/- 3679)

  test l1opt::none_at_99999__/__collect ... bench:      263004 ns/iter (+/- 15409)
  test l1opt::none_at_99999__/_____loop ... bench:      239036 ns/iter (+/- 78042)
  test l1opt::none_at_99999__/first_err ... bench:      242659 ns/iter (+/- 132)

  test l1opt::none_not_exists/__collect ... bench:      273014 ns/iter (+/- 28528)
  test l1opt::none_not_exists/_____loop ... bench:       83924 ns/iter (+/- 32051)
  test l1opt::none_not_exists/first_err ... bench:       79956 ns/iter (+/- 75761)

  test l2opt::l1_none_at_0_______l2_none_at_1000___/__collect ... bench:          18 ns/iter (+/- 0)
  test l2opt::l1_none_at_0_______l2_none_at_1000___/_____loop ... bench:          11 ns/iter (+/- 0)
  test l2opt::l1_none_at_0_______l2_none_at_1000___/first_err ... bench:          10 ns/iter (+/- 0)

  test l2opt::l1_none_at_10______l2_none_at_1000___/__collect ... bench:         120 ns/iter (+/- 1)
  test l2opt::l1_none_at_10______l2_none_at_1000___/_____loop ... bench:          20 ns/iter (+/- 0)
  test l2opt::l1_none_at_10______l2_none_at_1000___/first_err ... bench:          20 ns/iter (+/- 0)

  test l2opt::l1_none_at_100_____l2_none_at_1000___/__collect ... bench:         359 ns/iter (+/- 18)
  test l2opt::l1_none_at_100_____l2_none_at_1000___/_____loop ... bench:         154 ns/iter (+/- 0)
  test l2opt::l1_none_at_100_____l2_none_at_1000___/first_err ... bench:         154 ns/iter (+/- 1)

  test l2opt::l1_none_at_1000____l2_none_at_1000___/__collect ... bench:        1858 ns/iter (+/- 19)
  test l2opt::l1_none_at_1000____l2_none_at_1000___/_____loop ... bench:        1444 ns/iter (+/- 2)
  test l2opt::l1_none_at_1000____l2_none_at_1000___/first_err ... bench:        1460 ns/iter (+/- 87)

  test l2opt::l1_none_at_10000___l2_none_at_1000___/__collect ... bench:       16062 ns/iter (+/- 256)
  test l2opt::l1_none_at_10000___l2_none_at_1000___/_____loop ... bench:       14367 ns/iter (+/- 108)
  test l2opt::l1_none_at_10000___l2_none_at_1000___/first_err ... bench:       14340 ns/iter (+/- 155)

  test l2opt::l1_none_at_99999___l2_none_at_1000___/__collect ... bench:      180597 ns/iter (+/- 8952)
  test l2opt::l1_none_at_99999___l2_none_at_1000___/_____loop ... bench:      143304 ns/iter (+/- 58020)
  test l2opt::l1_none_at_99999___l2_none_at_1000___/first_err ... bench:      293926 ns/iter (+/- 82793)

  test l2opt::l1_none_not_exists_l2_none_at_1000___/__collect ... bench:      182543 ns/iter (+/- 4375)
  test l2opt::l1_none_not_exists_l2_none_at_1000___/_____loop ... bench:      143175 ns/iter (+/- 46427)
  test l2opt::l1_none_not_exists_l2_none_at_1000___/first_err ... bench:      140369 ns/iter (+/- 1080)

  test l2opt::l1_none_at_1000____l2_none_at_0______/__collect ... bench:        1940 ns/iter (+/- 53)
  test l2opt::l1_none_at_1000____l2_none_at_0______/_____loop ... bench:        1357 ns/iter (+/- 14)
  test l2opt::l1_none_at_1000____l2_none_at_0______/first_err ... bench:        1452 ns/iter (+/- 707)

  test l2opt::l1_none_at_1000____l2_none_at_10_____/__collect ... bench:        1920 ns/iter (+/- 90)
  test l2opt::l1_none_at_1000____l2_none_at_10_____/_____loop ... bench:        1358 ns/iter (+/- 675)
  test l2opt::l1_none_at_1000____l2_none_at_10_____/first_err ... bench:        1453 ns/iter (+/- 50)

  test l2opt::l1_none_at_1000____l2_none_at_100____/__collect ... bench:        1942 ns/iter (+/- 34)
  test l2opt::l1_none_at_1000____l2_none_at_100____/_____loop ... bench:        1468 ns/iter (+/- 408)
  test l2opt::l1_none_at_1000____l2_none_at_100____/first_err ... bench:        2951 ns/iter (+/- 663)

  test l2opt::l1_none_at_1000____l2_none_at_1000___/__collect #2 ... bench:        1949 ns/iter (+/- 57)
  test l2opt::l1_none_at_1000____l2_none_at_1000___/_____loop #2 ... bench:        1462 ns/iter (+/- 39)
  test l2opt::l1_none_at_1000____l2_none_at_1000___/first_err #2 ... bench:        1451 ns/iter (+/- 14)

  test l2opt::l1_none_at_1000____l2_none_at_10000__/__collect ... bench:        1954 ns/iter (+/- 34)
  test l2opt::l1_none_at_1000____l2_none_at_10000__/_____loop ... bench:        1449 ns/iter (+/- 33)
  test l2opt::l1_none_at_1000____l2_none_at_10000__/first_err ... bench:        1454 ns/iter (+/- 114)

  test l2opt::l1_none_at_1000____l2_none_at_99999__/__collect ... bench:        1947 ns/iter (+/- 17)
  test l2opt::l1_none_at_1000____l2_none_at_99999__/_____loop ... bench:        1510 ns/iter (+/- 57)
  test l2opt::l1_none_at_1000____l2_none_at_99999__/first_err ... bench:        1454 ns/iter (+/- 51)

  test l2opt::l1_none_at_1000____l2_none_not_exists/__collect ... bench:        1915 ns/iter (+/- 53)
  test l2opt::l1_none_at_1000____l2_none_not_exists/_____loop ... bench:        1451 ns/iter (+/- 49)
  test l2opt::l1_none_at_1000____l2_none_not_exists/first_err ... bench:        1460 ns/iter (+/- 99)

  test l2opt::l1_none_not_exists_l2_none_at_0______/__collect ... bench:      180624 ns/iter (+/- 1317)
  test l2opt::l1_none_not_exists_l2_none_at_0______/_____loop ... bench:      135205 ns/iter (+/- 53407)
  test l2opt::l1_none_not_exists_l2_none_at_0______/first_err ... bench:      142770 ns/iter (+/- 77869)

  test l2opt::l1_none_not_exists_l2_none_at_10_____/__collect ... bench:      178635 ns/iter (+/- 2068)
  test l2opt::l1_none_not_exists_l2_none_at_10_____/_____loop ... bench:      134618 ns/iter (+/- 326)
  test l2opt::l1_none_not_exists_l2_none_at_10_____/first_err ... bench:      142680 ns/iter (+/- 1325)

  test l2opt::l1_none_not_exists_l2_none_at_100____/__collect ... bench:      180881 ns/iter (+/- 4920)
  test l2opt::l1_none_not_exists_l2_none_at_100____/_____loop ... bench:      144097 ns/iter (+/- 66716)
  test l2opt::l1_none_not_exists_l2_none_at_100____/first_err ... bench:      139882 ns/iter (+/- 47413)

  test l2opt::l1_none_not_exists_l2_none_at_1000___/__collect #2 ... bench:      182234 ns/iter (+/- 1207)
  test l2opt::l1_none_not_exists_l2_none_at_1000___/_____loop #2 ... bench:      143290 ns/iter (+/- 277)
  test l2opt::l1_none_not_exists_l2_none_at_1000___/first_err #2 ... bench:      140271 ns/iter (+/- 2245)

  test l2opt::l1_none_not_exists_l2_none_at_10000__/__collect ... bench:      183328 ns/iter (+/- 2854)
  test l2opt::l1_none_not_exists_l2_none_at_10000__/_____loop ... bench:      143274 ns/iter (+/- 122)
  test l2opt::l1_none_not_exists_l2_none_at_10000__/first_err ... bench:      141344 ns/iter (+/- 1163)

  test l2opt::l1_none_not_exists_l2_none_at_99999__/__collect ... bench:      255029 ns/iter (+/- 4666)
  test l2opt::l1_none_not_exists_l2_none_at_99999__/_____loop ... bench:      143897 ns/iter (+/- 642)
  test l2opt::l1_none_not_exists_l2_none_at_99999__/first_err ... bench:      143559 ns/iter (+/- 1435)

  test l2opt::l1_none_at_0_______l2_none_not_exists/__collect ... bench:          18 ns/iter (+/- 0)
  test l2opt::l1_none_at_0_______l2_none_not_exists/_____loop ... bench:          11 ns/iter (+/- 0)
  test l2opt::l1_none_at_0_______l2_none_not_exists/first_err ... bench:          10 ns/iter (+/- 0)

  test l2opt::l1_none_at_10______l2_none_not_exists/__collect ... bench:         123 ns/iter (+/- 2)
  test l2opt::l1_none_at_10______l2_none_not_exists/_____loop ... bench:          20 ns/iter (+/- 0)
  test l2opt::l1_none_at_10______l2_none_not_exists/first_err ... bench:          21 ns/iter (+/- 0)

  test l2opt::l1_none_at_100_____l2_none_not_exists/__collect ... bench:         383 ns/iter (+/- 22)
  test l2opt::l1_none_at_100_____l2_none_not_exists/_____loop ... bench:         155 ns/iter (+/- 2)
  test l2opt::l1_none_at_100_____l2_none_not_exists/first_err ... bench:         154 ns/iter (+/- 4)

  test l2opt::l1_none_at_1000____l2_none_not_exists/__collect #2 ... bench:        1926 ns/iter (+/- 59)
  test l2opt::l1_none_at_1000____l2_none_not_exists/_____loop #2 ... bench:        1445 ns/iter (+/- 50)
  test l2opt::l1_none_at_1000____l2_none_not_exists/first_err #2 ... bench:        1450 ns/iter (+/- 12)

  test l2opt::l1_none_at_10000___l2_none_not_exists/__collect ... bench:       16122 ns/iter (+/- 144)
  test l2opt::l1_none_at_10000___l2_none_not_exists/_____loop ... bench:       14369 ns/iter (+/- 185)
  test l2opt::l1_none_at_10000___l2_none_not_exists/first_err ... bench:       14423 ns/iter (+/- 272)

  test l2opt::l1_none_at_99999___l2_none_not_exists/__collect ... bench:      179766 ns/iter (+/- 1038)
  test l2opt::l1_none_at_99999___l2_none_not_exists/_____loop ... bench:      143399 ns/iter (+/- 762)
  test l2opt::l1_none_at_99999___l2_none_not_exists/first_err ... bench:      143623 ns/iter (+/- 4965)

  test l2opt::l1_none_not_exists_l2_none_not_exists/__collect ... bench:      274073 ns/iter (+/- 4701)
  test l2opt::l1_none_not_exists_l2_none_not_exists/_____loop ... bench:      143589 ns/iter (+/- 3224)
  test l2opt::l1_none_not_exists_l2_none_not_exists/first_err ... bench:      143612 ns/iter (+/- 302)
  ```

  </p>
</details>



## Licence

MIT
