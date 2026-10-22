trait to_string { fn ToStr(&self) -> i64; }
fn main() {}
// warning: trait `to_string` should have an upper camel case name: `ToString`
// warning: method `ToStr` should have a snake case name: `to_str`
