#[derive(Discard, Debug)]
struct P { x: int }

fn main() {}
// error: can't derive `Debug`; only `Copy`, `Discard` and `Eq` can be derived
