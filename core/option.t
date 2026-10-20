// Optional values.

/// A value that may be missing: `Some(x)`, or `None`.
#[derive(Discard, Copy, Eq)]
pub enum Option<T> { None, Some(T) }

impl<T> Option<T> {
    /// Is it `Some`?
    pub fn is_some(&self) -> bool {
        match self {
            Option::Some(_) => true,
            Option::None => false,
        }
    }

    /// Is it `None`?
    pub fn is_none(&self) -> bool { !self.is_some() }

    /// A reference to what's in it, if anything.
    pub fn as_ref(&self) -> Option<&T> {
        match self {
            Option::Some(x) => Option::Some(x),
            Option::None => Option::None,
        }
    }

    /// A mutable reference to what's in it, if anything.
    pub fn as_mut(&mut self) -> Option<&mut T> {
        match self {
            Option::Some(x) => Option::Some(x),
            Option::None => Option::None,
        }
    }

    /// Takes what's in it out, leaving `None`.
    pub fn take(&mut self) -> Option<T> { crate::mem::replace(self, Option::None) }

    /// `f` of what's in it, if anything. On `None`, `f` is thrown away
    /// uncalled, so it must be `Discard`.
    pub fn map<U, F: FnOnce(T) -> U + Discard>(self, f: F) -> Option<U> {
        match self {
            Option::Some(x) => Option::Some(f(x)),
            Option::None => Option::None,
        }
    }

    /// `f` of what's in it, if anything, which may itself be `None`. On
    /// `None`, `f` is thrown away uncalled.
    pub fn and_then<U, F: FnOnce(T) -> Option<U> + Discard>(self, f: F) -> Option<U> {
        match self {
            Option::Some(x) => f(x),
            Option::None => Option::None,
        }
    }

    /// What's in it, or else what `f` makes. Nothing is thrown away but
    /// `f`, uncalled, so `T` needn't be `Discard`.
    pub fn unwrap_or_else<F: FnOnce() -> T + Discard>(self, f: F) -> T {
        match self {
            Option::Some(x) => x,
            Option::None => f(),
        }
    }

    /// What's in it, or else what `f` gives, which may be `None` too. On
    /// `Some`, `f` is thrown away uncalled.
    pub fn or_else<F: FnOnce() -> Option<T> + Discard>(self, f: F) -> Option<T> {
        match self {
            Option::Some(x) => Option::Some(x),
            Option::None => f(),
        }
    }
}

impl<T: Discard> Option<T> {
    /// What's in it, or else `default`. One of the two is thrown away.
    pub fn unwrap_or(self, default: T) -> T {
        match self {
            Option::Some(x) => x,
            Option::None => default,
        }
    }
}
