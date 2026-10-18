// An impl's method has the trait's signature, at its type: callers go by that.
trait Show { fn show(&self) -> int; }
#[derive(Discard)]
struct P {}
impl Show for P {
    fn show(&self) -> bool { true }
}
fn main() {}
// error: method `show` doesn't match its declaration in trait `Show`: it should return `int`, not `bool`
