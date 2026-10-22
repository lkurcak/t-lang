// Only `core` gives `[T]` methods of its own.
impl<T> [T] {
    fn second(&self) -> Option<&T> { self.get(1) }
}

fn main() {}
// error: an inherent impl can only be for a struct or enum, not `[T]`; to give `[T]` methods, declare a trait and implement that
