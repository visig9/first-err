//! # `first-err`
//!
//! Find the first `Err` in `Iterator<Result<T, E>>` and allow continuous iteration.
//!
//! This crate is specifically designed to replace the following pattern without allocation:
//!
//! ```txt
//! // iter: impl Iterator<Result<T, E>>
//! iter.collect::<Result<Vec<T>, E>>().map(|vec| vec.into_iter().foo() );
//! ```
//!
//! See [`FirstErr`] trait for more detail.
//!
//!
//!
//! ## Features
//!
//! - Find first `Err` in `Iterator<Result<T, E>>` and allow to iterating continuously.
//! - Speed: Roughly on par with a hand-written loop, using lazy evaluation and no allocation.
//! - Minimized: no `std`, no `alloc`, no dependency.
//!
//!
//!
//! ## Getting Started
//!
//! ```rust
//! // Use this trait in current scope.
//! use first_err::FirstErr;
//!
//! # fn main() {
//! // Everything is Ok.
//! let result = [Ok::<u8, u8>(0), Ok(1), Ok(2)]
//!     .into_iter()
//!     .first_err_or_else(|iter| iter.sum::<u8>());
//! assert_eq!(result, Ok(3));
//!
//! // Contains some `Err` values.
//! let result = [Ok::<u8, u8>(0), Err(1), Err(2)]
//!     .into_iter()
//!     .first_err_or_else(|iter| iter.sum::<u8>());
//! assert_eq!(result, Err(1));
//! # }
//! ```
//!
//!
//!
//! ## Why
//!
//! In Rust, I frequently encounter a pattern where I need to perform actions on all
//! items within an iterator, and halt immediately if any error is detected in the layer
//! I'm working on. But if no error found, the iterator should able to run continuously
//! and allow me to do further transform.
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
//! let result = fallible_sum(array);
//! assert_eq!(result, Err(1));
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
//! let result = fallible_sum(array);
//! assert_eq!(result, Err(1))
//! # }
//! ```
//!
//! Using a loop is not bad at all. However, in some situations, maintaining a chainable
//! iterator is preferable.
//!
//! Furthermore, some scenarios may not be as simple as the previous example. Consider this one:
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
//! let result = fallible_sum(array);
//! assert_eq!(result, Err(2));
//! # }
//! ```
//!
//! Implementing the above logic in a loop without allocation may be error-prone and complicated.
//! This crate simplifies that for you:
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
//! let result = fallible_sum(array);
//! assert_eq!(result, Err(2));
//! # }
//! ```
//!
//!
//!
//! ## Benchmark
//!
//! The performance of this crate is designed to be roughly on par with hand-written loops.
//! However, the compiler might apply different optimizations in various situations, and favoring
//! one approach over the others.
//!
//! If you want to to do a benchmark by yourself, use the following command:
//!
//! ```sh
//! cargo bench --bench benchmark -- --output-format bencher
//! ```
//!
//! Also, don't forget to check the actual code that is used for benchmarking, which is in the
//! `benches` folder.

#![no_std]

pub use option::FirstNoneIter;
pub use result::FirstErrIter;

