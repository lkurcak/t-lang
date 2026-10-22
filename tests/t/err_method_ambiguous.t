trait A { fn m(&self); }
trait B { fn m(&self); }
impl A for i64 { fn m(&self) {} }
impl B for i64 { fn m(&self) {} }
fn main() { 1i64.m(); }
// error: method `m` of this value is ambiguous: traits `A` and `B` both have one; say which, as in `A::m(..)`
