// Generic code knows nothing of `T::Item` but what its bounds say.
trait Container {
    type Item;

    fn first(&self) -> Self::Item;
}

fn total<C: Container<Item = i64>>(c: &C) -> i64 { c.first() }

fn g<T: Container>(t: &T) -> i64 { total(t) }

fn main() {}
// error: `total` needs `T::Item` to be `i64`, and nothing says it is; say it is with a bound: `T: Container<Item = i64>`
