// An array of references is covariant in their lifetime, not
// contravariant: a `Pair<'a>` isn't a `Pair<'static>`.
#[derive(Copy, Discard)]
struct Pair<'a> { refs: [&'a i64; 2] }

fn lengthen<'a>(p: Pair<'a>) -> Pair<'static> { p }

fn main() {}
// error: lifetime may not live long enough
