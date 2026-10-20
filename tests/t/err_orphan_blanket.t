// A blanket impl of a core trait would cover core's types too: its head is
// a type parameter, which is no package's.
impl<T: Eq> Ord for T {
    fn cmp(&self, other: &T) -> core::cmp::Ordering { core::cmp::Ordering::Equal }
}

fn main() {}
// error: `Ord` and `T` are both `core`'s, so only `core` can implement one for the other