/// This trait provides some methods on any `Iterator<Item = Result<T, E>>`, which can take
/// the first `Err` in iterators, and without allocation.
///
/// `Iterator<Item = Option<T>>` version with the same logic are also supported.
///
///
///
/// ## Guarantees
///
/// There are some methods in `FirstErr` trait take a closure that accepts an iterator
/// as its argument. This crate guarantees all those methods have the following properties.
///
///
///
/// ### Original Iterator is Evaluated Lazily
///
/// The `.next()` of the original iterator only be called as late as possible, For example,
///
/// ```rust
/// # use first_err::FirstErr;
/// # use std::cell::RefCell;
/// #
/// # fn main() {
/// let mut vec = RefCell::new(vec![]);
///
/// let mut result = [Ok::<u8, u8>(0), Ok(1), Err(2), Ok(3)]
///     .into_iter()
///     .inspect(|res| { vec.borrow_mut().push(*res) })         // push value from outer
///     .first_err_or_else(|iter| {
///         iter
///             .inspect(|n| { vec.borrow_mut().push(Ok(42)) }) // push value from inner
///             .sum::<u8>()
///     });
///
/// assert_eq!(result, Err(2));
/// assert_eq!(
///     vec.into_inner(),
///     vec![Ok(0), Ok(42), Ok(1), Ok(42), Err(2)],
/// );
/// # }
/// ```
///
///
///
/// ### No Need to Manually Consume the Closure's Iterator
///
/// User can simple ignore the iterator in closure partially of fully, and still can get
/// the correct result.
///
/// ```rust
/// # use first_err::FirstErr;
/// #
/// # fn main() {
/// let result = [Ok::<u8, u8>(0), Err(1), Err(2)]
///     .into_iter()
///     .first_err_or_else(|_iter| {}); // not need to consume `_iter` iterator,
/// assert_eq!(result, Err(1));         // and the result still correct.
/// # }
/// ```
///
///
///
/// ### Iterator in Closure Can't be Leaked Out of Closure Scope
///
/// Let the iterator in closure escaped from the closure is a compiler error.
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
pub trait FirstErr: Iterator {
    /// Returns the first `Err` item in the current iterator, or an `Ok` value produced by the
    /// `f` closure.
    ///
    /// The argument iterator of the `f` closure will producing the same values in `Ok` sequence,
    /// but will stop when encounter the first `Err` item.
    ///
    ///
    ///
    /// # Examples
    ///
    /// ```rust
    /// use first_err::FirstErr;
    ///
    /// # fn main() {
    /// // Everything is Ok.
    /// let result = [Ok::<u8, u8>(0), Ok(1), Ok(2)]
    ///     .into_iter()
    ///     .first_err_or_else(|iter| iter.sum::<u8>());
    /// assert_eq!(result, Ok(3));
    ///
    /// // Contains some `Err` values.
    /// let result = [Ok::<u8, u8>(0), Err(1), Err(2)]
    ///     .into_iter()
    ///     .first_err_or_else(|iter| iter.sum::<u8>());
    /// assert_eq!(result, Err(1));
    /// # }
    /// ```
    #[inline]
    fn first_err_or_else<T, E, O, F>(self, f: F) -> Result<O, E>
    where
        F: FnOnce(&mut FirstErrIter<Self, T, E>) -> O,
        Self: Iterator<Item = Result<T, E>> + Sized,
    {
        FirstErrIter::first_err_or_else(self, f)
    }

    /// Returns the first `Err` item in the current iterator, or an `Result` value produced
    /// by the `f` closure.
    ///
    /// The argument iterator of the `f` closure will producing the same values in `Ok` sequence,
    /// but will stop when encounter the first `Err` item.
    ///
    ///
    ///
    /// # Examples
    ///
    /// Basic concept:
    ///
    /// ```rust
    /// use first_err::FirstErr;
    ///
    /// # fn main() {
    /// // Everything is Ok.
    /// let result = [Ok::<u8, u8>(0), Ok(1), Ok(2)]
    ///     .into_iter()
    ///     .first_err_or_try(|_| Ok("ok"));
    /// assert_eq!(result, Ok("ok"));
    ///
    /// // When closure returns Err.
    /// let result = [Ok::<u8, u8>(0), Ok(1), Ok(2)]
    ///     .into_iter()
    ///     .first_err_or_try(|_| Err::<u8, u8>(42));
    /// assert_eq!(result, Err(42));
    ///
    /// // When outer iterator contains Err.
    /// let result = [Ok::<u8, u8>(0), Err(2), Ok(2)]
    ///     .into_iter()
    ///     .first_err_or_try(|_| Ok("ok"));
    /// assert_eq!(result, Err(2));
    ///
    /// // When both contains Err.
    /// let result = [Ok::<u8, u8>(0), Err(1), Ok(2)]
    ///     .into_iter()
    ///     .first_err_or_try(|_| Err::<u8, u8>(42));
    /// assert_eq!(result, Err(1));
    /// # }
    /// ```
    ///
    /// Use the `iter` argument of the `f` closure:
    ///
    /// ```rust
    /// # use first_err::FirstErr;
    /// #
    /// # fn main() {
    /// let admin_id: u32 = 1;
    /// let user_ids_in_conf = ["32", "5", "8", "19"];
    ///
    /// let admin_index = user_ids_in_conf
    ///     .into_iter()
    ///     .map(|s| s.parse::<u32>().map_err(|_| "user id parsing failed"))
    ///     .first_err_or_try(|user_ids_iter| {
    ///         user_ids_iter
    ///             .position(|user_id| user_id == admin_id)
    ///             .ok_or_else(|| "admin not in the user list")
    ///     });
    ///
    /// assert_eq!(admin_index, Err("admin not in the user list"));
    /// # }
    /// ```
    #[inline]
    fn first_err_or_try<T, E, O, F>(self, f: F) -> Result<O, E>
    where
        F: FnOnce(&mut FirstErrIter<Self, T, E>) -> Result<O, E>,
        Self: Iterator<Item = Result<T, E>> + Sized,
    {
        self.first_err_or_else(f).and_then(|res| res)
    }

