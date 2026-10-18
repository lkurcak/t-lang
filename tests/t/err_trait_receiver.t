trait Show { fn show(&self); }
#[derive(Discard)]
struct P {}
impl Show for P {
    fn show(self) {}
}
fn main() {}
// error: method `show` doesn't match its declaration in trait `Show`: its receiver should be `&P`, not `P`
