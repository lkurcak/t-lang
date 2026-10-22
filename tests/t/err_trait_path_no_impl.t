trait Show { fn show(&self); }
fn main() { Show::show(&1i64); }
// error: `i64` doesn't implement `Show`, which method `show` is of
