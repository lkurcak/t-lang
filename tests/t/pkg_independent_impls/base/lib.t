export trait Name {
    fn name(&self) -> i64 with panic;
}

/// Twice the name of `x`, through a generic call that
/// dispatches to whichever impl its caller's type has.
export fn twice<T: Name>(x: &T) -> i64 with panic { x.name() + x.name() }

#[derive(Discard)]
export struct Pair<A, B> { export a: A, export b: B }

impl<A: Name, B: Name> Name for Pair<A, B> {
    fn name(&self) -> i64 with panic { self.a.name() * 10 + self.b.name() }
}