    /// Returns the first `Err` item in the current iterator, or an `Ok(value)`.
    ///
    ///
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use first_err::FirstErr;
    /// #
    /// # fn main() {
    /// // Everything is Ok.
    /// let result = [Ok::<u8, u8>(0), Ok(1), Ok(2)]
    ///     .into_iter()
    ///     .first_err_or("foo");
    /// assert_eq!(result, Ok("foo"));
    ///
    /// // Contains some `Err` values.
    /// let result = [Ok::<u8, u8>(0), Err(1), Err(2)]
    ///     .into_iter()
    ///     .first_err_or("foo");
    /// assert_eq!(result, Err(1));
    /// # }
    /// ```
    #[inline]
    fn first_err_or<T, E, O>(self, value: O) -> Result<O, E>
    where
        Self: Iterator<Item = Result<T, E>> + Sized,
    {
        self.first_err_or_else(|_| value)
    }

    /// Returns the first `None` item in the current iterator, or an `Some` value produced
    /// by the `f` closure.
    ///
    /// The argument iterator of the `f` closure will producing the same values in `Some` sequence,
    /// but will stop when encounter the first `None` item.
    ///
    ///
    ///
    /// # Examples
    ///
    /// ```rust
    /// use first_err::FirstErr;
    ///
    /// # fn main() {
    /// // Everything is Some.
    /// let option = [Some::<u8>(0), Some(1), Some(2)]
    ///     .into_iter()
    ///     .first_none_or_else(|iter| iter.sum::<u8>());
    /// assert_eq!(option, Some(3));
    ///
    /// // Contains some `None` values.
    /// let option = [Some::<u8>(0), None, None]
    ///     .into_iter()
    ///     .first_none_or_else(|iter| iter.sum::<u8>());
    /// assert_eq!(option, None);
    /// # }
    /// ```
    #[inline]
    fn first_none_or_else<T, O, F>(self, f: F) -> Option<O>
    where
        F: FnOnce(&mut FirstNoneIter<Self, T>) -> O,
        Self: Iterator<Item = Option<T>> + Sized,
    {
        FirstNoneIter::first_none_or_else(self, f)
    }

    /// Returns the first `None` item in the current iterator, or an `Option` value produced
    /// by the `f` closure.
    ///
    /// The argument iterator of the `f` closure will producing the same values in `Some` sequence,
    /// but will stop when encounter the first `None` item.
    ///
    ///
    ///
    /// # Examples
    ///
    /// Basic concept:
    ///
    /// ```rust
    /// use first_err::FirstErr;
    ///
    /// # fn main() {
    /// // Everything is Some.
    /// let option = [Some::<u8>(0), Some(1), Some(2)]
    ///     .into_iter()
    ///     .first_none_or_try(|_| Some("ok"));
    /// assert_eq!(option, Some("ok"));
    ///
    /// // When closure returns None.
    /// let option: Option<&str> = [Some::<u8>(0), Some(1), Some(2)]
    ///     .into_iter()
    ///     .first_none_or_try(|_| None);
    /// assert_eq!(option, None);
    ///
    /// // When outer iterator contains None.
    /// let option = [Some::<u8>(0), None, Some(2)]
    ///     .into_iter()
    ///     .first_none_or_try(|_| Some("ok"));
    /// assert_eq!(option, None);
    ///
    /// // When both contains None.
    /// let option: Option<&str> = [Some::<u8>(0), None, Some(2)]
    ///     .into_iter()
    ///     .first_none_or_try(|_| None);
    /// assert_eq!(option, None);
    /// # }
    /// ```
    ///
    /// Use the `iter` argument of the `f` closure:
    ///
    /// ```rust
    /// # use first_err::FirstErr;
    /// #
    /// # fn main() {
    /// let admin_id: u32 = 1;
    /// let user_ids_in_conf = ["32", "5", "8", "19"];
    ///
    /// let admin_index = user_ids_in_conf
    ///     .into_iter()
    ///     .map(|s| s.parse::<u32>().ok())
    ///     .first_none_or_try(|user_ids_iter| {
    ///         user_ids_iter
    ///             .position(|user_id| user_id == admin_id)
    ///     });
    ///
    /// assert_eq!(admin_index, None);
    /// # }
    /// ```
    #[inline]
    fn first_none_or_try<T, O, F>(self, f: F) -> Option<O>
    where
        F: FnOnce(&mut FirstNoneIter<Self, T>) -> Option<O>,
        Self: Iterator<Item = Option<T>> + Sized,
    {
        self.first_none_or_else(f).and_then(|opt| opt)
    }

