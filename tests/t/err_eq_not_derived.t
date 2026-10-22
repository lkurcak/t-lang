#[derive(Discard)]
struct P { x: i64 }
fn main() { print_bool(P { x: 1 } == P { x: 1 }); }
// error: cannot compare `P` with `==`: it doesn't implement `Eq`; add `#[derive(Eq)]` to it
