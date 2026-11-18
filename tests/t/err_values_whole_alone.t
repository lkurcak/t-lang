// A whole value that isn't an integer or a `bool` is only compared,
// with `==` or `!=` (docs/85-values-in-predicates.md).
#[derive(Copy, Discard)]
struct Pair {
    lo: u8,
    hi: u8,
}

fn f(a: &mut Pair) ensures *a {}

fn main() {}
// error: a refinement can only name `*a`, of type `Pair`, by its fields and measures: `(*a).f`, or `(*a).m()`, where `m` is a `#[measure]`; or whole, with `==` or `!=`