    /// Returns the first `None` item in the current iterator, or an `Some(value)`.
    ///
    ///
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use first_err::FirstErr;
    /// #
    /// # fn main() {
    /// // Everything is Some.
    /// let option = [Some::<u8>(0), Some(1), Some(2)]
    ///     .into_iter()
    ///     .first_none_or("foo");
    /// assert_eq!(option, Some("foo"));
    ///
    /// // Contains some `None` values.
    /// let option = [Some::<u8>(0), None, None]
    ///     .into_iter()
    ///     .first_none_or("foo");
    /// assert_eq!(option, None);
    /// # }
    /// ```
    #[inline]
    fn first_none_or<T, O>(self, value: O) -> Option<O>
    where
        Self: Iterator<Item = Option<T>> + Sized,
    {
        self.first_none_or_else(|_| value)
    }
}

impl<I> FirstErr for I where I: Iterator {}

mod result {
    use core::iter::FusedIterator;

    /// An `Iterator` can take first `Err` from another iterator.
    ///
    /// See [`FirstErr::first_err_or_else()`](crate::FirstErr::first_err_or_else) for more details.
    #[derive(Debug)]
    pub struct FirstErrIter<I, T, E>
    where
        I: Iterator<Item = Result<T, E>>,
    {
        state: State<I, T, E>,
    }

    impl<I, T, E> FirstErrIter<I, T, E>
    where
        I: Iterator<Item = Result<T, E>>,
    {
        #[inline]
        pub(super) fn first_err_or_else<O, F>(inner: I, f: F) -> Result<O, E>
        where
            F: FnOnce(&mut Self) -> O,
        {
            let mut me = Self {
                state: State::Active(inner),
            };

            let output = f(&mut me);

            // Take first err, if not found and not exhausted yet, find it.
            // If just not found finally, return output.
            match me.state {
                State::Active(inner) => {
                    for res in inner {
                        let _ = res?;
                    }
                    Ok(output)
                }
                State::Exhausted => Ok(output),
                State::FoundFirstErr(e) => Err(e),
            }
        }
    }

    impl<I, T, E> Iterator for FirstErrIter<I, T, E>
    where
        I: Iterator<Item = Result<T, E>>,
    {
        type Item = T;

        #[inline]
        fn next(&mut self) -> Option<Self::Item> {
            match &mut self.state {
                State::Active(inner) => match inner.next() {
                    Some(Ok(t)) => Some(t),
                    Some(Err(e)) => {
                        self.state = State::FoundFirstErr(e);
                        None
                    }
                    None => {
                        self.state = State::Exhausted;
                        None
                    }
                },
                State::FoundFirstErr(_) => None,
                State::Exhausted => None,
            }
        }
    }

    impl<I, T, E> FusedIterator for FirstErrIter<I, T, E> where I: Iterator<Item = Result<T, E>> {}

    /// Internal state of [`FirstErrIter`].
    #[derive(Debug)]
    enum State<I, T, E>
    where
        I: Iterator<Item = Result<T, E>>,
    {
        Active(I),
        FoundFirstErr(E),
        Exhausted,
    }
}

mod option {
    use core::iter::FusedIterator;

