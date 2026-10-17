// No bound is implied: an impl of a struct must say what the struct needs.
trait Ord: Eq {}
struct Sorted<T: Ord> { lo: T }
impl<T> Sorted<T> {
    fn lo(&self) -> &T { &self.lo }
}
fn main() {}
// error: `Sorted` needs its `T` to implement `Ord`, and `T` doesn't; add a bound: `T: Ord`
