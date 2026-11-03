#[derive(Discard, Debug)]
struct P { x: i64 }

fn main() {}
// error: can't derive `Debug`; only `Copy`, `Discard`, `Eq`, `Send` and `Sync` can be derived
