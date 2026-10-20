// Neither `Ord` nor `int` is this package's: only `core` can say how ints
// are ordered.
use core::cmp::Ordering;

impl Ord for int {
    fn cmp(&self, other: &int) -> Ordering { Ordering::Equal }
}

fn main() {}
// error: `Ord` and `int` are both `core`'s, so only `core` can implement one for the other
