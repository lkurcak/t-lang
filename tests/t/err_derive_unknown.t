#[derive(Discard, Debug)]
struct P { x: i64 }

fn main() {}
// error: can't derive `Debug`; only `Copy`, `Discard` and `Eq` can be derived
