# CHANGELOG

## Unreleased

## v0.2.1 - 2023-11-09

- performance: impl `size_hint()` for  `FirstErrIter` and `FirstNoneIter`.



## v0.2.0 - 2023-10-28

- BREAKING: add trait bound `Iterator` on `FirstErr` trait.
- new: add following new methods to `FirstErr`:
    - `first_err_or_try()`
    - `first_none_or_else()`
    - `first_none_or_try()`
    - `first_none_or()`
- performance: Impl `FusedIterator` for `FirstErrIter`.
- performance: various performance tweak.
- bench: improve benchmarking code.



## v0.1.1 - 2023-10-22

- Fix some document error.



## v0.1.0 - 2023-10-22

- First release.
