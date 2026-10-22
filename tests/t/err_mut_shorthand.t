#[derive(Discard)]
struct P { x: i64 }

fn main() {
    let P { mut x } = P { x: 1 };
}
// error: write `x: mut x` to bind field `x` mutably
