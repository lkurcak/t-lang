// Core's `impl Iterator for Filter<I, P>` needs `I::Item: Discard`, which
// isn't a part of `Filter<I, P>`: that's fine as long as proving `Discard`
// can't lead back to `Iterator`. This impl would let it, so proving that a
// type implements `Iterator` could go on forever. It can't be written: an
// impl of `Discard` needs only `Discard`, as a derived one does.
struct W<T> { t: T }

impl<T: Iterator + Discard> Discard for W<T> {
    fn discard(self) {}
}

fn main() {}
// error: an impl of `Discard` may only need its type parameters to be `Discard`, not `core::iter::Iterator` of `T`
