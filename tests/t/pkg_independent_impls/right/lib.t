#[derive(Discard)]
export struct R {}

impl base::Name for R {
    fn name(&self) -> i64 with panic { 2 }
}

/// Each package's impls meet the other's only in the program, through
/// `base`'s generic code.
export fn wrapped<T: base::Name>(t: T) -> base::Pair<R, T> { base::Pair { a: R {}, b: t } }
