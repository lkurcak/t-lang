// What operators are written with.

/// `start..end`: from `start` up to, not including, `end`. It's empty if
/// `start` isn't before `end`. Its fields are private, so only `a..b` and
/// `Range::new` make one.
#[derive(Discard where T: Discard, Eq where T: Eq)]
pub struct Range<T> { start: T, end: T }

impl<T> Range<T> {
    /// `start..end`.
    pub fn new(start: T, end: T) -> Range<T> { Range { start, end } }
}

impl<T: Copy> Range<T> {
    /// Where it starts: where an iterator over it is up to.
    pub fn start(&self) -> T { self.start }

    /// Where it ends, not included.
    pub fn end(&self) -> T { self.end }
}

/// Counts up, and can't panic: it adds one only to what's less than the
/// end, which can't overflow, even at the end of its type, as the checker
/// proves of every integer type (see docs/08-refinement-types.md).
impl<T: Int> Iterator for Range<T> {
    type Item = T;
    effect e = {};

    fn next(&mut self) -> Option<T> {
        if self.start < self.end {
            let n = self.start;
            self.start = self.start + 1;
            Option::Some(n)
        } else {
            Option::None
        }
    }
}

/// Whether to go on, with a `C`, or stop early, with a `B`.
#[derive(Discard where B: Discard, C: Discard, Copy where B: Copy, C: Copy, Eq where B: Eq, C: Eq)]
pub enum ControlFlow<B, C> { Continue(C), Break(B) }

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
pub trait Try {
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
            Option::Some(v) => ControlFlow::Continue(v),
            Option::None => ControlFlow::Break(Option::None),
        }
    }

    fn from_residual(r: Option<!>) -> Option<T> {
        match r {
            Option::None => Option::None,
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
            Result::Ok(v) => ControlFlow::Continue(v),
            Result::Err(e) => ControlFlow::Break(Result::Err(e)),
        }
    }

    fn from_residual(r: Result<!, E>) -> Result<T, E> {
        match r {
            Result::Err(e) => Result::Err(e),
        }
    }
}
