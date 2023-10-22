//! # `first-err`
//!
//! Find first `Err` in `Iterator<Result<T, E>>` and allow to iterating continuously.
//!
//! This crate is specifically designed to replace the following pattern without allocation:
//!
//! ```rust
//! // iter: impl Iterator<Result<T, E>>
//! iter.collect::<Result<Vec<T>, E>>().map(|vec| vec.into_iter().foo() );
//! ```
//!
//!
//!
//! ## Features
//!
//! - Find first `Err` in `Iterator<Result<T, E>>` and allow to iterating continuously.
//! - Speed: rough on par with hand write loop, use lazy evaluation and without alloc.
//! - Minimized: `no_std`, no `alloc`, no dependency.
//!
//!
//!
//! ## Getting Started
//!
//! This crate help you to take first `Err` in a [`Result`] and keep iterating without
//! pay for allocation, here is a sample:
//!
//! ```rust
//! use first_err::FirstErr;
//!
//! # fn main() {
//! // Everything is Ok.
//! let ans = [Ok::<u8, u8>(0), Ok(1), Ok(2)]
//!     .into_iter()
//!     .first_err_or_else(|iter| iter.sum::<u8>());
//! assert_eq!(ans, Ok(3));
//!
//! // Contains some `Err` values.
//! let ans = [Ok::<u8, u8>(0), Err(1), Err(2)]
//!     .into_iter()
//!     .first_err_or_else(|iter| iter.sum::<u8>());
//! assert_eq!(ans, Err(1));
//! # }
//! ```
//!
//! See [`FirstErr::first_err_or_else()`] for more detail.
//!
//!
//!
//! ## Why
//!
//! In Rust, I always encountered a kind of pattern which is I need to do something on all
//! items within an iterator, and should also cancel as soon as possible if any error is
//! found in current working layer. But, if no error found, the iterator should able to run
//! continuously and allow me to do further transform.
//!
//! The pattern typically looks as follows:
//!
//! ```rust
//! # fn main() {
//! let array: [Result<u8, u8>; 3] = [Ok(0), Err(1), Err(2)];
//!
//! fn fallible_sum(iter: impl IntoIterator<Item = Result<u8, u8>>) -> Result<u8, u8> {
//!     let sum = iter
//!         .into_iter()
//!         .collect::<Result<Vec<_>, _>>()?    // early return (and a vec alloc in here)
//!         .into_iter()                        // continue iterate next layer ...
//!         .sum();
//!
//!     Ok(sum)
//! }
//!
//! let ans = fallible_sum(array);
//! assert_eq!(ans, Err(1));
//! # }
//! ```
//!
//! In theory, this allocation is not necessary. We can just write that code as an old good
//! loop:
//!
//! ```rust
//! # fn main() {
//! let array: [Result<u8, u8>; 3] = [Ok(0), Err(1), Err(2)];
//!
//! fn fallible_sum(iter: impl IntoIterator<Item = Result<u8, u8>>) -> Result<u8, u8> {
//!     let mut sum = 0;
//!     for res in iter {
//!         let val = res?;                     // early return, no alloc
//!         sum += val;
//!     }
//!
//!     Ok(sum)
//! }
//!
//! let ans = fallible_sum(array);
//! assert_eq!(ans, Err(1))
//! # }
//! ```
//!
//! Using a loop is not bad at all. But for some situation, I would like to keep iterator
//! chainable as much as possible. This crate offers another approach to achieve it.
//!
//! And even further, sometime life may not simple like previous example. consider is one:
//!
//! ```rust
//! # fn main() {
//! // The second layer `Result` is usually created by further transform after the first layer
//! // `Result` be processed. But for the sake of simplicity, we've just use pre-defined values.
//! let array: [Result<Result<u8, u8>, u8>; 3] = [Ok(Ok(0)), Ok(Err(1)), Err(2)];
//!
//! fn fallible_sum(
//!     iter: impl IntoIterator<Item = Result<Result<u8, u8>, u8>>
//! ) -> Result<u8, u8> {
//!     // take "first `Err`" layer by layer, or the sum value.
//!     let sum = iter
//!         .into_iter()
//!         .collect::<Result<Vec<Result<u8, u8>>, u8>>()?
//!         .into_iter()
//!         .collect::<Result<Vec<u8>, u8>>()?
//!         .into_iter()
//!         .sum();
//!
//!     Ok(sum)
//! }
//!
//! let ans = fallible_sum(array);
//! assert_eq!(ans, Err(2));
//! # }
//! ```
//!
//! Above logic may little hard to write as a loop without alloc. But this crate can do it
//! for you:
//!
//! ```rust
//! # use first_err::FirstErr;
//! #
//! # fn main() {
//! let array: [Result<Result<u8, u8>, u8>; 3] = [Ok(Ok(0)), Ok(Err(1)), Err(2)];
//!
//! fn fallible_sum(
//!     iter: impl IntoIterator<Item = Result<Result<u8, u8>, u8>>
//! ) -> Result<u8, u8> {
//!     iter
//!         .into_iter()
//!         .first_err_or_else(|iter1| { // iter1 = impl Iterator<Item = Result<u8, u8>>
//!             iter1.first_err_or_else(|iter2| { // iter2 = impl Iterator<Item = u8>
//!                 iter2.sum::<u8>()
//!             })
//!         })
//!         .and_then(|res_res| res_res)
//! }
//!
//! let ans = fallible_sum(array);
//! assert_eq!(ans, Err(2));
//! # }
//! ```
//!
//!
//!
//! ## Benchmark
//!
//! This crate's performance character is designed for rough on par with hand write loop.
//! But compiler may do some better optimization for one or another in difference situations.
//!
//! If you want do benchmark by yourself, use follows command:
//!
//! ```sh
//! cargo bench --bench benchmark -- --output-format bencher
//! ```
//!
//! And don't forget check which code I actual bench in `benches` folder.