    /// An `Iterator` can take first `None` from another iterator.
    ///
    /// See [`FirstErr::first_none_or_else()`](crate::FirstErr::first_none_or_else) for more details.
    #[derive(Debug)]
    pub struct FirstNoneIter<I, T>
    where
        I: Iterator<Item = Option<T>>,
    {
        state: State<I, T>,
    }

    impl<I, T> FirstNoneIter<I, T>
    where
        I: Iterator<Item = Option<T>>,
    {
        #[inline]
        pub(super) fn first_none_or_else<O, F>(inner: I, f: F) -> Option<O>
        where
            F: FnOnce(&mut Self) -> O,
        {
            let mut me = Self {
                state: State::Active(inner),
            };

            let output = f(&mut me);

            // Take first None, if not found and not exhausted yet, find it.
            // If just not found finally, return output.
            match me.state {
                State::Active(inner) => {
                    for opt in inner {
                        let _ = opt?;
                    }
                    Some(output)
                }
                State::Exhausted => Some(output),
                State::FoundFirstNone => None,
            }
        }
    }

    impl<I, T> Iterator for FirstNoneIter<I, T>
    where
        I: Iterator<Item = Option<T>>,
    {
        type Item = T;

        #[inline]
        fn next(&mut self) -> Option<Self::Item> {
            match &mut self.state {
                State::Active(inner) => match inner.next() {
                    Some(Some(t)) => Some(t),
                    Some(None) => {
                        self.state = State::FoundFirstNone;
                        None
                    }
                    None => {
                        self.state = State::Exhausted;
                        None
                    }
                },
                State::FoundFirstNone => None,
                State::Exhausted => None,
            }
        }
    }

    impl<I, T> FusedIterator for FirstNoneIter<I, T> where I: Iterator<Item = Option<T>> {}

    /// Internal state of [`FirstNoneIter`].
    #[derive(Debug)]
    enum State<I, T>
    where
        I: Iterator<Item = Option<T>>,
    {
        Active(I),
        FoundFirstNone,
        Exhausted,
    }
}

#[cfg(test)]
mod tests {
    mod test_first_err {
        //! Test first_err_* methods.

        use crate::FirstErr;

        #[test]
        fn _or_else_with_1_layer_data_and_without_err() {
            let ans = [Ok::<u8, u8>(0), Ok(1), Ok(2), Ok(3), Ok(4)]
                .into_iter()
                .first_err_or_else(|iter1| iter1.sum::<u8>());

            assert_eq!(ans, Ok(10));
        }

        #[test]
        fn _or_else_with_1_layer_data_and_with_err() {
            let ans = [Ok::<u8, u8>(0), Ok(1), Err(2), Ok(3), Ok(4)]
                .into_iter()
                .first_err_or_else(|iter1| iter1.sum::<u8>());

            assert_eq!(ans, Err(2));
        }

        // #[test]
        // fn test_first_none_or_else_with_1_layer_data_and_without_none() {
        //     let ans = [Some(0u8), Some(1), Some(2), Some(3), Some(4)]
        //         .into_iter()
        //         .first_none_or_else(|iter1| iter1.sum::<u8>());

        //     assert_eq!(ans, Some(10));
        // }

        // #[test]
        // fn test_first_none_or_else_with_1_layer_data_and_with_none() {
        //     let ans = [Some(0u8), Some(1), None, Some(3), Some(4)]
        //         .into_iter()
        //         .first_none_or_else(|iter1| iter1.sum::<u8>());

        //     assert_eq!(ans, None);
        // }

        #[test]
        fn _or_else_with_2_layer_data_and_outmost_err_in_layer_1() {
            let ans = [
                Ok::<Result<u8, u8>, u8>(Ok(0)),
                Ok(Err(1)),
                Err(2),
                Ok(Ok(3)),
                Ok(Ok(4)),
            ]
            .into_iter()
            .first_err_or_else(|iter1| {
                iter1
                    .map(|x| x) // could chain other ops
                    .first_err_or_else(|iter2| iter2.sum::<u8>())
            });

            assert_eq!(ans, Err(2));
        }

