trait Show { fn show(&self); }
struct P {}
impl Show for P {
    fn show<T>(&self) {}
}
fn main() {}
// error: method `show` doesn't match its declaration in trait `Show`: it has 1 type parameter but should have 0