#![no_std]

/// Iterator can take first error from inner iterator.
///
/// See [`FirstErr::first_err_or_else()`] for more detail.
#[derive(Debug)]
pub struct FirstErrIter<I, T, E>
where
    I: Iterator<Item = Result<T, E>>,
{
    /// Internal iterator.
    inner: I,

    /// The first `Err` when iterating `inner`.
    first_err: Option<E>,
}

impl<I, T, E> FirstErrIter<I, T, E>
where
    I: Iterator<Item = Result<T, E>>,
{
    fn consume_until_first_err(mut self) -> Option<E> {
        if self.first_err.is_none() {
            // try to found an error, or just run through the whole iterator.
            for _ in &mut self {}
        }

        self.first_err.take()
    }
}

impl<I, T, E> Iterator for FirstErrIter<I, T, E>
where
    I: Iterator<Item = Result<T, E>>,
{
    type Item = T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.first_err.is_some() {
            return None;
        }

        match self.inner.next() {
            // ok value
            Some(Ok(t)) => Some(t),

            // find first Err
            Some(Err(e)) => {
                self.first_err = Some(e);
                None
            }

            // exhausted
            None => None,
        }
    }
}

/// This trait provides `first_err_or_else()` method on all `Iterator<Item = Result<T, E>>`.
pub trait FirstErr<I, T, E> {
    /// Return the first `Err` item in current iterator or an `Ok` value return by `f` closure.
    /// If no error found, this method will consume all items before return.
    ///
    /// The iterator argument of closure produce the same sequence but stop from first `Err` item.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use first_err::FirstErr;
    ///
    /// # fn main() {
    /// // Everything is Ok.
    /// let ans = [Ok::<u8, u8>(0), Ok(1), Ok(2)]
    ///     .into_iter()
    ///     .first_err_or_else(|iter| iter.sum::<u8>());
    /// assert_eq!(ans, Ok(3));
    ///
    /// // Contains some `Err` values.
    /// let ans = [Ok::<u8, u8>(0), Err(1), Err(2)]
    ///     .into_iter()
    ///     .first_err_or_else(|iter| iter.sum::<u8>());
    /// assert_eq!(ans, Err(1));
    /// # }
    /// ```
    ///
    /// # Guarantees
    ///
    /// ## Not need to consume inner iterator manually:
    ///
    /// ```rust
    /// # use first_err::FirstErr;
    /// #
    /// # fn main() {
    /// let ans = [Ok::<u8, u8>(0), Err(1), Err(2)]
    ///     .into_iter()
    ///     .first_err_or_else(|_iter| {}); // not need to consume `_iter` iterator,
    /// assert_eq!(ans, Err(1));            // and the result still correct.
    /// # }
    /// ```
    ///
    /// ## Outer iterator will be evaluated lazily:
    ///
    /// ```rust
    /// # use first_err::FirstErr;
    /// # use std::cell::RefCell;
    /// #
    /// # fn main() {
    /// let mut vec = RefCell::new(vec![]);
    ///
    /// let mut ans = [Ok::<u8, u8>(0), Ok(1), Err(2), Ok(3)]
    ///     .into_iter()
    ///     .inspect(|res| { vec.borrow_mut().push(*res) })         // push value from outer
    ///     .first_err_or_else(|iter| {
    ///         iter
    ///             .inspect(|n| { vec.borrow_mut().push(Ok(42)) }) // push value from inner
    ///             .sum::<u8>()
    ///     });
    ///
    /// assert_eq!(ans, Err(2));
    /// assert_eq!(
    ///     vec.into_inner(),
    ///     vec![Ok(0), Ok(42), Ok(1), Ok(42), Err(2)],
    /// );
    /// # }
    /// ```
    ///
    /// ## Caller can't leak the inner iterator out from `f` closure:
    ///
    /// ```rust,compile_fail
    /// # use first_err::FirstErr;
    /// #
    /// # fn main() {
    /// let iter = [Ok::<u8, u8>(0), Err(1), Err(2)]
    ///     .into_iter()
    ///     .first_err_or_else(|iter| iter); // compile error: can't leak `iter` out
    /// # }
    /// ```
    fn first_err_or_else<F, O>(self, f: F) -> Result<O, E>
    where
        F: FnOnce(&mut FirstErrIter<Self, T, E>) -> O;