        #[test]
        fn _or_else_with_2_layer_data_and_outmost_err_in_layer_2() {
            let ans = [
                Ok::<Result<u8, u8>, u8>(Ok(0)),
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
        fn _or_else_with_3_layer_data_and_outmost_err_in_layer_2() {
            let ans = [
                Ok::<Result<Result<u8, u8>, u8>, u8>(Ok(Ok(0))),
                Ok(Ok(Ok(1))),
                Ok(Ok(Err(2))),
                Ok(Err(3)),
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

            assert_eq!(ans, Ok(Err(3)));
        }

        #[test]
        fn _or_else_not_need_to_consume_iter_manually() {
            let ans = [Ok::<u8, u8>(0), Err(1), Err(2)]
                .into_iter()
                .first_err_or_else(|_iter| {});

            assert_eq!(ans, Err(1));
        }

        /// In most cases, API users should not be concerned about how many times the original
        /// iterator's `.next()` method is called, as it gets consumed after
        /// `first_err_or_else()` is called.
        ///
        /// However, if the inner iterator has some side-effect, this behavior is still
        /// observable, and users may rely on it.
        ///
        /// This test is designed to ensure that this behavior remains consistent even when
        /// the code changes.
        #[test]
        fn _or_else_never_call_next_on_orig_iter_after_first_err_found() {
            let mut orig_iter_next_count = 0;

            [Ok::<u8, u8>(0), Err(1), Err(2)]
                .into_iter()
                .inspect(|_| orig_iter_next_count += 1) // side-effect
                .first_err_or_else(|iter| {
                    // exhaust whole iter.
                    for _ in &mut *iter {}

                    // call iter.next() after the iter already exhausted.
                    assert_eq!(iter.next(), None);
                })
                .ok();

            assert_eq!(orig_iter_next_count, 2);
        }

        #[test]
        fn _or_else_use_lazy_evaluation() {
            use core::cell::{Cell, RefCell};

            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            enum Trace {
                None,
                Outer(Result<u8, u8>),
                Inner(u8),
            }

            // if index >= N, it will panic.
            fn record_trace<const N: usize>(
                traces: &RefCell<[Trace; N]>,
                idx: &Cell<usize>,
                v: Trace,
            ) {
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

        #[test]
        fn _or_else_with_non_fused_iterator() {
            struct NonFusedIter {
                curr: u32,
            }

            impl NonFusedIter {
                fn new() -> Self {
                    Self { curr: 0 }
                }
            }

            impl Iterator for NonFusedIter {
                type Item = Result<u32, u32>;

                fn next(&mut self) -> Option<Self::Item> {
                    let tmp = self.curr;
                    self.curr += 1;

                    match tmp % 3 {
                        0 => Some(Ok(tmp)),
                        1 => None,
                        2 => Some(Err(tmp)),
                        _ => unreachable!(),
                    }
                }
            }

            let ans = NonFusedIter::new().first_err_or_else(|iter| iter.sum::<u32>());

            assert_eq!(ans, Ok(0));
        }

        #[test]
        fn _or_without_err() {
            let ans = [Ok::<u8, u8>(0), Ok(1), Ok(2), Ok(3), Ok(4)]
                .into_iter()
                .first_err_or("no err");

            assert_eq!(ans, Ok("no err"));
        }

        #[test]
        fn _or_with_err() {
            let ans = [Ok::<u8, u8>(0), Ok(1), Err(2), Ok(3), Ok(4)]
                .into_iter()
                .first_err_or("no err");

            assert_eq!(ans, Err(2));
        }

        #[test]
        fn _or_try_without_err_and_closure_produce_ok() {
            let ans = [Ok::<u8, u8>(0), Ok(1), Ok(2), Ok(3), Ok(4)]
                .into_iter()
                .first_err_or_try(|iter| iter.nth(1).ok_or(1));

            assert_eq!(ans, Ok(1));
        }

        #[test]
        fn _or_try_without_err_and_closure_produce_err() {
            let ans = [Ok::<u8, u8>(0), Ok(1), Ok(2), Ok(3), Ok(4)]
                .into_iter()
                .first_err_or_try(|iter| iter.nth(100).ok_or(100));

            assert_eq!(ans, Err(100));
        }

        #[test]
        fn _or_try_with_err_and_closure_produce_ok() {
            let ans = [Ok::<u8, u8>(0), Ok(1), Err(2), Ok(3), Ok(4)]
                .into_iter()
                .first_err_or_try(|iter| iter.nth(1).ok_or(1));

            assert_eq!(ans, Err(2));
        }

        #[test]
        fn _or_try_with_err_and_closure_produce_err() {
            let ans = [Ok::<u8, u8>(0), Ok(1), Err(2), Ok(3), Ok(4)]
                .into_iter()
                .first_err_or_try(|iter| iter.nth(100).ok_or(100));

            assert_eq!(ans, Err(2));
        }

        #[test]
        fn _methods_can_call_through_trait_object() {
            let mut array_iter = [Ok::<u8, u8>(0), Err(1), Err(2)].into_iter();

            fn take_dyn(iter: &mut dyn Iterator<Item = Result<u8, u8>>) {
                iter.first_err_or_else(|iter| iter.sum::<u8>()).ok();
                iter.first_err_or(0).ok();
                iter.first_err_or_try(|iter| Ok(iter.sum::<u8>())).ok();
            }

            take_dyn(&mut array_iter);
        }
    }

    mod test_first_none {
        //! Test first_none_* methods.

        use crate::FirstErr;

        #[test]
        fn _or_else_with_1_layer_data_and_without_none() {
            let ans = [Some(0u8), Some(1), Some(2), Some(3), Some(4)]
                .into_iter()
                .first_none_or_else(|iter1| iter1.sum::<u8>());

            assert_eq!(ans, Some(10));
        }

        #[test]
        fn _or_else_with_1_layer_data_and_with_none() {
            let ans = [Some(0u8), Some(1), None, Some(3), Some(4)]
                .into_iter()
                .first_none_or_else(|iter1| iter1.sum::<u8>());

            assert_eq!(ans, None);
        }

        #[test]
        fn _or_else_with_2_layer_data_and_outmost_none_in_layer_1() {
            let ans = [
                Some(Some(0u8)),
                Some(None),
                None,
                Some(Some(3)),
                Some(Some(4)),
            ]
            .into_iter()
            .first_none_or_else(|iter1| {
                iter1
                    .map(|x| x) // could chain other ops
                    .first_none_or_else(|iter2| iter2.sum::<u8>())
            });

            assert_eq!(ans, None);
        }

        #[test]
        fn _or_else_with_2_layer_data_and_outmost_none_in_layer_2() {
            let ans = [
                Some(Some(0u8)),
                Some(Some(1)),
                Some(None),
                Some(Some(3)),
                Some(Some(4)),
            ]
            .into_iter()
            .first_none_or_else(|iter1| {
                iter1
                    .map(|x| x) // could chain other ops
                    .first_none_or_else(|iter2| iter2.sum::<u8>())
            });

            assert_eq!(ans, Some(None));
        }

        #[test]
        fn _or_else_with_3_layer_data_and_outmost_none_in_layer_2() {
            let ans = [
                Some(Some(Some(0))),
                Some(Some(Some(1))),
                Some(Some(None)),
                Some(None),
                Some(Some(Some(4))),
            ]
            .into_iter()
            .first_none_or_else(|iter1| {
                iter1
                    .map(|x| x) // could chain other ops
                    .first_none_or_else(|iter2| {
                        iter2
                            .map(|x| x) // could chain other ops
                            .first_none_or_else(|iter3| iter3.sum::<u8>())
                    })
            });

            assert_eq!(ans, Some(None));
        }

        #[test]
        fn _or_else_not_need_to_consume_iter_manually() {
            let ans = [Some(0), None, None]
                .into_iter()
                .first_none_or_else(|_iter| {});

            assert_eq!(ans, None);
        }

        /// In most cases, API users should not be concerned about how many times the original
        /// iterator's `.next()` method is called, as it gets consumed after
        /// `first_none_or_else()` is called.
        ///
        /// However, if the inner iterator has some side-effect, this behavior is still
        /// observable, and users may rely on it.
        ///
        /// This test is designed to ensure that this behavior remains consistent even when
        /// the code changes.
        #[test]
        fn _or_else_never_call_next_on_orig_iter_after_first_none_found() {
            let mut orig_iter_next_count = 0;

            [Some(0), None, None]
                .into_iter()
                .inspect(|_| orig_iter_next_count += 1) // side-effect
                .first_none_or_else(|iter| {
                    // exhaust whole iter.
                    for _ in &mut *iter {}

                    // call iter.next() after the iter already exhausted.
                    assert_eq!(iter.next(), None);
                });

            assert_eq!(orig_iter_next_count, 2);
        }

        #[test]
        fn _or_else_use_lazy_evaluation() {
            use core::cell::{Cell, RefCell};

            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            enum Trace {
                None,
                Outer(Option<u8>),
                Inner(u8),
            }

            // if index >= N, it will panic.
            fn record_trace<const N: usize>(
                traces: &RefCell<[Trace; N]>,
                idx: &Cell<usize>,
                v: Trace,
            ) {
                let i = idx.get();
                traces.borrow_mut()[i] = v;
                idx.set(i + 1);
            }

            // already known N = 5 within [_; N] in this test case.
            // We don't use Vec here just bacause want to avoid `alloc` crate.
            let traces = RefCell::new([Trace::None; 5]);

            let index = Cell::new(0);

            let ans = [Some(0u8), Some(1), None, Some(3)]
                .iter()
                .cloned()
                // record value from outer
                .inspect(|&opt| record_trace(&traces, &index, Trace::Outer(opt)))
                .first_none_or_else(|iter| {
                    iter
                        // record value from inner
                        .inspect(|&n| record_trace(&traces, &index, Trace::Inner(n)))
                        .sum::<u8>()
                });

            assert_eq!(ans, None);
            assert_eq!(
                traces.into_inner(),
                [
                    Trace::Outer(Some(0)),
                    Trace::Inner(0),
                    Trace::Outer(Some(1)),
                    Trace::Inner(1),
                    Trace::Outer(None)
                ]
            );
        }

        #[test]
        fn _or_else_with_non_fused_iterator() {
            struct NonFusedIter {
                curr: u32,
            }

            impl NonFusedIter {
                fn new() -> Self {
                    Self { curr: 0 }
                }
            }

            impl Iterator for NonFusedIter {
                type Item = Option<u32>;

                fn next(&mut self) -> Option<Self::Item> {
                    let tmp = self.curr;
                    self.curr += 1;

                    match tmp % 3 {
                        0 => Some(Some(tmp)),
                        1 => None,       // after produce a None ...
                        2 => Some(None), // it still can produce Some(value)
                        _ => unreachable!(),
                    }
                }
            }

            let ans = NonFusedIter::new().first_none_or_else(|iter| iter.sum::<u32>());

            assert_eq!(ans, Some(0));
        }

        #[test]
        fn _or_without_none() {
            let ans = [Some(0u8), Some(1), Some(2), Some(3), Some(4)]
                .into_iter()
                .first_none_or("no none");

            assert_eq!(ans, Some("no none"));
        }

        #[test]
        fn _or_with_none() {
            let ans = [Some(0u8), Some(1), None, Some(3), Some(4)]
                .into_iter()
                .first_none_or("no none");

            assert_eq!(ans, None);
        }

        #[test]
        fn _or_try_without_none_and_closure_produce_some() {
            let ans = [Some(0u8), Some(1), Some(2), Some(3), Some(4)]
                .into_iter()
                .first_none_or_try(|iter| iter.nth(1));

            assert_eq!(ans, Some(1));
        }

        #[test]
        fn _or_try_without_none_and_closure_produce_none() {
            let ans = [Some(0u8), Some(1), Some(2), Some(3), Some(4)]
                .into_iter()
                .first_none_or_try(|iter| iter.nth(100));

            assert_eq!(ans, None);
        }

        #[test]
        fn _or_try_with_none_and_closure_produce_some() {
            let ans = [Some(0u8), Some(1), None, Some(3), Some(4)]
                .into_iter()
                .first_none_or_try(|iter| iter.nth(1));

            assert_eq!(ans, None);
        }

        #[test]
        fn _or_try_with_none_and_closure_produce_none() {
            let ans = [Some(0u8), Some(1), None, Some(3), Some(4)]
                .into_iter()
                .first_none_or_try(|iter| iter.nth(100));

            assert_eq!(ans, None);
        }

        #[test]
        fn _methods_can_call_through_trait_object() {
            let mut array_iter = [Some(0u8), None, None].into_iter();

            fn take_dyn(iter: &mut dyn Iterator<Item = Option<u8>>) {
                iter.first_none_or_else(|iter| iter.sum::<u8>());
                iter.first_none_or(0);
                iter.first_none_or_try(|iter| Some(iter.sum::<u8>()));
            }

            take_dyn(&mut array_iter);
        }
    }
}
