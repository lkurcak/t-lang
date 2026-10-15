fn take(r: &Box<int>) -> Box<int> { *r }

fn main() {}
// error: cannot move out of `*r`, which is behind a reference
