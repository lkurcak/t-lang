trait Show { fn show(&self); }
fn main() { Show::show(&1); }
// error: `int` doesn't implement `Show`, which method `show` is of
