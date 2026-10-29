//! Results of what can fail.

use crate::option::Option;

/// What worked, `Ok(x)`, or why it didn't, `Err(e)`.
#[derive(Discard, Copy, Eq)]
export enum Result<T, E> { Ok(T), Err(E) }

impl<T, E> Result<T, E> {
    /// Is it `Ok`?
    export fn is_ok(&self) -> bool {
        match self {
            Result::Ok(_) => true,
            Result::Err(_) => false,
        }
    }

    /// Is it `Err`?
    export fn is_err(&self) -> bool { !self.is_ok() }

    /// References to what's in it.
    export fn as_ref(&self) -> Result<&T, &E> {
        match self {
            Result::Ok(x) => Result::Ok(x),
            Result::Err(e) => Result::Err(e),
        }
    }

    /// `f` of what's in an `Ok`.
    export fn map<U, effect e, F: FnOnce(T) -> U with e + Discard>(self, f: F) -> Result<U, E>
    with e
    {
        match self {
            Result::Ok(x) => Result::Ok(f(x)),
            Result::Err(e) => Result::Err(e),
        }
    }

    /// `f` of what's in an `Ok`, which may itself fail.
    export fn and_then<U, effect e, F: FnOnce(T) -> Result<U, E> with e + Discard>(
        self,
        f: F,
    ) -> Result<U, E> with e {
        match self {
            Result::Ok(x) => f(x),
            Result::Err(e) => Result::Err(e),
        }
    }

    /// `f` of what's in an `Err`.
    export fn map_err<G, effect e, F: FnOnce(E) -> G with e + Discard>(self, f: F) -> Result<T, G>
    with e
    {
        match self {
            Result::Ok(x) => Result::Ok(x),
            Result::Err(e) => Result::Err(f(e)),
        }
    }

    /// What's in an `Ok`, or else `f` of the error. The error goes to `f`
    /// rather than being thrown away, so neither `T` nor `E` need be
    /// `Discard`.
    export fn unwrap_or_else<effect e, F: FnOnce(E) -> T with e + Discard>(self, f: F) -> T with e {
        match self {
            Result::Ok(x) => x,
            Result::Err(e) => f(e),
        }
    }

    /// `f` of what's in an `Err`, which may recover from it, or fail with
    /// an error of another type. An `Ok` is kept, and `f` thrown away.
    export fn or_else<G, effect e, F: FnOnce(E) -> Result<T, G> with e + Discard>(
        self,
        f: F,
    ) -> Result<T, G> with e {
        match self {
            Result::Ok(x) => Result::Ok(x),
            Result::Err(e) => f(e),
        }
    }
}

impl<T, E: Discard> Result<T, E> {
    /// What's in an `Ok`, throwing away an `Err`.
    export fn ok(self) -> Option<T> {
        match self {
            Result::Ok(x) => Option::Some(x),
            Result::Err(_e) => Option::None,
        }
    }
}

impl<T: Discard, E> Result<T, E> {
    /// What's in an `Err`, throwing away an `Ok`.
    export fn err(self) -> Option<E> {
        match self {
            Result::Ok(_x) => Option::None,
            Result::Err(e) => Option::Some(e),
        }
    }
}

impl<T: Discard, E: Discard> Result<T, E> {
    /// What's in an `Ok`, or else `default`.
    export fn unwrap_or(self, default: T) -> T {
        match self {
            Result::Ok(x) => x,
            Result::Err(_e) => default,
        }
    }
}

impl<T, E: Discard> Result<T, E> {
    /// What's in an `Ok`, or else a panic. The error is thrown away.
    export fn unwrap(self) -> T with panic { self.expect("`unwrap` of `Err`") }

    /// What's in an `Ok`, or else a panic, saying `why`. The error is
    /// thrown away.
    export fn expect(self, why: &str) -> T with panic {
        match self {
            Result::Ok(x) => x,
            Result::Err(_) => crate::panic::panic(why),
        }
    }
}

impl<T> Result<T, !> {
    /// What's in it, which is an `Ok`, as nothing is a `!`: so, unlike
    /// `unwrap`, it can't panic. What a sink that can't fail gives, such as
    /// a `String`, is one (see `core::fmt`).
    export fn into_ok(self) -> T {
        match self {
            Result::Ok(x) => x,
        }
    }
}
