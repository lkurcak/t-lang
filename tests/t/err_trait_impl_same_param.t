// `T` could be `&i64` at two different lifetimes: found for a
// `Pair<&'x i64, &'y i64>`, `leak` would swap them.
#[derive(Discard)]
struct Pair<A, B> { long: A, short: B }

trait Leak { fn leak(self) -> Self; }

impl<T> Leak for Pair<T, T> {
    fn leak(self) -> Pair<T, T> { Pair { long: self.short, short: self.long } }
}

fn main() {}
// error: `Pair<T, T>` says two of its parts are the same type `T`, which could have lifetimes in it that differ, but an impl of a trait must hold at every lifetime, as finding the impl for a type doesn't look at lifetimes
