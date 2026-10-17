// Bounds don't keep impls apart: `int` might be `Show` one day, and then an
// impl somewhere else would change meaning.
trait Show { fn show(&self); }
trait Print { fn print(&self); }
impl<T: Show> Print for T { fn print(&self) {} }
impl Print for int { fn print(&self) {} }
fn main() {}
// error: this impl of `Print` overlaps `impl Print for T`
