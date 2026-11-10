//! What operators are written with.

use crate::fmt::Debug;

/// What can be called, taking it: `F: FnOnce(i64) -> i64` says a value
/// of type `F` can be called once with an `i64`, giving an `i64`.
/// Functions and closures have it, and nothing else can (see
/// docs/06c-closures.md).
#[intrinsic]
export trait FnOnce {}

/// What can be called, borrowing it mutably, so as often as wanted.
#[intrinsic]
export trait FnMut: FnOnce {}

/// What can be called, borrowing it, so as often as wanted, and from
/// anywhere it's shared.
#[intrinsic]
export trait Fn: FnMut {}

/// `start..end`: from `start` up to, not including, `end`. It's empty if
/// `start` isn't before `end`. Its fields are private, so only `a..b` and
/// `Range::new` make one.
#[derive(
    Discard where T: Discard,
    Debug where T: Debug,
    Eq where T: Eq,
    Send where T: Send,
    Sync where T: Sync,
)]
export struct Range<T> { start: T, end: T }

impl<T: Int> Range<T> {
    /// `start..end`.
    export fn new(start: T, end: T) -> { r: Range<T> | r.start() == start && r.end() == end } {
        Range { start, end }
    }

    /// Where it starts: where an iterator over it is up to.
    #[measure]
    export fn start(&self) -> T { self.start }

    /// Where it ends, not included.
    #[measure]
    export fn end(&self) -> T { self.end }
}

/// Counts up, and can't panic: it adds one only to what's less than the
/// end, which can't overflow, even at the end of its type, as the checker
/// proves of every integer type (see docs/08-refinement-types.md). What it
/// gives is less than the end, which it keeps: so `for x in a..b` knows
/// `x < b` (see docs/53-refined-type-arguments.md).
impl<T: Int> Iterator for Range<T> {
    type Item = T;
    effect e = {};

    fn next(&mut self) -> Option<{ v: T | v < self.end() }> ensures self.end() == old(self.end()) {
        if self.start < self.end {
            let n = self.start;
            self.start = self.start + 1;
            Some(n)
        } else {
            None
        }
    }
}

/// Counts down from the end, as `next` counts up from the start: what it
/// gives is at least the start, which it keeps, and less than the end it
/// had. Neither can panic: it takes one only from what's more than the
/// start.
impl<T: Int> DoubleEndedIterator for Range<T> {
    fn next_back(&mut self) -> Option<{ v: T | v >= self.start() }>
    ensures
        self.start() == old(self.start()),
    {
        if self.start < self.end {
            self.end = self.end - 1;
            Some(self.end)
        } else {
            None
        }
    }
}

/// Whether to go on, with a `C`, or stop early, with a `B`.
#[derive(
    Discard where B: Discard, C: Discard,
    Copy where B: Copy, C: Copy,
    Debug where B: Debug, C: Debug,
    Eq where B: Eq, C: Eq,
    Send where B: Send, C: Send,
    Sync where B: Sync, C: Sync,
)]
export enum ControlFlow<B, C> { Continue(C), Break(B) }

/// What `?` works on: a value that either goes on, as an `Output`, or
/// stops the function early, as a `Residual`, from which a value of the
/// function's result type is made. `e?` means
///
/// ```text
/// match Try::branch(e) {
///     ControlFlow::Continue(v) => v,
///     ControlFlow::Break(r) => return Try::from_residual(r),
/// }
/// ```
///
/// so `?` returns early only into a type with the same `Residual`. Neither
/// method has effects, so neither has a `?`.
export trait Try {
    type Output;
    type Residual;

    /// Go on with what's in it, or stop with the rest.
    fn branch(self) -> ControlFlow<Self::Residual, Self::Output>;

    /// What a function returns when a `?` in it stops early.
    fn from_residual(r: Self::Residual) -> Self;
}

/// An `Option`'s `?` gives what's in a `Some`, or returns `None`. What goes
/// up is an `Option<!>`, which can only be a `None`, so it goes into an
/// `Option` of any type, and into nothing else.
impl<T> Try for Option<T> {
    type Output = T;
    type Residual = Option<!>;

    fn branch(self) -> ControlFlow<Option<!>, T> {
        match self {
            Some(v) => ControlFlow::Continue(v),
            None => ControlFlow::Break(None),
        }
    }

    fn from_residual(r: Option<!>) -> Option<T> {
        match r {
            None => None,
        }
    }
}

/// A `Result`'s `?` gives what's in an `Ok`, or returns the `Err`. What goes
/// up is a `Result<!, E>`, which can only be an `Err`, so it goes into a
/// `Result` with the same error type, and into nothing else: an error isn't
/// converted.
impl<T, E> Try for Result<T, E> {
    type Output = T;
    type Residual = Result<!, E>;

    fn branch(self) -> ControlFlow<Result<!, E>, T> {
        match self {
            Ok(v) => ControlFlow::Continue(v),
            Err(e) => ControlFlow::Break(Err(e)),
        }
    }

    fn from_residual(r: Result<!, E>) -> Result<T, E> {
        match r {
            Err(e) => Err(e),
        }
    }
}
