// A tuple's `Ord` is its values': proving `(N<(&T, u8)>, u8): Ord` in
// `N<T>`'s `cmp` uses `N`'s impl again at a bigger type, with no end to
// the copies monomorphisation would make. The proof goes through the
// tuple, so must the check.
use core::cmp::Ordering;

#[derive(Discard)]
struct N<T> { v: T }

impl<T: Eq> Eq for N<T> {
    fn eq(&self, other: &N<T>) -> bool { self.v == other.v }
}

impl<T: Ord> Ord for N<T> {
    fn cmp(&self, other: &N<T>) -> Ordering {
        let a = (N { v: (&self.v, 0u8) }, 1u8);
        let b = (N { v: (&other.v, 0u8) }, 1u8);
        a.cmp(&b)
    }
}

fn main() {
    let _ = N { v: 1u8 }.cmp(&N { v: 2u8 });
}
// error: polymorphic recursion
