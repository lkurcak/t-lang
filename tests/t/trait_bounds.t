// Bounds on a struct's type parameters: every use of the struct must show
// they hold, its impls included, as nothing implies them.

trait Ord: Eq {
    fn lt(&self, other: &Self) -> bool;
}

impl Ord for int {
    fn lt(&self, other: &int) -> bool { *self < *other }
}

#[derive(Discard)]
struct Sorted<T: Ord> { lo: T, hi: T }

impl<T: Ord> Sorted<T> {
    fn of(a: T, b: T) -> Sorted<T> {
        if b.lt(&a) { Sorted { lo: b, hi: a } } else { Sorted { lo: a, hi: b } }
    }
}

fn width(s: Sorted<int>) -> int { s.hi - s.lo }

fn main() {
    print(width(Sorted::of(9, 3)));  // expect: 6
}
