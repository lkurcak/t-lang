// Bounds don't keep impls apart where another package could make them
// hold: core might make `i64` an iterator one day, and then an impl here
// would change meaning.
trait Print { fn print(&self); }
impl<T: Iterator> Print for T { fn print(&self) {} }
impl Print for i64 { fn print(&self) {} }
fn main() {}
// error: this impl of `Print` overlaps `impl Print for T`
