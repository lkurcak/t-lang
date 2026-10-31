#[derive(Discard)]
export struct L {}

impl base::Name for L {
    fn name(&self) -> i64 { 1 }
}

/// Each package's impls meet the other's only in the program, through
/// `base`'s generic code.
export fn wrapped<T: base::Name>(t: T) -> base::Pair<L, T> { base::Pair { a: L {}, b: t } }
