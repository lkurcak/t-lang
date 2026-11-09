#[derive(Discard, Hash)]
struct P { x: i64 }

fn main() {}
// error: can't derive `Hash`; only `Copy`, `Debug`, `Discard`, `Eq`, `Ord`, `Send` and `Sync` can be derived
