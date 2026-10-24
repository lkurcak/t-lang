// Core's `impl Iterator for Filter<I, P>` needs `I::Item: Discard`, which
// isn't a part of `Filter<I, P>`: that's fine as long as proving `Discard`
// can't lead back to `Iterator`. This impl would let it, so proving that a
// type implements `Iterator` could go on forever. It can't be written:
// `Discard` is only derived, and a derived `Discard` needs only `Discard`.
struct W<T> { t: T }

impl<T: Iterator + Discard> Discard for W<T> {}

fn main() {}
// error: `Discard` can only be derived
