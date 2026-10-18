#[derive(Discard)]
struct P { x: int }
fn main() { print_bool(P { x: 1 } == P { x: 1 }); }
// error: cannot compare `P` with `==`: it doesn't implement `Eq`; add `#[derive(Eq)]` to it