    /// Return the first `Err` item in current iterator or an `Ok` value. If no error found,
    /// this method will consume all items before return.
    ///
    /// This method is a shorter version of [`first_err_or_else(|_| value)`](Self::first_err_or_else).
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use first_err::FirstErr;
    /// #
    /// # fn main() {
    /// // Everything is Ok.
    /// let ans = [Ok::<u8, u8>(0), Ok(1), Ok(2)]
    ///     .into_iter()
    ///     .first_err_or("foo");
    /// assert_eq!(ans, Ok("foo"));
    ///
    /// // Contains some `Err` values.
    /// let ans = [Ok::<u8, u8>(0), Err(1), Err(2)]
    ///     .into_iter()
    ///     .first_err_or("foo");
    /// assert_eq!(ans, Err(1));
    /// # }
    /// ```
    fn first_err_or<O>(self, value: O) -> Result<O, E>;
}

impl<I, T, E> FirstErr<I, T, E> for I
where
    I: Iterator<Item = Result<T, E>>,
{
    #[inline]
    fn first_err_or_else<F, O>(self, f: F) -> Result<O, E>
    where
        F: FnOnce(&mut FirstErrIter<Self, T, E>) -> O,
    {
        let mut first_err_iter = FirstErrIter {
            inner: self,
            first_err: None,
        };

        let output = f(&mut first_err_iter);

        // Take the `first_err` back if err exists in whole iterator.
        match first_err_iter.consume_until_first_err() {
            Some(e) => Err(e),
            None => Ok(output),
        }
    }

    #[inline]
    fn first_err_or<O>(self, value: O) -> Result<O, E> {
        self.first_err_or_else(|_| value)
    }
}

#[cfg(test)]
mod tests {
    use super::FirstErr;

    #[test]
    fn test_first_err_or_else_with_1_layer_data_and_without_err() {
        let ans = [Ok::<u8, u8>(0), Ok(1), Ok(2), Ok(3), Ok(4)]
            .into_iter()
            .first_err_or_else(|iter1| iter1.sum::<u8>());

        assert_eq!(ans, Ok(10));
    }

    #[test]
    fn test_first_err_or_else_with_1_layer_data_and_with_err() {
        let ans = [Ok::<u8, u8>(0), Ok(1), Err(2), Ok(3), Ok(4)]
            .into_iter()
            .first_err_or_else(|iter1| iter1.sum::<u8>());

        assert_eq!(ans, Err(2));
    }

    #[test]
    fn test_first_err_or_else_with_2_layer_data_and_outmost_err_in_layer_1() {
        let ans = [
            Ok::<Result<u8, u8>, Result<u8, u8>>(Ok::<u8, u8>(0)),
            Ok(Err(1)),
            Err(Ok(2)),
            Ok(Ok(3)),
            Ok(Ok(4)),
        ]
        .into_iter()
        .first_err_or_else(|iter1| {
            iter1
                .map(|x| x) // could chain other ops
                .first_err_or_else(|iter2| iter2.sum::<u8>())
        });

        assert_eq!(ans, Err(Ok(2)));
    }

