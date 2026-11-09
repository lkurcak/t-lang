// Through a derived `Ord`, which compares field by field: `D<&V<T>>`'s
// `cmp` compares a `V<&V<T>>`, whose `cmp` compares a `D<&V<&V<T>>>`, and
// so on, a bigger type each time, so monomorphising would never end. The
// graph of what's copied has a node for each derived `cmp`, as it has for
// each derived `eq` and `fmt`.
use core::cmp::Ordering;

#[derive(Discard, Eq, Ord)]
struct D<U> { v: V<U> }

#[derive(Discard)]
struct V<T> { x: T }

impl<T: Eq> Eq for V<T> {
    fn eq(&self, other: &V<T>) -> bool { self.x == other.x }
}

impl<T: Ord> Ord for V<T> {
    fn cmp(&self, other: &V<T>) -> Ordering {
        let a = D { v: V { x: self } };
        let b = D { v: V { x: other } };
        a.cmp(&b)
    }
}

fn main() {
    let _ = V { x: 1u8 }.cmp(&V { x: 2u8 });
}
// error: polymorphic recursion: this use of `<D<U> as Ord>::cmp` with `U` = `&V<T>` leads back
