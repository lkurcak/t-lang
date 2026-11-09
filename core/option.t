//! Optional values.

/// A value that may be missing: `Some(x)`, or `None`.
#[derive(Discard, Copy, Debug, Eq, Send, Sync)]
export enum Option<T> { None, Some(T) }

impl<T> Option<T> {
    /// Is it `Some`? A measure: a refinement can say which it is.
    #[measure]
    export fn is_some(&self) -> bool {
        match self {
            Some(_) => true,
            None => false,
        }
    }

    /// Is it `None`?
    #[measure]
    export fn is_none(&self) -> bool { !self.is_some() }

    /// A reference to what's in it, if anything.
    export fn as_ref(&self) -> Option<&T> {
        match self {
            Some(x) => Some(x),
            None => None,
        }
    }

    /// A mutable reference to what's in it, if anything.
    export fn as_mut(&mut self) -> Option<&mut T> {
        match self {
            Some(x) => Some(x),
            None => None,
        }
    }

    /// What's in it, or else a panic.
    #[blame_caller]
    export fn unwrap(self) -> T with panic { self.expect("`unwrap` of `None`") }

    /// What's in it, or else a panic, saying `why`.
    #[blame_caller]
    export fn expect(self, why: &str) -> T with panic {
        match self {
            Some(x) => x,
            None => crate::panic::panic(why),
        }
    }

    /// Takes what's in it out, leaving `None`.
    export fn take(&mut self) -> Option<T> { crate::mem::replace(self, None) }

    /// What's in the `Some`, as an `Ok`, or else `err`, as the `Err`: so
    /// `?` can give an error where there's nothing.
    export fn ok_or<E: Discard>(self, err: E) -> Result<T, E> {
        match self {
            Some(x) => Ok(x),
            None => Err(err),
        }
    }

    /// `f` of what's in it, if anything. On `None`, `f` is thrown away
    /// uncalled, so it must be `Discard`.
    export fn map<U, effect e, F: FnOnce(T) -> U with e + Discard>(self, f: F) -> Option<U> with e {
        match self {
            Some(x) => Some(f(x)),
            None => None,
        }
    }

    /// `f` of what's in it, if anything, which may itself be `None`. On
    /// `None`, `f` is thrown away uncalled.
    export fn and_then<U, effect e, F: FnOnce(T) -> Option<U> with e + Discard>(
        self,
        f: F,
    ) -> Option<U> with e {
        match self {
            Some(x) => f(x),
            None => None,
        }
    }

    /// What's in it, or else what `f` makes. Nothing is thrown away but
    /// `f`, uncalled, so `T` needn't be `Discard`.
    export fn unwrap_or_else<effect e, F: FnOnce() -> T with e + Discard>(self, f: F) -> T with e {
        match self {
            Some(x) => x,
            None => f(),
        }
    }

    /// What's in it, or else what `f` gives, which may be `None` too. On
    /// `Some`, `f` is thrown away uncalled.
    export fn or_else<effect e, F: FnOnce() -> Option<T> with e + Discard>(self, f: F) -> Option<T>
    with e
    {
        match self {
            Some(x) => Some(x),
            None => f(),
        }
    }
}

impl<T: Discard> Option<T> {
    /// What's in it, or else `default`. One of the two is thrown away.
    export fn unwrap_or(self, default: T) -> T {
        match self {
            Some(x) => x,
            None => default,
        }
    }
}

impl<'a, T: Copy> Option<&'a T> {
    /// What it points to, copied, if anything: `Option<&T>` to
    /// `Option<T>`.
    export fn copied(self) -> Option<T> {
        match self {
            Some(x) => Some(*x),
            None => None,
        }
    }
}
