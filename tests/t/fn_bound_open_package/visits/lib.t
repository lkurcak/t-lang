//! A library whose exported items leave what their closures do open: an
//! exported function says `F::e`, a trait's method's bound is open for
//! every impl, in any package, and an impl's effect is its closure's.

/// Gives `f` each of `0..n`, in order.
export fn each<F: FnMut(u8) + Discard>(n: u8, mut f: F) with F::e {
    let mut i = 0;
    while i < n {
        f(i);
        i = i + 1;
    }
}

/// What can give a closure its values.
export trait Visit {
    fn visit<F: FnMut(u8) + Discard>(&self, f: F) with F::e;
}

/// The values a closure makes, until it makes `None`.
#[derive(Discard where F: Discard)]
export struct FromFn<F> { f: F }

impl<F> FromFn<F> {
    /// The values `f` makes.
    export fn new(f: F) -> FromFn<F> { FromFn { f } }
}

impl<T, F: FnMut() -> Option<T>> Iterator for FromFn<F> {
    type Item = T;
    effect e = F::e;

    fn next(&mut self) -> Option<T> with F::e { (self.f)() }
}
