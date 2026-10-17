// An impl's method can't need more than the trait's says it does.
trait Pick { fn pick<T>(&self, a: T, b: T) -> T; }
struct First {}
impl Pick for First {
    fn pick<T: Copy>(&self, a: T, b: T) -> T { a }
}
fn main() {}
// error: method `pick` doesn't match its declaration in trait `Pick`: its bounds should be none
