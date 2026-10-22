// Neither `Ord` nor `i64` is this package's: only `core` can say how ints
// are ordered.
use core::cmp::Ordering;

impl Ord for i64 {
    fn cmp(&self, other: &i64) -> Ordering { Ordering::Equal }
}

fn main() {}
// error: `Ord` and `i64` are both `core`'s, so only `core` can implement one for the other