    #[test]
    fn test_first_err_or_else_with_2_layer_data_and_outmost_err_in_layer_2() {
        let ans = [
            Ok::<Result<u8, u8>, Result<u8, u8>>(Ok::<u8, u8>(0)),
            Ok(Ok(1)),
            Ok(Err(2)),
            Ok(Err(3)),
            Ok(Ok(4)),
        ]
        .into_iter()
        .first_err_or_else(|iter1| {
            iter1
                .map(|x| x) // could chain other ops
                .first_err_or_else(|iter2| iter2.sum::<u8>())
        });

        assert_eq!(ans, Ok(Err(2)));
    }

    #[test]
    fn test_first_err_or_else_with_3_layer_data_and_outmost_err_in_layer_2() {
        let ans = [
            Ok::<Result<Result<u8, u8>, Result<u8, u8>>, Result<Result<u8, u8>, Result<u8, u8>>>(
                Ok(Ok(0)),
            ),
            Ok(Ok(Ok(1))),
            Ok(Ok(Err(2))),
            Ok(Err(Ok(3))),
            Ok(Ok(Ok(4))),
        ]
        .into_iter()
        .first_err_or_else(|iter1| {
            iter1
                .map(|x| x) // could chain other ops
                .first_err_or_else(|iter2| {
                    iter2
                        .map(|x| x) // could chain other ops
                        .first_err_or_else(|iter3| iter3.sum::<u8>())
                })
        });

        assert_eq!(ans, Ok(Err(Ok(3))));
    }

    #[test]
    fn test_first_err_or_else_not_need_to_consume_iter_manually() {
        let ans = [Ok::<u8, u8>(0), Err(1), Err(2)]
            .into_iter()
            .first_err_or_else(|_iter| {});

        assert_eq!(ans, Err(1));
    }

    /// For the most cases, API user should not notice the inner iterator's `.next()`
    /// function be called how many times due to it's already consumed. But if inner
    /// iterator has some side-effect, the behavior still observable, and user maybe
    /// rely with it.
    ///
    /// So here is a test to make sure the behavior keep the same when code changed.
    #[test]
    fn test_first_err_or_else_not_call_next_on_inner_iter_after_first_err() {
        let mut inner_next_count = 0;

        [Ok::<u8, u8>(0), Err(1), Err(2)]
            .into_iter()
            .inspect(|_| inner_next_count += 1) // side-effect
            .first_err_or_else(|iter| {
                // exhaust whole iter.
                for _ in &mut *iter {}

                // call iter.next() after the iter already exhausted.
                assert_eq!(iter.next(), None);
            })
            .ok();

        assert_eq!(inner_next_count, 2);
    }

    #[test]
    fn test_first_err_or_else_use_lazy_evaluation() {
        use core::cell::{Cell, RefCell};

        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum Trace {
            None,
            Outer(Result<u8, u8>),
            Inner(u8),
        }

        // if index >= N, it will panic.
        fn record_trace<const N: usize>(traces: &RefCell<[Trace; N]>, idx: &Cell<usize>, v: Trace) {
            let i = idx.get();
            traces.borrow_mut()[i] = v;
            idx.set(i + 1);
        }

        // already known N = 5 within [_; N] in this test case.
        // We don't use Vec here just bacause want to avoid `alloc` crate.
        let traces = RefCell::new([Trace::None; 5]);

        let index = Cell::new(0);

        let ans = [Ok::<u8, u8>(0), Ok(1), Err(2), Ok(3)]
            .iter()
            .cloned()
            // record value from outer
            .inspect(|&res| record_trace(&traces, &index, Trace::Outer(res)))
            .first_err_or_else(|iter| {
                iter
                    // record value from inner
                    .inspect(|&n| record_trace(&traces, &index, Trace::Inner(n)))
                    .sum::<u8>()
            });

        assert_eq!(ans, Err(2));
        assert_eq!(
            traces.into_inner(),
            [
                Trace::Outer(Ok(0)),
                Trace::Inner(0),
                Trace::Outer(Ok(1)),
                Trace::Inner(1),
                Trace::Outer(Err(2))
            ]
        );
    }
}
