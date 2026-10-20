// Results of what can fail.

use crate::option::Option;

/// What worked, `Ok(x)`, or why it didn't, `Err(e)`.
#[derive(Discard, Copy, Eq)]
pub enum Result<T, E> { Ok(T), Err(E) }

impl<T, E> Result<T, E> {
    /// Is it `Ok`?
    pub fn is_ok(&self) -> bool {
        match self {
            Result::Ok(_) => true,
            Result::Err(_) => false,
        }
    }

    /// Is it `Err`?
    pub fn is_err(&self) -> bool { !self.is_ok() }

    /// References to what's in it.
    pub fn as_ref(&self) -> Result<&T, &E> {
        match self {
            Result::Ok(x) => Result::Ok(x),
            Result::Err(e) => Result::Err(e),
        }
    }

    /// `f` of what's in an `Ok`.
    pub fn map<U, F: FnOnce(T) -> U + Discard>(self, f: F) -> Result<U, E> {
        match self {
            Result::Ok(x) => Result::Ok(f(x)),
            Result::Err(e) => Result::Err(e),
        }
    }

    /// `f` of what's in an `Ok`, which may itself fail.
    pub fn and_then<U, F: FnOnce(T) -> Result<U, E> + Discard>(self, f: F) -> Result<U, E> {
        match self {
            Result::Ok(x) => f(x),
            Result::Err(e) => Result::Err(e),
        }
    }

    /// `f` of what's in an `Err`.
    pub fn map_err<G, F: FnOnce(E) -> G + Discard>(self, f: F) -> Result<T, G> {
        match self {
            Result::Ok(x) => Result::Ok(x),
            Result::Err(e) => Result::Err(f(e)),
        }
    }

    /// What's in an `Ok`, or else `f` of the error. The error goes to `f`
    /// rather than being thrown away, so neither `T` nor `E` need be
    /// `Discard`.
    pub fn unwrap_or_else<F: FnOnce(E) -> T + Discard>(self, f: F) -> T {
        match self {
            Result::Ok(x) => x,
            Result::Err(e) => f(e),
        }
    }

    /// `f` of what's in an `Err`, which may recover from it, or fail with
    /// an error of another type. An `Ok` is kept, and `f` thrown away.
    pub fn or_else<G, F: FnOnce(E) -> Result<T, G> + Discard>(self, f: F) -> Result<T, G> {
        match self {
            Result::Ok(x) => Result::Ok(x),
            Result::Err(e) => f(e),
        }
    }
}

impl<T, E: Discard> Result<T, E> {
    /// What's in an `Ok`, throwing away an `Err`.
    pub fn ok(self) -> Option<T> {
        match self {
            Result::Ok(x) => Option::Some(x),
            Result::Err(_e) => Option::None,
        }
    }
}

impl<T: Discard, E> Result<T, E> {
    /// What's in an `Err`, throwing away an `Ok`.
    pub fn err(self) -> Option<E> {
        match self {
            Result::Ok(_x) => Option::None,
            Result::Err(e) => Option::Some(e),
        }
    }
}

impl<T: Discard, E: Discard> Result<T, E> {
    /// What's in an `Ok`, or else `default`.
    pub fn unwrap_or(self, default: T) -> T {
        match self {
            Result::Ok(x) => x,
            Result::Err(_e) => default,
        }
    }
}
