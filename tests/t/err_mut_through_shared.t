#[derive(Discard)]
struct P { x: i64 }

fn set(p: &P) { p.x = 1; }

fn main() {}
// error: cannot assign to `p.x`: it's behind a `&` reference
