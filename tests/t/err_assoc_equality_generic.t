// Generic code knows nothing of `T::Item` but what its bounds say.
trait Container {
    type Item;

    fn first(&self) -> Self::Item;
}

fn total<C: Container<Item = int>>(c: &C) -> int { c.first() }

fn g<T: Container>(t: &T) -> int { total(t) }

fn main() {}
// error: `total` needs `T::Item` to be `int`, and nothing says it is; say it is with a bound: `T: Container<Item = int>`
