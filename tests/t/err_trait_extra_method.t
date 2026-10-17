trait Show { fn show(&self); }
struct P {}
impl Show for P {
    fn show(&self) {}
    fn hide(&self) {}
}
fn main() {}
// error: `hide` isn't a method of trait `Show`
