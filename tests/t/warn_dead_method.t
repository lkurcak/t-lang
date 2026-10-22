#[derive(Discard)]
struct P { x: i64 }
impl P {
    fn used(&self) -> i64 { self.x }
    fn unused(&self) -> i64 { 0 }
}
// Methods of impls of traits are never reported: which one runs depends on
// types.
trait Show { fn show(&self); }
impl Show for P { fn show(&self) {} }
fn main() { print(P { x: 1 }.used()); } // expect: 1
// warning: method `unused` is never used
