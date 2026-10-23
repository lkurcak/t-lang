// Core's `impl Iterator for Filter<I, P>` needs `I::Item: Discard`, which
// isn't a part of `Filter<I, P>`: that's fine as long as proving `Discard`
// can't lead back to `Iterator`, and in core alone it can't. This impl
// would let it, so proving that a type implements `Iterator` could then go
// on forever. Core checked on its own, so it's this impl that's at fault,
// and the error is here, not in core.
struct W<T> { t: T }

impl<T: Iterator + Discard> Discard for W<T> {}

fn main() {}
// error: this impl lets proving that lead back to `core::iter::Iterator`
